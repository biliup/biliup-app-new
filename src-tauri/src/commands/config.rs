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
) -> Result<bool, String> {
    let data = app.state::<Mutex<AppData>>();

    data.lock()
        .await
        .config
        .lock()
        .await
        .save_user_config(uid, line, proxy, limit, watermark)
        .map_err(|e| format!("保存用户配置失败: {e}"))?;
    Ok(true)
}

#[tauri::command]
pub async fn save_global_config(
    app: AppHandle,
    max_curr: u32,
    auto_start: bool,
    auto_upload: bool,
) -> Result<bool, String> {
    let data = app.state::<Mutex<AppData>>();

    data.lock()
        .await
        .config
        .lock()
        .await
        .save_global_config(max_curr, auto_start, auto_upload);

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
    info!("Delted template: {}", template_name);

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
    info!("Template updated: {}", template_name);

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
    info!("Template added: {}", template_name);

    Ok(TemplateCommandResponse {
        success: true,
        message: "模板添加成功".to_string(),
        template: Some(added),
    })
}
