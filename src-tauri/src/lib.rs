mod commands;
mod models;
mod services;
mod utils;

use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

use biliup::bilibili::BiliBili;
use commands::*;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::{error, info};
use utils::CompatibilityConverter;

use crate::{
    models::{ConfigRoot, User},
    services::{AuthService, upload_service::UploadService},
    utils::{crypto::encode_base64, get_config_json_path, get_log_path},
};

pub struct MyClient {
    bilibili: BiliBili,
    user: User,
}

pub struct AppData {
    config: Arc<Mutex<ConfigRoot>>,
    auth_service: AuthService,
    upload_service: UploadService,
    clients: Arc<Mutex<HashMap<u64, MyClient>>>,
    // client: StatelessClient,
}

async fn startup() -> Result<AppData> {
    let config = ConfigRoot::from_file(&get_config_json_path()?)?;
    let mut clients = HashMap::new();

    for user_config in config.config.values() {
        let bilibili = biliup::credential::bilibili_from_info(
            user_config.user.cookie.clone(),
            user_config.proxy.as_deref(),
        )?;

        let myinfo = bilibili.my_info().await?;
        let username = myinfo["data"]["name"].as_str().unwrap().to_owned();
        let uid = myinfo["data"]["mid"].as_u64().unwrap_or(0);
        let avatar_url = myinfo["data"]["face"].as_str().unwrap_or("").to_string();

        let avatar = bilibili
            .client
            .get(avatar_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("获取用户头像失败: {}", e))?
            .bytes()
            .await?;
        let avatar = encode_base64(&avatar);

        clients.insert(
            uid,
            MyClient {
                bilibili,
                user: User::new(uid, username, avatar),
            },
        );
    }

    let max_curr = config.max_curr;
    Ok(AppData {
        config: Arc::new(Mutex::new(config)),
        auth_service: AuthService::new(),
        upload_service: UploadService::new(max_curr),
        clients: Arc::new(Mutex::new(clients)),
    })
}

fn setup_logs(log_level: &str) -> Result<()> {
    let log_dir = get_log_path()?;
    let log_file = format!(
        "biliup-{}.log",
        chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S")
    );
    let file_appender = tracing_appender::rolling::never(log_dir, &log_file);

    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing_subscriber::filter::LevelFilter::TRACE,
        "debug" => tracing_subscriber::filter::LevelFilter::DEBUG,
        "info" => tracing_subscriber::filter::LevelFilter::INFO,
        "warn" => tracing_subscriber::filter::LevelFilter::WARN,
        "error" => tracing_subscriber::filter::LevelFilter::ERROR,
        _ => tracing_subscriber::filter::LevelFilter::INFO,
    };

    use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // 禁用 ANSI 颜色代码
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true) // 启用 ANSI 颜色代码
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    // !!!!!!
    // !!!!!! Attention
    // Tauri #9453提出启用tracing后traui应用有概率卡死
    // 在发行版本中尽量少的输出日志

    #[cfg(debug_assertions)]
    tracing_subscriber::registry()
        .with(file_layer.with_filter(level))
        .with(console_layer.with_filter(tracing_subscriber::filter::LevelFilter::TRACE))
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::registry()
        .with(file_layer.with_filter(level))
        .with(console_layer.with_filter(level))
        .init();

    info!("日志已输出到: {:?} {}", get_log_path()?, log_file);

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // 启动时进行兼容性检查
    if let Err(e) = CompatibilityConverter::startup_with_compatibility().await {
        info!("无旧biliup配置: {}", e);
    }

    let appdata = startup().await.unwrap_or_else(|e| {
        error!("加载配置失败: {}", e);
        let config = ConfigRoot::default();
        let max_curr = config.max_curr;
        AppData {
            config: Arc::new(Mutex::new(config)),
            auth_service: AuthService::new(),
            upload_service: UploadService::new(max_curr),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    });

    setup_logs(&appdata.config.lock().await.log_level.clone()).expect("日志初始化失败");

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app: &mut tauri::App| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let windows = app.webview_windows();
                for (_name, window) in windows {
                    window.open_devtools();
                }
            }

            // 管理应用数据状态
            app.manage(Mutex::new(appdata));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 认证相关命令
            get_login_qr,
            check_qr_login,
            login_with_cookie,
            login_with_password,
            send_sms_code,
            login_with_sms,
            get_login_users,
            logout_user,
            // 上传相关命令
            create_upload_task,
            start_upload,
            pause_upload,
            cancel_upload,
            get_upload_queue,
            retry_upload,
            submit,
            // 配置相关命令
            load_config,
            save_config,
            save_user_config,
            save_global_config,
            add_user_template,
            update_user_template,
            delete_user_template,
            // 其他命令
            get_current_version,
            get_file_size,
            read_dir_recursive,
            upload_cover,
            download_cover,
            get_type_list,
            get_topic_list,
            search_topics,
            get_season_list,
            get_video_detail,
            get_video_season,
            switch_season,
            export_logs,
            check_update,
            console_log
        ])
        .run(tauri::generate_context!())
        .expect("运行Tauri应用程序时出错");
}
