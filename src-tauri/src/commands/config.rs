use tauri::AppHandle;
use tauri::Manager;

use tracing::info;

use crate::{AppData, error::AppError, models::ConfigRoot};
use crate::{models::TemplateConfig, utils::get_config_json_path};

/// 加载配置文件
#[tauri::command]
pub async fn load_config(app: AppHandle) -> Result<ConfigRoot, AppError> {
    let app_data = app.state::<AppData>();

    Ok(app_data.config.lock().await.clone())
}

/// 保存配置文件
#[tauri::command]
pub async fn save_config(app: AppHandle) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();

    app_data
        .config
        .lock()
        .await
        .save_to_file(&get_config_json_path().map_err(AppError::Internal)?)
        .map_err(|e| AppError::Config(format!("保存配置失败: {e}")))?;
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
) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    info!("用户({uid})配置已保存");

    app_data
        .config
        .lock()
        .await
        .save_user_config(uid, line, proxy, limit, watermark, auto_edit)
        .map_err(|e| AppError::Config(format!("保存用户配置失败: {e}")))?;
    Ok(true)
}

#[tauri::command]
pub async fn save_global_config(
    app: AppHandle,
    max_curr: u32,
    auto_start: bool,
    auto_upload: bool,
    log_level: String,
) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();

    info!("全局配置已保存");

    app_data
        .config
        .lock()
        .await
        .save_global_config(max_curr, auto_start, auto_upload, log_level);

    app_data.upload_service.set_max_concurrent(max_curr).await;
    Ok(true)
}

#[tauri::command]
pub async fn delete_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();

    app_data
        .config
        .lock()
        .await
        .delete_user_template(uid, &template_name);
    info!("删除模板: {}", template_name);

    Ok(true)
}

#[tauri::command]
pub async fn update_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
    template: TemplateConfig,
) -> Result<TemplateConfig, AppError> {
    let app_data = app.state::<AppData>();

    let updated = app_data
        .config
        .lock()
        .await
        .add_user_template(uid, &template_name, template);
    info!("更新模板: {}", template_name);

    Ok(updated)
}

#[tauri::command]
pub async fn add_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
    template: TemplateConfig,
) -> Result<TemplateConfig, AppError> {
    let app_data = app.state::<AppData>();

    let added = app_data
        .config
        .lock()
        .await
        .add_user_template(uid, &template_name, template);
    info!("添加模板: {}", template_name);

    Ok(added)
}

#[tauri::command]
pub async fn rename_user_template(
    app: AppHandle,
    uid: u64,
    old_name: String,
    new_name: String,
) -> Result<Vec<String>, AppError> {
    let app_data = app.state::<AppData>();

    let mut config = app_data.config.lock().await;
    config
        .rename_user_template(uid, &old_name, &new_name)
        .map_err(|e| AppError::Config(format!("重命名模板失败: {e}")))?;

    info!("重命名模板: {} -> {}", old_name, new_name);

    Ok(config.get_user_template_order(uid).to_owned())
}

#[tauri::command]
pub async fn save_template_order(
    app: AppHandle,
    uid: u64,
    template_order: Vec<String>,
) -> Result<Vec<String>, AppError> {
    let app_data = app.state::<AppData>();

    let mut config = app_data.config.lock().await;
    config
        .save_template_order(uid, template_order)
        .map_err(|e| AppError::Config(format!("保存模板顺序失败: {e}")))?;

    let saved_order = config.get_user_template_order(uid).to_owned();

    config
        .save_to_file(&get_config_json_path().map_err(AppError::Internal)?)
        .map_err(|e| AppError::Config(format!("保存模板顺序到配置文件失败: {e}")))?;

    info!("保存模板顺序: uid={}", uid);

    Ok(saved_order)
}

#[tauri::command]
pub async fn save_user_order(app: AppHandle, user_order: Vec<u64>) -> Result<Vec<u64>, AppError> {
    let app_data = app.state::<AppData>();

    let mut config = app_data.config.lock().await;
    config.user_order = user_order.clone();

    info!("用户排序已保存");

    config
        .save_to_file(&get_config_json_path().map_err(AppError::Internal)?)
        .map_err(|e| AppError::Config(format!("保存用户顺序到配置文件失败: {e}")))?;

    Ok(user_order)
}
