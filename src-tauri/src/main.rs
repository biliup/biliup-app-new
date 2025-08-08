// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    unsafe {
        // 设置环境变量
        std::env::set_var("RUST_BACKTRACE", "0");
    }

    biliup_app_lib::run().await
}
