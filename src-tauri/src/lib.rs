mod commands;
pub(crate) mod error;
mod i18n;
mod models;
mod services;
mod utils;

use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

use biliup::bilibili::BiliBili;
use commands::*;
use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use tokio::sync::Mutex;
use tracing::{info, warn};
use utils::CompatibilityConverter;

use crate::{
    error::AppError,
    models::{ConfigRoot, User},
    services::{AuthService, upload_service::UploadService},
    utils::{crypto::encode_base64, get_config_json_path, get_log_path},
};

#[derive(Clone)]
pub struct MyClient {
    pub bilibili: BiliBili,
    pub user: User,
    pub proxy: Option<String>,
}

pub struct AppData {
    pub startup_error: Option<String>,
    pub config: Arc<Mutex<ConfigRoot>>,
    pub auth_service: Arc<Mutex<AuthService>>,
    pub upload_service: UploadService,
    pub clients: Arc<Mutex<HashMap<u64, MyClient>>>,
}

impl AppData {
    pub async fn get_client(&self, uid: u64) -> Result<MyClient, AppError> {
        self.clients
            .lock()
            .await
            .get(&uid)
            .cloned()
            .ok_or_else(|| AppError::UserNotFound(uid))
    }

    pub async fn get_bilibili(&self, uid: u64) -> Result<BiliBili, AppError> {
        self.clients
            .lock()
            .await
            .get(&uid)
            .map(|c| c.bilibili.clone())
            .ok_or_else(|| AppError::UserNotFound(uid))
    }
}

async fn startup() -> Result<AppData> {
    let config_path = get_config_json_path()?;
    let config =
        if config_path.try_exists()? || config_path.with_extension("json.bak").try_exists()? {
            ConfigRoot::from_file(&config_path)?
        } else {
            ConfigRoot::default()
        };
    i18n::set_language(&config.language);
    let mut clients = HashMap::new();

    for user_config in config.config.values() {
        let bilibili = match biliup::credential::bilibili_from_info(
            user_config.user.cookie.clone(),
            user_config.proxy.as_deref(),
        ) {
            Ok(client) => client,
            Err(e) => {
                warn!(
                    "{}",
                    crate::tr!(
                        "startup.account_init_failed",
                        uid = user_config.user.uid,
                        error = e
                    )
                );
                continue;
            }
        };

        let fallback_uid = user_config.user.uid;
        let fallback_name = user_config.user.name.clone();

        let myinfo = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            bilibili.my_info(),
        )
        .await
        {
            Ok(Ok(info)) => info,
            result => {
                warn!(
                    "{}",
                    crate::tr!(
                        "startup.account_verify_failed",
                        uid = fallback_uid,
                        error = format!("{:?}", result)
                    )
                );
                clients.insert(
                    fallback_uid,
                    MyClient {
                        bilibili,
                        user: User::new(fallback_uid, fallback_name, String::new(), false),
                        proxy: user_config.proxy.clone(),
                    },
                );
                continue;
            }
        };
        let code = myinfo["code"].as_i64().unwrap_or(-1);
        let uid = myinfo["data"]["mid"].as_u64().unwrap_or(0);

        if code == 0 && uid > 0 {
            let username = myinfo["data"]["name"].as_str().unwrap_or("").to_owned();
            let avatar_url = myinfo["data"]["face"].as_str().unwrap_or("").to_string();
            let avatar = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                let response = bilibili.client.get(avatar_url).send().await?;
                let bytes = response.error_for_status()?.bytes().await?;
                Ok::<_, reqwest::Error>(encode_base64(&bytes))
            })
            .await;
            let avatar = match avatar {
                Ok(Ok(avatar)) => avatar,
                _ => String::new(),
            };

            clients.insert(
                uid,
                MyClient {
                    bilibili,
                    user: User::new(uid, username, avatar, false),
                    proxy: user_config.proxy.clone(),
                },
            );
        } else {
            let msg = myinfo["message"]
                .as_str()
                .unwrap_or(crate::i18n::text("common.unknown_reason"));
            info!(
                "{}",
                crate::tr!(
                    "startup.invalid_login",
                    username = fallback_name,
                    code = code,
                    message = msg
                )
            );
            clients.insert(
                fallback_uid,
                MyClient {
                    bilibili,
                    user: User::new(fallback_uid, fallback_name, "".to_string(), true),
                    proxy: user_config.proxy.clone(),
                },
            );
        }
    }

    let max_curr = config.max_curr;
    Ok(AppData {
        startup_error: None,
        config: Arc::new(Mutex::new(config)),
        auth_service: Arc::new(Mutex::new(AuthService::new())),
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
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::NEVER)
        .filename_prefix(log_file.clone())
        .build(log_dir)?;

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

    info!(
        "{}",
        crate::tr!(
            "logging.output",
            directory = format!("{:?}", get_log_path()?),
            level = log_file
        )
    );

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // 启动时进行兼容性检查
    if let Err(e) = CompatibilityConverter::startup_with_compatibility().await {
        info!("{}", crate::tr!("config.legacy_missing", error = e));
    }

    let appdata = match startup().await {
        Ok(data) => data,
        Err(e) => {
            let path = get_config_json_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|error| crate::tr!("config.path_unavailable", error = error));
            let config = ConfigRoot {
                read_only: true,
                ..ConfigRoot::default()
            };
            AppData {
                startup_error: Some(crate::tr!("config.load_failed", path = path, error = e)),
                config: Arc::new(Mutex::new(config)),
                auth_service: Arc::new(Mutex::new(AuthService::new())),
                upload_service: UploadService::new(0),
                clients: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    };

    if let Err(e) = setup_logs(&appdata.config.lock().await.log_level.clone()) {
        eprintln!("{}", crate::tr!("logging.init_failed", error = e));
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(move |app: &mut tauri::App| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let windows = app.webview_windows();
                for (_name, window) in windows {
                    window.open_devtools();
                }
            }

            // 管理应用数据状态
            appdata
                .upload_service
                .risk
                .set_app_handle(app.handle().clone());
            app.manage(appdata);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 认证相关命令
            get_login_qr,
            check_qr_login,
            send_sms_code,
            submit_sms_recaptcha,
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
            get_startup_error,
            load_config,
            save_config,
            save_user_config,
            save_global_config,
            add_user_template,
            update_user_template,
            delete_user_template,
            rename_user_template,
            save_template_order,
            save_user_order,
            // 其他命令
            get_current_version,
            get_file_size,
            get_avatar_cache_dir,
            read_dir_recursive,
            upload_cover,
            download_cover,
            get_archive_pre,
            get_topic_list,
            search_topics,
            search_mention,
            get_season_list,
            get_video_detail,
            get_video_season,
            switch_season,
            export_logs,
            check_update,
            console_log
        ])
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { .. } => {
                    // 在窗口关闭前保存状态
                    let _ = window.app_handle().save_window_state(StateFlags::all());
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect(crate::i18n::text("app.run_failed"));
}
