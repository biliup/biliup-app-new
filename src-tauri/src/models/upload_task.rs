use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    MyClient,
    models::{ConfigRoot, User, VideoInfo},
    utils::get_file_size,
};
use debug_ignore::DebugIgnore;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct UploadTask {
    pub id: String,
    pub template: String,
    pub user: User,
    pub video: VideoInfo,
    pub status: TaskStatus,
    pub error_message: Option<String>,
    pub created_at: usize,
    pub started_at: Option<usize>,
    pub finished_at: Option<usize>,
    pub retry_count: u32,
    pub progress: f64,
    pub total_size: u64,
    pub total_transmit_bytes: u64,
    #[serde(skip)]
    pub last_activity: std::time::Instant,
    #[serde(skip)]
    pub transferring: bool,
    #[serde(skip)]
    pub config: Arc<Mutex<ConfigRoot>>,
    #[serde(skip)]
    pub clients: DebugIgnore<Arc<Mutex<HashMap<u64, MyClient>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Waiting,   // 未开始
    Pending,   // 等待中
    Running,   // 运行中
    Paused,    // 已暂停
    Completed, // 已完成
    Failed,    // 失败
    Cancelled, // 已取消
}

impl UploadTask {
    pub fn new(
        user: &User,
        template: &str,
        video: &VideoInfo,
        config: Arc<Mutex<ConfigRoot>>,
        clients: Arc<Mutex<HashMap<u64, MyClient>>>,
    ) -> Self {
        Self {
            id: video.id.clone(),
            user: user.clone(),
            video: video.clone(),
            template: template.to_string(),
            status: TaskStatus::Waiting,
            error_message: None,
            created_at: chrono::Utc::now().timestamp_millis() as usize,
            started_at: None,
            finished_at: None,
            retry_count: 0,
            progress: 0.0,
            total_size: get_file_size(Path::new(&video.path)).unwrap_or(0),
            total_transmit_bytes: 0,
            last_activity: std::time::Instant::now(),
            transferring: false,
            config,
            clients: DebugIgnore(clients),
        }
    }

    pub fn title(&self) -> String {
        format!("{} - {}", self.user.username, self.video.title)
    }

    pub fn start(&mut self) {
        self.last_activity = std::time::Instant::now();
        self.transferring = false;
        self.status = TaskStatus::Running;
        self.total_transmit_bytes = 0;
        self.started_at = Some(chrono::Utc::now().timestamp_millis() as usize);
    }

    pub fn resume(&mut self) {
        self.last_activity = std::time::Instant::now();
        self.status = TaskStatus::Running;
    }

    pub fn pending(&mut self) {
        self.status = TaskStatus::Pending;
    }

    pub fn pause(&mut self) {
        self.status = TaskStatus::Paused;
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.progress = 100.0;
        self.finished_at = Some(chrono::Utc::now().timestamp_millis() as usize);
    }

    pub fn fail(&mut self, error_message: String) {
        self.status = TaskStatus::Failed;
        self.error_message = Some(error_message);
        self.finished_at = Some(chrono::Utc::now().timestamp_millis() as usize);
    }

    pub fn cancel(&mut self) {
        self.total_transmit_bytes = 0;
        self.progress = 0.0;
        self.status = TaskStatus::Cancelled;
        self.finished_at = Some(chrono::Utc::now().timestamp_millis() as usize);
    }

    pub fn retry(&mut self) {
        self.status = TaskStatus::Pending;
        self.error_message = None;
        self.started_at = None;
        self.finished_at = None;
        self.retry_count += 1;
    }

    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    pub fn update_progress(&mut self, progress: f64) {
        self.last_activity = std::time::Instant::now();
        self.progress = progress.clamp(0.0, 100.0);
    }

    pub fn update_total_transmit_bytes(&mut self, total_transmit_bytes: u64) {
        self.last_activity = std::time::Instant::now();
        self.total_transmit_bytes += total_transmit_bytes;
    }

    pub fn has_stalled(&self) -> bool {
        // 上传库未暴露分片响应状态，等待响应期间也可能没有字节进展。
        // 传输阶段覆盖分片请求的 240 秒超时，避免中止正常的慢请求。
        let timeout = if self.transferring { 300 } else { 180 };
        self.is_running() && self.last_activity.elapsed().as_secs() >= timeout
    }

    pub fn is_waiting(&self) -> bool {
        matches!(self.status, TaskStatus::Waiting)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.status, TaskStatus::Pending)
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.status, TaskStatus::Paused)
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self.status, TaskStatus::Cancelled)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, TaskStatus::Failed)
    }

    pub fn config(&self) -> &Arc<Mutex<ConfigRoot>> {
        &self.config
    }

    pub fn clients(&self) -> &Arc<Mutex<HashMap<u64, MyClient>>> {
        &self.clients.0
    }
}
