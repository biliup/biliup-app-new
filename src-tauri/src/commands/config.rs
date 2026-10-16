use tauri::AppHandle;
use tauri::Manager;

use tracing::info;

use crate::{AppData, error::AppError, models::ConfigRoot};
use crate::{models::TemplateConfig, utils::get_config_json_path};

fn persist_update<T>(
    config: &mut ConfigRoot,
    update: impl FnOnce(&mut ConfigRoot) -> anyhow::Result<T>,
) -> Result<T, AppError> {
    let mut updated = config.clone();
    let result = update(&mut updated).map_err(AppError::Internal)?;
    updated
        .save_to_file(&get_config_json_path().map_err(AppError::Internal)?)
        .map_err(|e| AppError::Config(crate::tr!("config.save_failed", error = e)))?;
    *config = updated;
    Ok(result)
}

#[tauri::command]
pub async fn get_startup_error(app: AppHandle) -> Option<String> {
    app.state::<AppData>().startup_error.clone()
}

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
        .map_err(|e| AppError::Config(crate::tr!("config.save_failed", error = e)))?;
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
    info!("{}", crate::tr!("config.user_saved", uid = uid));

    persist_update(&mut *app_data.config.lock().await, |config| {
        config.save_user_config(uid, line, proxy, limit, watermark, auto_edit)?;
        Ok(())
    })?;
    Ok(true)
}

#[tauri::command]
pub async fn save_global_config(
    app: AppHandle,
    max_curr: u32,
    auto_start: bool,
    auto_upload: bool,
    log_level: String,
    language: Option<String>,
) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();

    let mut config = app_data.config.lock().await;
    persist_update(&mut config, |config| {
        let language = language.unwrap_or_else(|| config.language.clone());
        config.save_global_config(max_curr, auto_start, auto_upload, log_level, language);
        Ok(())
    })?;

    crate::i18n::set_language(&config.language);
    drop(config);
    info!("{}", crate::tr!("config.global_saved"));
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

    persist_update(&mut *app_data.config.lock().await, |config| {
        config.delete_user_template(uid, &template_name);
        Ok(())
    })?;
    info!("{}", crate::tr!("template.deleting", name = template_name));

    Ok(true)
}

#[tauri::command]
pub async fn sync_template_season(
    app: AppHandle,
    uid: u64,
    aids: Vec<u64>,
    season_id: u64,
    section_id: u64,
) -> Result<(), AppError> {
    let app_data = app.state::<AppData>();
    persist_update(&mut *app_data.config.lock().await, |config| {
        if let Some(user) = config.config.get_mut(&uid) {
            for template in user.templates.values_mut() {
                if template.aid.is_some_and(|aid| aids.contains(&aid)) {
                    template.season_id = Some(season_id);
                    template.section_id = Some(section_id);
                }
            }
        }
        Ok(())
    })
}

#[tauri::command]
pub async fn update_user_template(
    app: AppHandle,
    uid: u64,
    template_name: String,
    template: TemplateConfig,
) -> Result<TemplateConfig, AppError> {
    let app_data = app.state::<AppData>();

    let updated = persist_update(&mut *app_data.config.lock().await, |config| {
        anyhow::ensure!(
            config.config.contains_key(&uid),
            crate::i18n::text("config.user_missing")
        );
        Ok(config.add_user_template(uid, &template_name, template))
    })?;
    info!("{}", crate::tr!("template.updating", name = template_name));

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

    let added = persist_update(&mut *app_data.config.lock().await, |config| {
        anyhow::ensure!(
            config.config.contains_key(&uid),
            crate::i18n::text("config.user_missing")
        );
        Ok(config.add_user_template(uid, &template_name, template))
    })?;
    info!("{}", crate::tr!("template.adding", name = template_name));

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

    persist_update(&mut *app_data.config.lock().await, |config| {
        config.rename_user_template(uid, &old_name, &new_name)?;
        Ok(config.get_user_template_order(uid).to_owned())
    })
}

#[tauri::command]
pub async fn save_template_order(
    app: AppHandle,
    uid: u64,
    template_order: Vec<String>,
) -> Result<Vec<String>, AppError> {
    let app_data = app.state::<AppData>();

    persist_update(&mut *app_data.config.lock().await, |config| {
        config.save_template_order(uid, template_order)?;
        Ok(config.get_user_template_order(uid).to_owned())
    })
}

#[tauri::command]
pub async fn save_user_order(app: AppHandle, user_order: Vec<u64>) -> Result<Vec<u64>, AppError> {
    let app_data = app.state::<AppData>();

    persist_update(&mut *app_data.config.lock().await, |config| {
        config.save_user_order(user_order)?;
        Ok(config.user_order.clone())
    })
}
