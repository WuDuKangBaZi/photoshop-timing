# Photoshop自动化控制端

## Tauri+Vue+TypeScript ## 

[photoshop实际服务端](https://github.com/WuDuKangBaZi/photoshop-server)

- 具体逻辑 

    Tauri+Vue3+Element-plus构建前端界面，用于界面展示和设置cron等关键功能
    在后端使用Rust.cron 来控制定时任务
    
- 前后端交互
    - 控制端
        本地通过invoke来访问后端Rust方法
    - 服务端
        程序启动时调用Python打包的实际操作逻辑，通过Flask创建http接口，控制端访问http接口来实际操作程序。

- 本项目为客户项目需求，内部任何技术资料无法提供额外支持。