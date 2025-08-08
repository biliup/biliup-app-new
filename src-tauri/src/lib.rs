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
#[cfg(debug_assertions)]
use tracing::Level;
use tracing::{error, info};
#[cfg(debug_assertions)]
use tracing_subscriber::FmtSubscriber;
use utils::CompatibilityConverter;

use crate::{
    models::{ConfigRoot, User},
    services::{AuthService, upload_service::UploadService},
    utils::{crypto::encode_base64, get_config_json_path},
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // 避免 Tauri #9453
    // 仅调试模式启用tracing
    #[cfg(debug_assertions)]
    {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }

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

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app: &mut tauri::App| {
            // let stdout_log = tracing_subscriber::fmt::layer()
            //     .pretty()
            //     .with_filter(LevelFilter::INFO);
            // let file_appender =
            //     tracing_appender::rolling::never(config_path(app.handle())?, "biliup.log");
            // // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            // let file_layer = tracing_subscriber::fmt::layer()
            //     .with_ansi(false)
            //     .with_writer(file_appender)
            //     .with_filter(LevelFilter::INFO);
            // Registry::default().with(stdout_log).with(file_layer).init();
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let windows = app.webview_windows();
                for (name, window) in windows {
                    error!("Window name: {}", name);
                    window.open_devtools();
                }
            }
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
            get_upload_progress,
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
            upload_cover,
            download_cover,
            get_type_list,
            get_topic_list,
            search_topics,
            get_season_list,
            get_video_detail,
        ])
        .setup(|app: &mut tauri::App| {
            app.manage(Mutex::new(appdata));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
