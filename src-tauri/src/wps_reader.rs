use log::{error, info};
use reqwest::{header, Error};
use serde_json::json;
use std::env;
use dotenv::dotenv;

pub async fn fetch_wps_data() -> Result<String, Error> {
    dotenv().ok();
    let url = env::var("wps_url").expect("WPS_URL must be set in .env file");
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    let token = env::var("Airscript_Token").expect("Airscript-Token must be set in .env file");
    headers.insert(
        "Airscript-Token",
        header::HeaderValue::from_str(&token).expect("Invalid token"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );

    let payload = json!(
        {
            "Context":{
                "argv":{
                    "action":"search",
                    "sheet":"数据表",
                    "filter":{
                        "mode":"AND",
                        "criteria":[
                            {
                                "field":"设计师确认是否已完整填写可运行",
                                "op":"Equals",
                                "values":[
                                    "是"
                                ]
                            },
                            {
                                "field":"是否运行完成",
                                "op":"Equals",
                                "values":[""]
                            }
                        ]
                    }
                }
            }
        }
    );

    let response = client
        .post(url)
        .headers(headers)
        .json(&payload)
        .send()
        .await?;

    let response_json: serde_json::Value = response.json().await?;
    response_json.get("data").map_or_else(
        || {
            error!("数据获取失败: 响应体中没有 'data' 字段");
            let error_message = json!({
                "status": "Error",
                "message": "数据获取失败"
            });
            Ok(error_message.to_string())
        },
        |data| {
            let success_message = json!({
                "status": "Success",
                "data": data.get("result").and_then(|result| result.get("data"))
            });
            Ok(success_message.to_string())
        },
    )
}

pub async fn update_wps_date(target_id: &str, res: &str) -> Result<String, Error> {
    dotenv().ok();
    let url = env::var("wps_url").expect("WPS_URL must be set in .env file");
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    let token = env::var("Airscript_Token").expect("Airscript-Token must be set in .env file");
    headers.insert(
        "Airscript-Token",
        header::HeaderValue::from_str(&token).expect("Invalid token"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );
    let payload = json!({
      "Context": {
        "argv": {
          "action": "update",
          "sheet": "数据表",
          "records": [
            {
              "fields":
                {
                  "是否运行完成": res
                }
              ,
              "id": target_id
            }
          ]
        }
      }
    });
    let response = client
        .post(url)
        .headers(headers)
        .json(&payload)
        .send()
        .await?;
    let response_json: serde_json::Value = response.json().await?;
    response_json.get("status").map_or_else(
        || {
            error!("解析失败没有获取到status字段");
            let error_message = json!({
                "status": "Error",
                "message": "更新失败"
            });
            Ok(error_message.to_string())
        },
        |status| {
            info!("更新成功: {}", status);
            let success_message = json!({
                "status": "Success",
                "message": "更新成功"
            });
            Ok(success_message.to_string())
        },
    )
}
