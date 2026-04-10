use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::info;

use crate::{AppData, models::ConfigRoot};
use crate::{models::TemplateConfig, utils::get_config_json_path};

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateCommandResponse {
    pub success: bool,
    pub message: String,
    pub template: Option<TemplateConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateOrderCommandResponse {
    pub success: bool,
    pub message: String,
    pub template_order: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameTemplateCommandResponse {
    pub success: bool,
    pub message: String,
    pub template_order: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserOrderCommandResponse {
    pub success: bool,
    pub message: String,
    pub user_order: Vec<u64>,
}

/// 加载配置文件
#[tauri::command]
pub async fn load_config(app: AppHandle) -> Result<ConfigRoot, String> {
    let data = app.state::<Mutex<AppData>>();

    Ok(data.lock().await.config.lock().await.clone())
}

/// 保存配置文件
#[tauri::command]
pub async fn save_config(app: AppHandle) -> Result<bool, String> {
    let data = app.state::<Mutex<AppData>>();

    data.lock()
        .await
        .config
        .lock()
        .await
        .save_to_file(&get_config_json_path().map_err(|e| format!("获取配置路径失败: {e}"))?)
        .map_err(|e| format!("保存配置失败: {e}"))?;
    Ok(true)
}

#[tauri::command]
pub async fn save_user_config(
    app: AppHandle,
    uid: u64,
    line: Option<String>,
    proxy: Option<String>,
    limit: u32,
    watermark: u8,
    auto_edit: u8,
) -> Result<bool, String> {
    let data = app.state::<Mutex<AppData>>();
    info!("用户({uid})配置已保存");

    data.lock()
        .await
        .config
        .lock()
        .await
        .save_user_config(uid, line, proxy, limit, watermark, auto_edit)
        .map_err(|e| format!("保存用户配置失败: {e}"))?;
    Ok(true)
}

#[tauri::command]
pub async fn save_global_config(
    app: AppHandle,
    max_curr: u32,
    auto_start: bool,
    auto_upload: bool,
    log_level: String,
) -> Result<bool, String> {
    let data = app.state::<Mutex<AppData>>();

    info!("全局配置已保存");

    data.lock().await.config.lock().await.save_global_config(
        max_curr,
        auto_start,
        auto_upload,
        log_level,
    );

    data.lock()
        .await
        .upload_service
        .set_max_concurrent(max_curr)
        .await;
    Ok(true)
}

#[tauri::command]
pub async fn delete_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
) -> Result<TemplateCommandResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    app_data
        .config
        .lock()
        .await
        .delete_user_template(uid, &template_name);
    info!("删除模板: {}", template_name);

    Ok(TemplateCommandResponse {
        success: true,
        message: "模板删除成功".to_string(),
        template: None,
    })
}

#[tauri::command]
pub async fn update_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
    template: TemplateConfig,
) -> Result<TemplateCommandResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let updated = app_data
        .config
        .lock()
        .await
        .add_user_template(uid, &template_name, template);
    info!("更新模板: {}", template_name);

    Ok(TemplateCommandResponse {
        success: true,
        message: "模板更新成功".to_string(),
        template: Some(updated),
    })
}

#[tauri::command]
pub async fn add_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
    template: TemplateConfig,
) -> Result<TemplateCommandResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let added = app_data
        .config
        .lock()
        .await
        .add_user_template(uid, &template_name, template);
    info!("添加模板: {}", template_name);

    Ok(TemplateCommandResponse {
        success: true,
        message: "模板添加成功".to_string(),
        template: Some(added),
    })
}

#[tauri::command]
pub async fn rename_user_template(
    app: AppHandle,
    uid: u64,
    old_name: String,
    new_name: String,
) -> Result<RenameTemplateCommandResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let mut config = app_data.config.lock().await;
    config
        .rename_user_template(uid, &old_name, &new_name)
        .map_err(|e| format!("重命名模板失败: {e}"))?;

    let saved_order = config
        .config
        .get(&uid)
        .map(|user_config| user_config.template_order.clone())
        .unwrap_or_default();

    info!("重命名模板: uid={}, {} -> {}", uid, old_name, new_name);

    Ok(RenameTemplateCommandResponse {
        success: true,
        message: "模板重命名成功".to_string(),
        template_order: saved_order,
    })
}

#[tauri::command]
pub async fn save_template_order(
    app: AppHandle,
    uid: u64,
    template_order: Vec<String>,
) -> Result<TemplateOrderCommandResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let mut config = app_data.config.lock().await;
    config
        .save_template_order(uid, template_order)
        .map_err(|e| format!("保存模板顺序失败: {e}"))?;

    let saved_order = config
        .config
        .get(&uid)
        .map(|user_config| user_config.template_order.clone())
        .unwrap_or_default();

    info!("保存模板顺序: uid={}", uid);

    Ok(TemplateOrderCommandResponse {
        success: true,
        message: "模板顺序保存成功".to_string(),
        template_order: saved_order,
    })
}

#[tauri::command]
pub async fn save_user_order(
    app: AppHandle,
    user_order: Vec<u64>,
) -> Result<UserOrderCommandResponse, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let mut config = app_data.config.lock().await;
    config
        .save_user_order(user_order)
        .map_err(|e| format!("保存用户顺序失败: {e}"))?;

    let saved_order = config.user_order.clone();

    info!("保存用户顺序");

    Ok(UserOrderCommandResponse {
        success: true,
        message: "用户顺序保存成功".to_string(),
        user_order: saved_order,
    })
}
