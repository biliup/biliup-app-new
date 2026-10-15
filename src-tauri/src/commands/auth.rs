use crate::models::{ConfigRoot, User};
use crate::services::{AuthService, QrLoginCheckResult, SmsSendState};
use crate::{AppData, error::AppError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

async fn persist_login_success(
    config: Arc<Mutex<ConfigRoot>>,
    clients: Arc<Mutex<HashMap<u64, crate::MyClient>>>,
    bilibili: biliup::bilibili::BiliBili,
    user: User,
    proxy: Option<String>,
) {
    {
        let mut config_guard = config.lock().await;
        if let Some(existing_config) = config_guard.config.get_mut(&user.uid) {
            existing_config.user.name = user.username.clone();
            existing_config.user.cookie = bilibili.login_info.clone();
            existing_config.proxy = proxy.clone();
        } else {
            config_guard.new_user_config(
                user.uid,
                user.username.clone(),
                bilibili.login_info.clone(),
                proxy.clone(),
            );
        }
    }

    clients.lock().await.insert(
        user.uid,
        crate::MyClient {
            bilibili,
            user: user.clone(),
            proxy,
        },
    );
}

/// 获取登录二维码
#[tauri::command]
pub async fn get_login_qr(
    app: tauri::AppHandle,
    proxy: Option<String>,
) -> Result<String, AppError> {
    let app_data = app.state::<AppData>();
    let auth_service = app_data.auth_service.clone();
    let mut auth_service = auth_service.lock().await;
    auth_service.init(proxy.as_deref());

    let qrcode = auth_service
        .get_qr_code()
        .await
        .map_err(|e| AppError::Custom(crate::tr!("auth.qr_fetch_failed", error = e)))?;
    // info!("{}", qrcode);
    Ok(qrcode)
}

/// 检查二维码登录状态
#[tauri::command]
pub async fn check_qr_login(app: tauri::AppHandle) -> Result<LoginResponse, AppError> {
    let app_data = app.state::<AppData>();
    let auth_service = app_data.auth_service.clone();

    let (check_result, proxy) = {
        let auth_service = auth_service.lock().await;
        let check_result = auth_service
            .check_qr_login()
            .await
            .map_err(|e| AppError::Custom(crate::tr!("auth.qr_check_failed", error = e)))?;
        let proxy = auth_service.get_proxy();
        (check_result, proxy)
    };

    match check_result {
        QrLoginCheckResult::Success(login_info) => {
            let (bilibili, user) =
                AuthService::login_done_with_proxy(&login_info, proxy.as_deref())
                    .await
                    .map_err(|e| AppError::Custom(crate::tr!("auth.qr_check_failed", error = e)))?;

            let config = app_data.config.clone();
            let clients = app_data.clients.clone();
            persist_login_success(config, clients, bilibili, user.clone(), proxy).await;
            auth_service.lock().await.destroy();

            info!(
                "{}",
                crate::tr!(
                    "auth.qr_login_success",
                    uid = user.uid,
                    username = user.username
                )
            );

            Ok(LoginResponse {
                success: true,
                message: crate::i18n::text("auth.login_success").to_string(),
                status: Some("success".to_string()),
            })
        }
        QrLoginCheckResult::Pending => Ok(LoginResponse {
            success: false,
            message: crate::i18n::text("auth.qr_waiting").to_string(),
            status: Some("pending".to_string()),
        }),
        QrLoginCheckResult::Expired(message) => Ok(LoginResponse {
            success: false,
            message,
            status: Some("expired".to_string()),
        }),
        QrLoginCheckResult::Error(message) => Ok(LoginResponse {
            success: false,
            message,
            status: Some("error".to_string()),
        }),
        QrLoginCheckResult::Idle => Ok(LoginResponse {
            success: false,
            message: crate::i18n::text("auth.qr_required").to_string(),
            status: Some("idle".to_string()),
        }),
    }
}

/// 退出登录
#[tauri::command]
pub async fn logout_user(app: tauri::AppHandle, uid: u64) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();

    let queue = app_data
        .upload_service
        .get_upload_queue()
        .await
        .map_err(AppError::Internal)?;
    if queue
        .iter()
        .any(|task| task.user.uid == uid && !task.is_completed() && !task.is_cancelled())
    {
        return Err(AppError::Custom(
            crate::i18n::text("auth.unfinished_uploads").into(),
        ));
    }
    let mut clients = app_data.clients.lock().await;
    if !clients.contains_key(&uid) {
        return Err(AppError::UserNotFound(uid));
    }
    let mut config = app_data.config.lock().await;
    let mut updated = config.clone();
    updated
        .remove_user_config(uid)
        .map_err(|e| AppError::Config(crate::tr!("config.user_clear_failed", error = e)))?;
    updated
        .save_logout_to_file(&crate::utils::get_config_json_path().map_err(AppError::Internal)?)
        .map_err(|e| AppError::Config(crate::tr!("auth.logout_save_failed", error = e)))?;
    *config = updated;
    clients.remove(&uid);
    app_data.upload_service.risk.clear(uid);
    info!("{}", crate::tr!("auth.logout_success", uid = uid));
    Ok(true)
}

/// 发送短信验证码
#[tauri::command]
pub async fn send_sms_code(
    app: tauri::AppHandle,
    phone: String,
    country_code: String,
    proxy: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let app_data = app.state::<AppData>();
    let auth_service = app_data.auth_service.clone();

    let phone_number = phone
        .trim()
        .parse::<u64>()
        .map_err(|_| AppError::Custom(crate::i18n::text("auth.invalid_phone").to_string()))?;
    let country_code_number = country_code.trim().parse::<u32>().map_err(|_| {
        AppError::Custom(crate::i18n::text("auth.invalid_country_code").to_string())
    })?;

    let mut auth_service = auth_service.lock().await;
    auth_service.init(proxy.as_deref());
    let result = auth_service
        .send_sms_code(phone_number, country_code_number)
        .await
        .map_err(|e| AppError::Custom(crate::tr!("auth.sms_send_failed", error = e)))?;

    info!("{}", crate::tr!("auth.sms_requested"));

    match result {
        SmsSendState::Sent => Ok(json!({
            "success": true,
            "message": crate::i18n::text("auth.sms_sent")
        })),
        SmsSendState::NeedRecaptcha(recaptcha_url) => Ok(json!({
            "success": false,
            "needRecaptcha": true,
            "message": crate::i18n::text("auth.slider_required"),
            "recaptchaUrl": recaptcha_url
        })),
    }
}

/// 提交短信滑块验证参数并继续发送验证码
#[tauri::command]
pub async fn submit_sms_recaptcha(
    app: tauri::AppHandle,
    challenge: String,
    validate: String,
) -> Result<serde_json::Value, AppError> {
    let app_data = app.state::<AppData>();
    let auth_service = app_data.auth_service.clone();
    let mut auth_service = auth_service.lock().await;

    auth_service
        .submit_sms_recaptcha(challenge.trim(), validate.trim())
        .await
        .map_err(|e| AppError::Custom(crate::tr!("auth.slider_submit_failed", error = e)))?;

    Ok(json!({
        "success": true,
        "message": crate::i18n::text("auth.sms_sent")
    }))
}

/// 短信登录
#[tauri::command]
pub async fn login_with_sms(
    app: tauri::AppHandle,
    phone: String,
    country_code: String,
    code: String,
    proxy: Option<String>,
) -> Result<LoginResponse, AppError> {
    let app_data = app.state::<AppData>();
    let auth_service = app_data.auth_service.clone();

    // 第1步：校验输入并获取 LoginInfo
    let login_info = {
        let mut auth_service = auth_service.lock().await;

        if auth_service.get_proxy().is_none() && proxy.is_some() {
            auth_service.set_proxy_and_credential(proxy.as_deref());
        }

        let phone_number = phone
            .trim()
            .parse::<u64>()
            .map_err(|_| AppError::Custom(crate::i18n::text("auth.invalid_phone").to_string()))?;
        let country_code_number = country_code.trim().parse::<u32>().map_err(|_| {
            AppError::Custom(crate::i18n::text("auth.invalid_country_code").to_string())
        })?;
        let sms_code = code
            .trim()
            .parse::<u32>()
            .map_err(|_| AppError::Custom(crate::i18n::text("auth.invalid_code").to_string()))?;

        auth_service
            .login_with_sms_code_and_phone(sms_code, Some(phone_number), Some(country_code_number))
            .await
            .map_err(|e| AppError::Custom(crate::tr!("auth.sms_login_failed", error = e)))?
    };

    // 第2步：网络请求（获取用户信息）在锁外执行
    let proxy_opt = {
        let auth_service = auth_service.lock().await;
        auth_service.get_proxy()
    };

    let (bilibili, user) = AuthService::login_done_with_proxy(&login_info, proxy_opt.as_deref())
        .await
        .map_err(|e| AppError::Custom(crate::tr!("auth.sms_login_failed", error = e)))?;

    // 第3步：持久化结果
    let config = app_data.config.clone();
    let clients = app_data.clients.clone();
    persist_login_success(config, clients, bilibili, user.clone(), proxy_opt).await;
    auth_service.lock().await.destroy();

    info!(
        "{}",
        crate::tr!(
            "auth.sms_login_success",
            uid = user.uid,
            username = user.username
        )
    );

    Ok(LoginResponse {
        success: true,
        message: crate::i18n::text("auth.login_success").to_string(),
        status: None,
    })
}

/// 获取所有已保存的用户
#[tauri::command]
pub async fn get_login_users(app: tauri::AppHandle) -> Result<Vec<User>, AppError> {
    let app_data = app.state::<AppData>();
    let user_order = app_data.config.lock().await.user_order.clone();
    let clients = app_data.clients.lock().await;

    let mut users = Vec::new();
    let mut used_uids = HashSet::new();

    for uid in &user_order {
        if let Some(client) = clients.get(uid) {
            users.push(client.user.clone());
            used_uids.insert(*uid);
        }
    }

    let mut missing_users: Vec<(u64, User)> = clients
        .iter()
        .filter_map(|(uid, client)| {
            if used_uids.contains(uid) {
                None
            } else {
                Some((*uid, client.user.clone()))
            }
        })
        .collect();

    missing_users.sort_unstable_by_key(|(uid, _)| *uid);
    users.extend(missing_users.into_iter().map(|(_, user)| user));

    Ok(users)
}
