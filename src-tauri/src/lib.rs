use chrono::Local;
use log::info;
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use rusqlite::{params, Connection};
use std::{
    process::{Child, Command},
    str::FromStr,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_log::{Target, TargetKind};
use cron::Schedule as CronSchedule;
use tauri::async_runtime::JoinHandle as AsyncJoinHandle;
use tokio::sync::Mutex;
use uuid::Uuid;
mod wps_reader;

struct ChildProcess(Mutex<Option<Child>>);

struct Db(Mutex<Connection>);

struct Schedule(Mutex<Option<AsyncJoinHandle<()>>>);

fn normalize_cron(expr:&str) -> String{
    if expr.split_whitespace().count() == 5{
        format!("0 {}",expr)
    }else{
        expr.to_string()
    }
}

#[tauri::command]
async fn start_cron(
    app:tauri::AppHandle,
    schedule:tauri::State<'_,Schedule>,
    cron:String,
) -> Result<(),String>{
    info!("设置定时任务: {}",cron);
    if let Some(h) = schedule.0.lock().await.take(){
        h.abort();
    }
    let expr = normalize_cron(&cron);
    let cron_schedule = CronSchedule::from_str(&expr).map_err(|e|format!("解析cron表达式失败:{}",e))?;

    let handle = tauri::async_runtime::spawn({
        let app = app.clone();
        async move{
            let mut upcoming = cron_schedule.upcoming(Local);
            loop {
                let Some(next_dt) = upcoming.next() else {break};
                let now = Local::now();
                let dur = (next_dt -now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(0));
                tokio::time::sleep(dur).await;

                let db_state:tauri::State<Db> = app.state();
                let _ = execute_task(db_state).await;
            }
        }
    });
    *schedule.0.lock().await = Some(handle);
    Ok(())
}

#[tauri::command]
async fn stop_cron(scheduler:tauri::State<'_,Schedule>) -> Result<(),String>{
    info!("停止定时任务");
    if let Some(h) = scheduler.0.lock().await.take(){
        h.abort();
    }
    Ok(())
}

async fn stop_background_service(app: tauri::AppHandle) {
    let child_opt = {
        let state: tauri::State<ChildProcess> = app.state();
        let mut guard = state.0.lock().await;
        guard.take()
    }; // 这里 guard 被提前释放

    if let Some(mut child) = child_opt {
        let _ = child.kill();
        let _ = child.wait(); // 可选：等待子进程退出更干净
        println!("服务已停止");
    }
}

#[tauri::command]
async fn get_task_list(
    sku: Option<String>,
    module: Option<String>,
    db: tauri::State<'_, Db>,
) -> Result<String, String> {
    let conn = db.0.lock().await;
    let mut sql = String::from(
        r#"
         SELECT
          task_id,
          replace(substr(run_time,1,19),'T',' ') AS run_time,
          SKU,
          module,
          status
        FROM tasks
        WHERE 1=1       
        "#,
    );
    let mut binds: Vec<String> = vec![];
    if let Some(s) = sku.as_ref() {
        sql.push_str(" AND SKU like ?");
        binds.push(format!("%{}%", s));
    }
    if let Some(m) = module.as_ref() {
        sql.push_str(" AND module like ?");
        binds.push(format!("%{}%", m));
    }
    sql.push_str(" ORDER BY run_time DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("数据库查询失败: {}", e))?;

    let params_vec: Vec<&dyn rusqlite::ToSql> =
        binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let mut rows = stmt
        .query(rusqlite::params_from_iter(params_vec))
        .map_err(|e| format!("数据库查询失败: {}", e))?;

    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("读取行失败!:{}", e))? {
        items.push(serde_json::json!({
            "task_id": row.get::<_,String>(0).unwrap_or_default(),
            "run_time": row.get::<_,String>(1).unwrap_or_default(),
            "SKU": row.get::<_,String>(2).unwrap_or_default(),
            "module": row.get::<_,String>(3).unwrap_or_default(),
            "status": row.get::<_,i32>(4).unwrap_or_default(),
        }));
    }
    // 这里可以添加获取任务列表的逻辑
    Ok(serde_json::json!({"status":"success","data":items}).to_string())
}

#[tauri::command]
async fn get_task_logs(task_id: String, db: tauri::State<'_, Db>) -> Result<String, String> {
    let conn = db.0.lock().await;
    let mut stmt = conn
        .prepare(
            r#"
         select 
            log_time,
            message 
         from task_logs 
         where 
            task_id = ?1 
        order by log_time
        "#,
        )
        .map_err(|e| format!("数据库查询失败: {}", e))?;
    let mut rows = stmt
        .query(rusqlite::params![task_id])
        .map_err(|e| format!("数据库查询失败: {}", e))?;

    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("读取行失败!:{}", e))? {
        items.push(serde_json::json!({
            "log_time": row.get::<_,String>(0).unwrap_or_default(),
            "message": row.get::<_,String>(1).unwrap_or_default(),
        }));
    }
    Ok(serde_json::json!({"status":"success","data":items}).to_string())
}

#[tauri::command]
async fn get_data() -> Result<String, String> {
    println!("get_data");
    println!("获取数据被调用");
    // 这里可以添加获取数据的逻辑
    match wps_reader::fetch_wps_data().await {
        Ok(data) => {
            println!("数据获取成功: {}", data);
            return Ok(data);
        }
        Err(e) => {
            eprintln!("数据获取失败: {}", e);
            return Err("数据获取失败".to_string());
        }
    }
}

#[tauri::command]
async fn execute_task(db: tauri::State<'_, Db>) -> Result<String, String> {
    let data_str = wps_reader::fetch_wps_data()
        .await
        .map_err(|e| format!("数据获取失败: {}", e))?;
    info!("获取到的数据: {}", data_str);
    let v: serde_json::Value =
        serde_json::from_str(&data_str).map_err(|e| format!("JSON解析失败: {}", e))?;
    let Some(items) = v.get("data").and_then(|d| d.as_array()) else {
        return Err("数据获取失败: 响应体中没有 'data' 字段".to_string());
    };
    let client = reqwest::Client::new();
    let mut inserted = 0;

    for item in items {
        let Some(fields) = item.get("fields") else {
            continue;
        };
        let row_id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let sku = fields.get("SKU").and_then(|v| v.as_str()).unwrap_or("");
        let module = fields
            .get("调用PS模版")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if sku.is_empty() || module.is_empty() {
            continue;
        }
        let task_id = Uuid::now_v7().to_string();
        let run_time = chrono::Local::now().to_rfc3339();
        let create_time = fields
            .get("创建时间")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        {
            let conn = db.0.lock().await;
            conn.execute(
                "INSERT INTO tasks (task_id, run_time, SKU, module, create_time, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![task_id, run_time, sku, module, create_time, 0],
            )
            .map_err(|e| format!("数据库插入失败: {}", e))?;
        }
        let resp = client
            .post("http://127.0.0.1:5000/automator")
            .json(fields)
            .send()
            .await
            .map_err(|e| format!("请求后台服务失败: {}", e))?;
        let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
        if json.get("status").and_then(|v| v.as_str()) == Some("error") {
            let message = json
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("未知错误");
            let conn = db.0.lock().await;
            conn.execute(
                "INSERT INTO task_logs (task_id, log_time, message) VALUES (?1, ?2, ?3)",
                params![
                    task_id,
                    chrono::Local::now().to_rfc3339(),
                    format!("任务执行失败: {}", message)
                ],
            )
            .map_err(|e| format!("日志插入失败: {}", e))?;
            wps_reader::update_wps_date(&row_id, "否")
                .await
                .map_err(|e| format!("更新任务状态失败: {}", e))?;
            continue;
        }
        if let Some(logs) = json.get("logs").and_then(|v| v.as_array()) {
            for log in logs {
                let message = log[1].as_str().unwrap_or("无日志信息");
                let log_time = log[0].as_str().unwrap_or("");
                let conn = db.0.lock().await;
                conn.execute(
                    "INSERT INTO task_logs (task_id, log_time, message) VALUES (?1, ?2, ?3)",
                    params![task_id, log_time, message],
                )
                .map_err(|e| format!("日志插入失败: {}", e))?;
            }
        }
        inserted += 1;
        wps_reader::update_wps_date(&row_id, "是")
            .await
            .map_err(|e| format!("更新任务状态失败: {}", e))?;
        let conn = db.0.lock().await;
        conn.execute(
            "UPDATE tasks SET status = 1 WHERE task_id = ?1",
            params![task_id],
        )
        .map_err(|e| format!("更新任务状态失败: {}", e))?;
    }
    Ok(format!("任务执行完成, 共插入 {} 条任务", inserted))
}

#[tauri::command]
async fn open_timing_window(app: tauri::AppHandle) {
    let lable = "timing";
    if let Some(window) = app.get_webview_window(lable) {
        if window.is_minimizable().unwrap_or(false) {
            window.unminimize().unwrap();
        }

        window.show().unwrap();
        window.set_focus().unwrap();

        // 获取主屏幕的尺寸
        if let Some(monitor) = window.primary_monitor().unwrap() {
            let screen_size = monitor.size();
            let window_size = window.inner_size().unwrap();

            // 计算窗口的中心位置
            let x = (screen_size.width - window_size.width) / 2;
            let y = (screen_size.height - window_size.height) / 2;

            // 将窗口移动到屏幕中心
            window
                .set_position(tauri::PhysicalPosition { x, y })
                .unwrap();
        }
    } else {
        tauri::WebviewWindowBuilder::new(&app, lable, tauri::WebviewUrl::App("timing.html".into()))
            .title("设置定时")
            .inner_size(500.0, 500.0)
            .resizable(false)
            .maximizable(false)
            .center()
            .build()
            .unwrap();
    }
}

#[tauri::command]
async fn open_logs_window(app: tauri::AppHandle) {
    let lable: &'static str = "logs";
    if let Some(window) = app.get_webview_window(lable) {
        if window.is_minimizable().unwrap_or(false) {
            window.unminimize().unwrap();
        }

        window.show().unwrap();
        window.set_focus().unwrap();

        // 获取主屏幕的尺寸
        if let Some(monitor) = window.primary_monitor().unwrap() {
            let screen_size = monitor.size();
            let window_size = window.inner_size().unwrap();

            // 计算窗口的中心位置
            let x = (screen_size.width - window_size.width) / 2;
            let y = (screen_size.height - window_size.height) / 2;

            // 将窗口移动到屏幕中心
            window
                .set_position(tauri::PhysicalPosition { x, y })
                .unwrap();
        }
    } else {
        tauri::WebviewWindowBuilder::new(&app, lable, tauri::WebviewUrl::App("log.html".into()))
            .title("查看日志")
            .inner_size(800.0, 600.0)
            .resizable(true)
            .maximizable(true)
            .center()
            .build()
            .unwrap();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(ChildProcess(Mutex::new(None)))
        .manage(Schedule(Mutex::new(None)))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let main_window = app
                .get_webview_window("main")
                .expect("main window not found");
            main_window.show().unwrap();
            main_window.set_focus().unwrap();
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            open_timing_window,
            get_data,
            open_logs_window,
            execute_task,
            get_task_list,
            get_task_logs,
            start_cron,
            stop_cron
        ])
        .setup(|app| {
            // 初始化Sqlite数据库
            let data_dir = app.path().app_data_dir().map_err(|e| anyhow::anyhow!(e))?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("app_data.db");
            println!("数据库路径: {:?}", db_path);
            let conn = Connection::open(&db_path)?;
            conn.execute(
                r#"
                create table if not exists tasks(
                 id integer primary key autoincrement,
                 task_id text not null,
                 run_time text not null,
                 SKU text not null,
                 module text not null,
                 create_time text not null,
                 status integer not null
                )
            "#,
                (),
            )?;
            conn.execute(
                r#"
                create table if not exists task_logs(
                id integer primary key autoincrement,
                task_id text not null,
                log_time text not null,
                message text not null)
            "#,
                (),
            )?;
            app.manage(Db(Mutex::new(conn)));
            // 启动后台服务
            let exe_path = app
                .path()
                .resource_dir()
                .ok()
                .map(|p| p.join("server/PhotoshopAutomator.exe"))
                .filter(|p| p.exists())
                .unwrap_or_else(|| {
                    std::path::PathBuf::from("src-tauri/server/PhotoshopAutomator.exe")
                });

            if exe_path.exists() {
                match Command::new(&exe_path).spawn() {
                    Ok(child) => {
                        let state: tauri::State<ChildProcess> = app.state();
                        tauri::async_runtime::block_on(async {
                            let mut guard = state.0.lock().await;
                            *guard = Some(child);
                        });
                        println!("后台服务已启动: {:?}", exe_path);
                    }
                    Err(e) => eprintln!("启动后台服务失败: {}", e),
                }
            } else {
                eprintln!("未找到后台服务可执行文件: {:?}", exe_path);
            }

            let show_i = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        id: _,
                        position: _,
                        rect: _,
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                    } => {
                        let win = tray
                            .app_handle()
                            .get_webview_window("main")
                            .expect("REASON");
                        match win.is_visible() {
                            Ok(visible) if !visible => {
                                win.show().unwrap();
                            }
                            Err(e) => eprint!("{}", e),
                            _ => (),
                        };
                        win.set_focus().unwrap();
                    }
                    _ => {}
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let win = app.get_webview_window("main").unwrap();
                        match win.is_visible() {
                            Ok(visible) if !visible => {
                                win.show().unwrap();
                            }
                            Err(e) => eprintln!("{}", e),
                            _ => (),
                        };
                        win.set_focus().unwrap();
                    }
                    "quit" => {
                        let app_cloned = app.clone();
                        tauri::async_runtime::spawn(async move {
                            stop_background_service(app_cloned).await;
                        });
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let app_handle = window.app_handle().clone();
            if let WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    "main" => {
                        api.prevent_close(); // 阻止窗口关闭
                        let _ = window.hide(); // 隐藏窗口
                    }
                    _ => {}
                }
            };
            if let WindowEvent::Destroyed = event {
                if window.label() == "main" {
                    let app_handle = window.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        stop_background_service(app_handle).await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
