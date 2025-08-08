use crate::AppData;
use crate::models::User;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
}

/// 获取登录二维码
#[tauri::command]
pub async fn get_login_qr(app: tauri::AppHandle, proxy: Option<String>) -> Result<String, String> {
    let app_data = app.state::<Mutex<AppData>>();
    let auth_service = &mut app_data.lock().await.auth_service;
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
    let mut app_data = app_lock.lock().await;

    let (bilibili, user) = app_data
        .auth_service
        .qr_login()
        .await
        .map_err(|e| format!("二维码登录状态失败: {e}"))?;

    let proxy = app_data.auth_service.get_proxy();
    app_data.config.lock().await.new_user_config(
        user.uid,
        user.username.clone(),
        bilibili.login_info.clone(),
        proxy,
    );

    app_data.clients.lock().await.insert(
        user.uid,
        crate::MyClient {
            bilibili,
            user: user.clone(),
        },
    );
    app_data.auth_service.destroy();

    Ok(LoginResponse {
        success: true,
        message: "登录成功".to_string(),
    })
}

/// Cookie 登录
#[tauri::command]
pub async fn login_with_cookie(
    _cookie: String,
    _proxy: Option<String>,
) -> Result<LoginResponse, String> {
    // TODO: 实现 Cookie 登录逻辑
    Ok(LoginResponse {
        success: true,
        message: "登录成功".to_string(),
    })
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

/// 密码登录
#[tauri::command]
pub async fn login_with_password(
    app: tauri::AppHandle,
    username: String,
    password: String,
    proxy: Option<String>,
) -> Result<LoginResponse, String> {
    let result = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async {
            let app_data = app.state::<Mutex<AppData>>();
            let auth_service = &mut app_data.lock().await.auth_service;
            auth_service.init(proxy.as_deref());

            auth_service
                .login_with_username_password(&username, &password)
                .await
        })
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?;

    let (_bilibili, _user) = result.map_err(|e| format!("登录失败: {e}"))?;
    // info!("用户 {} 登录成功", user.username);

    Ok(LoginResponse {
        success: true,
        message: "登录成功".to_string(),
    })
}

/// 发送短信验证码
#[tauri::command]
pub async fn send_sms_code(
    phone: String,
    country_code: String,
    _proxy: Option<String>,
) -> Result<serde_json::Value, String> {
    // TODO: 实现发送短信验证码逻辑
    // 完整手机号格式: +{country_code}{phone}
    let full_phone = format!("+{country_code}{phone}");
    info!("发送验证码到: {}", full_phone);

    Ok(serde_json::json!({
        "success": true,
        "message": "验证码已发送"
    }))
}

/// 短信登录
#[tauri::command]
pub async fn login_with_sms(
    phone: String,
    country_code: String,
    code: String,
    _proxy: Option<String>,
) -> Result<LoginResponse, String> {
    // TODO: 实现短信登录逻辑
    // 完整手机号格式: +{country_code}{phone}
    let full_phone = format!("+{country_code}{phone}");
    info!("短信登录: {} 验证码: {}", full_phone, code);

    Ok(LoginResponse {
        success: true,
        message: "登录成功".to_string(),
    })
}

/// 获取所有已保存的用户
#[tauri::command]
pub async fn get_login_users(app: tauri::AppHandle) -> Result<Vec<User>, String> {
    let app_data = app.state::<Mutex<AppData>>();

    Ok(app_data
        .lock()
        .await
        .clients
        .lock()
        .await
        .values()
        .map(|client| client.user.clone())
        .collect())
}
