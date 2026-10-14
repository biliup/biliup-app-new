use std::sync::Arc;

use crate::{
    AppData,
    error::AppError,
    models::{TemplateConfig, UploadTask, VideoInfo},
};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};

use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchTaskActionResult {
    pub success_count: u32,
    pub noop_task_ids: Vec<String>,
    pub failed_task_ids: Vec<String>,
}

/// 创建上传任务
#[tauri::command]
pub async fn create_upload_task(
    app: AppHandle,
    uid: u64,
    template: String,
    videos: Vec<VideoInfo>,
) -> Result<u32, AppError> {
    let app_data = app.state::<AppData>();
    let user = app_data.get_client(uid).await?.user;
    let config_copy = Arc::clone(&app_data.config);
    let clients_copy = Arc::clone(&app_data.clients);
    let upload_service = &app_data.upload_service;

    let mut created_count = 0u32;
    for video in videos {
        let created = upload_service
            .create_task(
                &user,
                &template,
                &video,
                Arc::clone(&config_copy),
                Arc::clone(&clients_copy),
            )
            .await
            .map_err(AppError::Internal)?;

        if created {
            created_count += 1;
        }
    }

    Ok(created_count)
}

/// 开始上传
#[tauri::command]
pub async fn start_upload(
    app: AppHandle,
    task_ids: Vec<String>,
) -> Result<BatchTaskActionResult, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    let mut started_count = 0u32;
    let mut noop_task_ids = Vec::new();
    let mut failed_task_ids = Vec::new();
    for task_id in task_ids {
        match upload_service.start_upload(&task_id).await {
            Ok(true) => started_count += 1,
            Ok(false) => noop_task_ids.push(task_id),
            Err(error) => {
                warn!(
                    "{}",
                    crate::tr!("upload.start_failed", task_id = task_id, error = error)
                );
                failed_task_ids.push(task_id);
            }
        }
    }

    Ok(BatchTaskActionResult {
        success_count: started_count,
        noop_task_ids,
        failed_task_ids,
    })
}

/// 暂停上传
#[tauri::command]
pub async fn pause_upload(
    app: AppHandle,
    task_ids: Vec<String>,
) -> Result<BatchTaskActionResult, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    let mut paused_count = 0u32;
    let mut noop_task_ids = Vec::new();
    let mut failed_task_ids = Vec::new();
    for task_id in task_ids {
        match upload_service.pause_upload(&task_id).await {
            Ok(true) => paused_count += 1,
            Ok(false) => noop_task_ids.push(task_id),
            Err(error) => {
                warn!(
                    "{}",
                    crate::tr!("upload.pause_failed", task_id = task_id, error = error)
                );
                failed_task_ids.push(task_id);
            }
        }
    }

    Ok(BatchTaskActionResult {
        success_count: paused_count,
        noop_task_ids,
        failed_task_ids,
    })
}

/// 取消上传
#[tauri::command]
pub async fn cancel_upload(
    app: AppHandle,
    task_ids: Vec<String>,
) -> Result<BatchTaskActionResult, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    let mut cancelled_count = 0u32;
    let mut noop_task_ids = Vec::new();
    let mut failed_task_ids = Vec::new();
    for task_id in task_ids {
        match upload_service.cancel_upload(&task_id).await {
            Ok(true) => cancelled_count += 1,
            Ok(false) => noop_task_ids.push(task_id),
            Err(error) => {
                warn!(
                    "{}",
                    crate::tr!("upload.cancel_failed", task_id = task_id, error = error)
                );
                failed_task_ids.push(task_id);
            }
        }
    }

    Ok(BatchTaskActionResult {
        success_count: cancelled_count,
        noop_task_ids,
        failed_task_ids,
    })
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
pub async fn retry_upload(
    app: AppHandle,
    task_ids: Vec<String>,
) -> Result<BatchTaskActionResult, AppError> {
    let app_data = app.state::<AppData>();
    let upload_service = &app_data.upload_service;

    let mut retried_count = 0u32;
    let mut noop_task_ids = Vec::new();
    let mut failed_task_ids = Vec::new();
    for task_id in task_ids {
        match upload_service.retry_upload(&task_id).await {
            Ok(true) => retried_count += 1,
            Ok(false) => noop_task_ids.push(task_id),
            Err(error) => {
                warn!(
                    "{}",
                    crate::tr!("upload.retry_failed", task_id = task_id, error = error)
                );
                failed_task_ids.push(task_id);
            }
        }
    }

    Ok(BatchTaskActionResult {
        success_count: retried_count,
        noop_task_ids,
        failed_task_ids,
    })
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
            debug!(
                "{}",
                crate::tr!("submission.form_converted", uid = uid, form = json_content)
            );
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
                info!("{}", crate::tr!("submission.created", response = resp));
                Ok(resp.data.ok_or_else(|| {
                    AppError::Biliup(crate::i18n::text("submission.invalid_response").to_string())
                })?)
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
                info!("{}", crate::tr!("submission.edited", response = resp));
                Ok(resp["data"].clone())
            }
            Err(e) => Err(AppError::Internal(anyhow::anyhow!("{}", e))),
        }
    }
}
