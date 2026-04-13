use crate::AppData;
use crate::models::{ConfigRoot, User};
use crate::services::{AuthService, QrLoginCheckResult, SmsSendState};
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
        },
    );
}

/// 获取登录二维码
#[tauri::command]
pub async fn get_login_qr(app: tauri::AppHandle, proxy: Option<String>) -> Result<String, String> {
    let app_data = app.state::<Mutex<AppData>>();
    let auth_service = {
        let app_data = app_data.lock().await;
        app_data.auth_service.clone()
    };
    let mut auth_service = auth_service.lock().await;
    auth_service.init(proxy.as_deref());

    let qrcode = auth_service
        .get_qr_code()
        .await
        .map_err(|e| format!("获取二维码失败: {e}"))?;
    // info!("{}", qrcode);
    Ok(qrcode)
}

/// 检查二维码登录状态
#[tauri::command]
pub async fn check_qr_login(app: tauri::AppHandle) -> Result<LoginResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let auth_service = {
        let app_data = app_lock.lock().await;
        app_data.auth_service.clone()
    };

    let (check_result, proxy) = {
        let auth_service = auth_service.lock().await;
        let check_result = auth_service
            .check_qr_login()
            .await
            .map_err(|e| format!("二维码登录状态失败: {e}"))?;
        let proxy = auth_service.get_proxy();
        (check_result, proxy)
    };

    match check_result {
        QrLoginCheckResult::Success(login_info) => {
            let (bilibili, user) =
                AuthService::login_done_with_proxy(&login_info, proxy.as_deref())
                    .await
                    .map_err(|e| format!("二维码登录状态失败: {e}"))?;

            let (config, clients) = {
                let app_data = app_lock.lock().await;
                (app_data.config.clone(), app_data.clients.clone())
            };
            persist_login_success(config, clients, bilibili, user.clone(), proxy).await;
            auth_service.lock().await.destroy();

            info!("用户：{} - {} 通过二维码登录成功", user.uid, user.username);

            Ok(LoginResponse {
                success: true,
                message: "登录成功".to_string(),
                status: Some("success".to_string()),
            })
        }
        QrLoginCheckResult::Pending => Ok(LoginResponse {
            success: false,
            message: "等待扫码确认".to_string(),
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
            message: "请先获取二维码".to_string(),
            status: Some("idle".to_string()),
        }),
    }
}

/// 退出登录
#[tauri::command]
pub async fn logout_user(app: tauri::AppHandle, uid: u64) -> Result<bool, String> {
    let app_data = app.state::<Mutex<AppData>>();
    let app_data = app_data.lock().await;

    if app_data.clients.lock().await.remove(&uid).is_some() {
        let _ = app_data
            .config
            .lock()
            .await
            .remove_user_config(uid)
            .map_err(|e| format!("清除用户配置失败: {e}"))?;
        info!("用户 {} 登出成功", uid);
        Ok(true)
    } else {
        Err("用户未登录".into())
    }
}

/// 发送短信验证码
#[tauri::command]
pub async fn send_sms_code(
    app: tauri::AppHandle,
    phone: String,
    country_code: String,
    proxy: Option<String>,
) -> Result<serde_json::Value, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let auth_service = {
        let app_data = app_lock.lock().await;
        app_data.auth_service.clone()
    };

    let phone_number = phone
        .trim()
        .parse::<u64>()
        .map_err(|_| "手机号格式错误".to_string())?;
    let country_code_number = country_code
        .trim()
        .parse::<u32>()
        .map_err(|_| "国家代码格式错误".to_string())?;

    let mut auth_service = auth_service.lock().await;
    auth_service.init(proxy.as_deref());
    let result = auth_service
        .send_sms_code(phone_number, country_code_number)
        .await
        .map_err(|e| format!("发送验证码失败: {e}"))?;

    let full_phone = format!("+{country_code}{phone}");
    info!("请求发送验证码到: {}", full_phone);

    match result {
        SmsSendState::Sent => Ok(json!({
            "success": true,
            "message": "验证码已发送"
        })),
        SmsSendState::NeedRecaptcha(recaptcha_url) => Ok(json!({
            "success": false,
            "needRecaptcha": true,
            "message": "需要先完成滑块验证",
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
) -> Result<serde_json::Value, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let auth_service = {
        let app_data = app_lock.lock().await;
        app_data.auth_service.clone()
    };
    let mut auth_service = auth_service.lock().await;

    auth_service
        .submit_sms_recaptcha(challenge.trim(), validate.trim())
        .await
        .map_err(|e| format!("滑块验证提交失败: {e}"))?;

    Ok(json!({
        "success": true,
        "message": "验证码已发送"
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
) -> Result<LoginResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let auth_service = {
        let app_data = app_lock.lock().await;
        app_data.auth_service.clone()
    };

    // 第1步：校验输入并获取 LoginInfo
    let login_info = {
        let mut auth_service = auth_service.lock().await;

        if auth_service.get_proxy().is_none() && proxy.is_some() {
            auth_service.set_proxy_and_credential(proxy.as_deref());
        }

        let phone_number = phone
            .trim()
            .parse::<u64>()
            .map_err(|_| "手机号格式错误".to_string())?;
        let country_code_number = country_code
            .trim()
            .parse::<u32>()
            .map_err(|_| "国家代码格式错误".to_string())?;
        let sms_code = code
            .trim()
            .parse::<u32>()
            .map_err(|_| "验证码格式错误".to_string())?;

        auth_service
            .login_with_sms_code_and_phone(sms_code, Some(phone_number), Some(country_code_number))
            .await
            .map_err(|e| format!("短信登录失败: {e}"))?
    };

    // 第2步：网络请求（获取用户信息）在锁外执行
    let proxy_opt = {
        let auth_service = auth_service.lock().await;
        auth_service.get_proxy()
    };

    let (bilibili, user) = AuthService::login_done_with_proxy(&login_info, proxy_opt.as_deref())
        .await
        .map_err(|e| format!("短信登录失败: {e}"))?;

    // 第3步：持久化结果
    let (config, clients) = {
        let app_data = app_lock.lock().await;
        (app_data.config.clone(), app_data.clients.clone())
    };
    persist_login_success(config, clients, bilibili, user.clone(), proxy_opt).await;
    auth_service.lock().await.destroy();

    let full_phone = format!("+{country_code}{phone}");
    info!(
        "用户：{} - {} 通过短信登录成功，手机号 {}",
        user.uid, user.username, full_phone
    );

    Ok(LoginResponse {
        success: true,
        message: "登录成功".to_string(),
        status: None,
    })
}

/// 获取所有已保存的用户
#[tauri::command]
pub async fn get_login_users(app: tauri::AppHandle) -> Result<Vec<User>, String> {
    let app_data = app.state::<Mutex<AppData>>();
    let app_data = app_data.lock().await;
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
