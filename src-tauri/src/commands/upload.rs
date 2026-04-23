use std::sync::Arc;

use crate::{
    AppData,
    error::AppError,
    models::{TemplateConfig, UploadTask, VideoInfo},
};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use tracing::info;

/// 创建上传任务
#[tauri::command]
pub async fn create_upload_task(
    app: AppHandle,
    uid: u64,
    template: String,
    video: VideoInfo,
) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    let user = app_data.get_client(uid).await?.user;
    let config_copy = Arc::clone(&app_data.config);
    let clients_copy = Arc::clone(&app_data.clients);
    let upload_service = &app_data.upload_service;

    let created = upload_service
        .create_task(&user, &template, &video, config_copy, clients_copy)
        .await
        .map_err(AppError::Internal)?;

    Ok(created)
}

/// 开始上传
#[tauri::command]
pub async fn start_upload(app: AppHandle, task_id: String) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    Ok(upload_service
        .start_upload(&task_id)
        .await
        .map_err(AppError::Internal)?)
}

/// 暂停上传
#[tauri::command]
pub async fn pause_upload(app: AppHandle, task_id: String) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    Ok(upload_service
        .pause_upload(&task_id)
        .await
        .map_err(AppError::Internal)?)
}

/// 取消上传
#[tauri::command]
pub async fn cancel_upload(app: AppHandle, task_id: String) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    Ok(upload_service
        .cancel_upload(&task_id)
        .await
        .map_err(AppError::Internal)?)
}

/// 获取上传队列
#[tauri::command]
pub async fn get_upload_queue(app: AppHandle) -> Result<Vec<UploadTask>, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;
    Ok(upload_service
        .get_upload_queue()
        .await
        .map_err(AppError::Internal)?)
}

/// 重新上传失败的任务
#[tauri::command]
pub async fn retry_upload(app: AppHandle, task_id: String) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    Ok(upload_service
        .retry_upload(&task_id)
        .await
        .map_err(AppError::Internal)?)
}

#[tauri::command]
pub async fn submit(app: AppHandle, uid: u64, form: TemplateConfig) -> Result<Value, AppError> {
    let app_data = app.state::<AppData>();

    if form.aid.is_none() {
        // 将前端表单转换为B站API需要的格式
        let bilibili_form = form.into_bilibili_form();
        let studio = bilibili_form
            .try_into_studio()
            .map_err(AppError::Internal)?;

        #[cfg(debug_assertions)]
        {
            use tracing::debug;

            let json_content = serde_json::to_string_pretty(&studio).unwrap();
            debug!("转换后的B站提交表单: {uid}\n{}", json_content);
        }

        let proxy = app_data
            .config
            .lock()
            .await
            .config
            .get(&uid)
            .and_then(|c| c.proxy.clone());

        let bilibili = app_data.get_bilibili(uid).await?;
        match bilibili.submit_by_web(&studio, proxy.as_deref()).await {
            Ok(resp) => {
                info!("添加稿件成功：{resp}");
                Ok(resp
                    .data
                    .ok_or_else(|| AppError::Biliup("返回值错误".to_string()))?)
            }
            Err(e) => Err(AppError::Internal(anyhow::anyhow!("{}", e))),
        }
    } else {
        let bilibili_form = form.into_bilibili_form();
        let studio = bilibili_form
            .try_into_studio()
            .map_err(AppError::Internal)?;
        let bilibili = app_data.get_bilibili(uid).await?;
        match bilibili.edit_by_web(&studio).await {
            Ok(resp) => {
                info!("编辑稿件成功：{resp}");
                Ok(resp["data"].clone())
            }
            Err(e) => Err(AppError::Internal(anyhow::anyhow!("{}", e))),
        }
    }
}
