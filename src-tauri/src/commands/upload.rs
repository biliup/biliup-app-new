use std::sync::Arc;

use crate::{
    AppData,
    models::{UploadForm, UploadTask, VideoInfo},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadProgress {
    pub task_id: String,
    pub progress: f64,
    pub status: String,
    pub message: String,
}

/// 创建上传任务
#[tauri::command]
pub async fn create_upload_task(app: AppHandle, uid: u64, video: VideoInfo) -> Result<(), String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let mut app_data = app_lock.lock().await;
    let user = app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .user
        .clone();
    let config_copy = Arc::clone(&app_data.config);
    let clients_copy = Arc::clone(&app_data.clients);
    let upload_service = &mut app_data.upload_service;

    upload_service
        .create_task(&user, &video, config_copy, clients_copy)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 开始上传
#[tauri::command]
pub async fn start_upload(app: AppHandle, task_id: String) -> Result<bool, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let mut app_data = app_lock.lock().await;
    let upload_service = &mut app_data.upload_service;

    upload_service
        .start_upload(&task_id)
        .await
        .map_err(|e| e.to_string())
}

/// 暂停上传
#[tauri::command]
pub async fn pause_upload(app: AppHandle, task_id: String) -> Result<bool, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let mut app_data = app_lock.lock().await;
    let upload_service = &mut app_data.upload_service;

    upload_service
        .pause_upload(&task_id)
        .await
        .map_err(|e| e.to_string())
}

/// 取消上传
#[tauri::command]
pub async fn cancel_upload(app: AppHandle, task_id: String) -> Result<bool, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let mut app_data = app_lock.lock().await;
    let upload_service = &mut app_data.upload_service;

    upload_service
        .cancel_upload(&task_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取上传队列
#[tauri::command]
pub async fn get_upload_queue(app: AppHandle) -> Result<Vec<UploadTask>, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let mut app_data = app_lock.lock().await;
    let upload_service = &mut app_data.upload_service;
    upload_service
        .get_upload_queue()
        .await
        .map_err(|e| e.to_string())
}

/// 获取上传进度
#[tauri::command]
pub async fn get_upload_progress(task_id: String) -> Result<UploadProgress, String> {
    // TODO: 获取指定任务的上传进度
    Ok(UploadProgress {
        task_id,
        progress: 0.0,
        status: "waiting".to_string(),
        message: "等待上传".to_string(),
    })
}

/// 重新上传失败的任务
#[tauri::command]
pub async fn retry_upload(app: AppHandle, task_id: String) -> Result<bool, String> {
        let app_lock = app.state::<Mutex<AppData>>();
    let mut app_data = app_lock.lock().await;
    let upload_service = &mut app_data.upload_service;

    upload_service
        .retry_upload(&task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn submit(
    app: AppHandle,
    uid: u64,
    form: UploadForm,
) -> Result<String, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    if form.aid.is_none() {
        // 将前端表单转换为B站API需要的格式
        let bilibili_form = form.into_bilibili_form();
        let studio = bilibili_form.try_into_studio().map_err(|e| e.to_string())?;

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

        match app_data
            .clients
            .lock()
            .await
            .get(&uid)
            .ok_or("用户未登录或不存在")?
            .bilibili
            .submit_by_app(&studio, proxy.as_deref())
            .await
        {
            Ok(resp) => Ok(serde_json::to_string_pretty(
                &resp.data.ok_or("返回值错误").map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?),
            Err(e) => Err(e.to_string()),
        }
    } else {
        let bilibili_form = form.into_bilibili_form();
        let studio = bilibili_form.try_into_studio().map_err(|e| e.to_string())?;
        match app_data
            .clients
            .lock()
            .await
            .get(&uid)
            .ok_or("用户未登录或不存在")?
            .bilibili
            .edit_by_web(&studio)
            .await
        {
            Ok(resp) => Ok(serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())?),
            Err(e) => Err(e.to_string()),
        }
    }
}
