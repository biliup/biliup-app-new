use std::{collections::HashMap, path::PathBuf, pin::Pin, sync::Arc, task::Poll};

use super::upload_risk::{UploadRiskService, verified_line};
use crate::{
    MyClient,
    models::{ConfigRoot, TaskStatus, UploadTask, User, VideoInfo},
};
use anyhow::Result;
use biliup::{
    client::StatelessClient,
    uploader::{VideoFile, line},
};
use bytes::{Buf, Bytes};
use futures::Stream;
use futures::StreamExt;
use indexmap::IndexMap;
use reqwest::Body;
use tokio::{
    select,
    sync::{Mutex, mpsc},
    task,
};
use tracing::{debug, error, info, warn};

// define a macro :  task_title!(task_mutex)
macro_rules! task_title {
    ($task_mutex:expr) => {
        $task_mutex.lock().await.title()
    };
}

pub struct UploadService {
    pub risk: Arc<UploadRiskService>,
    upload_queue: Arc<Mutex<IndexMap<String, Arc<Mutex<UploadTask>>>>>,
    upload_handle: Arc<Mutex<HashMap<String, task::JoinHandle<()>>>>,
    _upload_backgnd: task::JoinHandle<()>,
    max_running: Arc<Mutex<u32>>,
    stop_tx: mpsc::Sender<()>,
}

impl UploadService {
    pub async fn set_max_concurrent(&self, max_curr: u32) {
        let mut curr = self.max_running.lock().await;
        let old = *curr;
        *curr = max_curr;

        debug!(
            "{}",
            crate::tr!(
                "upload.concurrency_changed",
                previous = old,
                current = max_curr
            )
        );
    }

    pub fn new(max_curr: u32) -> Self {
        let max_running = Arc::new(Mutex::new(max_curr));
        let max_running_clone = Arc::clone(&max_running);

        let upload_queue = Arc::new(Mutex::new(IndexMap::new()));
        let upload_queue_clone = Arc::clone(&upload_queue);

        let upload_handle = Arc::new(Mutex::new(HashMap::new()));
        let upload_handle_clone = Arc::clone(&upload_handle);

        let (stop_tx, stop_rx) = mpsc::channel(1);
        let risk = Arc::new(UploadRiskService::default());
        let risk_clone = Arc::clone(&risk);

        Self {
            risk,
            upload_queue,
            upload_handle,
            _upload_backgnd: task::spawn(async move {
                upload_background(
                    upload_queue_clone,
                    upload_handle_clone,
                    max_running_clone,
                    risk_clone,
                    stop_rx,
                )
                .await;
            }),
            max_running,
            stop_tx,
        }
    }

    /// 创建上传任务
    pub async fn create_task(
        &self,
        user: &User,
        template: &str,
        video: &VideoInfo,
        config_copy: Arc<Mutex<ConfigRoot>>,
        clients: Arc<Mutex<HashMap<u64, MyClient>>>,
    ) -> Result<bool> {
        let task = UploadTask::new(user, template, video, config_copy, clients);
        if self.upload_queue.lock().await.contains_key(&task.id) {
            warn!(
                "{}",
                crate::tr!("upload.task_exists", task_id = task.title())
            );
            return Ok(false); // 任务已存在
        }
        // 将任务添加到上传队列
        let title = task.title().clone();
        self.upload_queue
            .lock()
            .await
            .insert(task.id.clone(), Arc::new(Mutex::new(task)));
        info!(
            "{}",
            crate::tr!("upload.task_created", title = format!("{:?}", title))
        );
        Ok(true)
    }

    pub async fn get_upload_queue(&self) -> Result<Vec<UploadTask>> {
        let task_mutexes: Vec<_> = self.upload_queue.lock().await.values().cloned().collect();
        let mut tasks = Vec::new();
        for task_mutex in task_mutexes {
            tasks.push(task_mutex.lock().await.clone());
        }
        Ok(tasks)
    }

    async fn stop_handle(&self, task_id: &str) {
        let handle = self.upload_handle.lock().await.remove(task_id);
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
    }

    pub async fn start_upload(&self, task_id: &str) -> Result<bool> {
        let queue = self.upload_queue.lock().await;
        let task_mutex = queue.get(task_id).ok_or_else(|| {
            anyhow::anyhow!("{}", crate::tr!("upload.task_missing", task_id = task_id))
        })?;
        let (failed, uid) = {
            let task = task_mutex.lock().await;
            (task.is_failed(), task.user.uid)
        };
        if failed {
            self.stop_handle(task_id).await;
        }
        let risk_cleared = self.risk.prepare_retry(uid);
        let mut task = task_mutex.lock().await;
        if task.is_failed() {
            task.cancel();
            task.retry();
            return Ok(true);
        }
        if task.is_paused() || task.is_waiting() || (risk_cleared && task.is_pending()) {
            task.pending();
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn pause_upload(&self, task_id: &str) -> Result<bool> {
        let queue = self.upload_queue.lock().await;
        let task_mutex = queue.get(task_id).ok_or_else(|| {
            anyhow::anyhow!("{}", crate::tr!("upload.task_missing", task_id = task_id))
        })?;
        let mut task = task_mutex.lock().await;
        if !(task.is_running()
            || task.is_pending()
            || matches!(task.status, TaskStatus::AwaitingVerification))
        {
            return Ok(false);
        }
        task.pause();
        Ok(true)
    }

    pub async fn cancel_upload(&self, task_id: &str) -> Result<bool> {
        let mut queue = self.upload_queue.lock().await;
        let task_mutex = queue.shift_remove(task_id).ok_or_else(|| {
            anyhow::anyhow!("{}", crate::tr!("upload.task_missing", task_id = task_id))
        })?;
        self.stop_handle(task_id).await;
        task_mutex.lock().await.cancel();
        Ok(true)
    }

    pub async fn retry_upload(&self, task_id: &str) -> Result<bool> {
        let queue = self.upload_queue.lock().await;
        let task_mutex = queue.get(task_id).ok_or_else(|| {
            anyhow::anyhow!("{}", crate::tr!("upload.task_missing", task_id = task_id))
        })?;
        let uid = {
            let task = task_mutex.lock().await;
            if matches!(
                task.status,
                TaskStatus::Completed | TaskStatus::AwaitingVerification
            ) {
                return Ok(false);
            }
            task.user.uid
        };
        self.stop_handle(task_id).await;
        self.risk.prepare_retry(uid);
        let mut task = task_mutex.lock().await;
        task.cancel();
        task.retry();
        Ok(true)
    }
}

impl Drop for UploadService {
    fn drop(&mut self) {
        let _ = self.stop_tx.try_send(());
        if let Ok(mut handles) = self.upload_handle.try_lock() {
            for (_, handle) in handles.drain() {
                handle.abort();
            }
        }
        info!("{}", crate::tr!("upload.service_stopped"));
    }
}

async fn upload_background(
    queue: Arc<Mutex<IndexMap<String, Arc<Mutex<UploadTask>>>>>,
    handle: Arc<Mutex<HashMap<String, task::JoinHandle<()>>>>,
    max_running: Arc<Mutex<u32>>,
    risk: Arc<UploadRiskService>,
    mut stop_rx: mpsc::Receiver<()>,
) {
    let mut one_sec = tokio::time::interval(tokio::time::Duration::from_secs(1));
    loop {
        let queue_clone = Arc::clone(&queue);
        let handle_clone = Arc::clone(&handle);
        let max_running_clone = Arc::clone(&max_running);
        select! {
            _ = stop_rx.recv() => {
                info!("{}", crate::tr!("upload.service_stopped"));
                return;
            }
            _ = one_sec.tick() => {
                upload_background_interval(queue_clone, handle_clone, max_running_clone, Arc::clone(&risk)).await;
            }
        }
    }
}

async fn upload_background_interval(
    queue: Arc<Mutex<IndexMap<String, Arc<Mutex<UploadTask>>>>>,
    handle: Arc<Mutex<HashMap<String, task::JoinHandle<()>>>>,
    max_running: Arc<Mutex<u32>>,
    risk: Arc<UploadRiskService>,
) {
    let queue = queue.lock().await;
    handle
        .lock()
        .await
        .retain(|_, worker| !worker.is_finished());
    for task_mutex in queue.values() {
        let (stalled, task_id) = {
            let task = task_mutex.lock().await;
            (task.has_stalled(), task.id.clone())
        };
        if !stalled {
            continue;
        }
        let worker = handle.lock().await.remove(&task_id);
        if let Some(worker) = worker {
            worker.abort();
            let _ = worker.await;
        }
        let mut task = task_mutex.lock().await;
        if task.retry_count < 3 {
            warn!(
                "{}",
                crate::tr!("upload.stalled_retry", task_id = task.title())
            );
            task.cancel();
            task.retry();
        } else {
            task.fail(crate::i18n::text("upload.retries_exhausted").into());
        }
    }
    let mut current_running = 0;
    for task_mutex in queue.values() {
        if task_mutex.lock().await.is_running() {
            current_running += 1;
        }
    }
    let mut remain = max_running.lock().await.saturating_sub(current_running);
    for task_mutex in queue.values() {
        if remain == 0 {
            break;
        }
        let mut task = task_mutex.lock().await;
        if !task.is_pending() || risk.is_blocked(task.user.uid) {
            continue;
        }
        let mut handles = handle.lock().await;
        if handles.contains_key(&task.id) {
            task.resume();
        } else {
            task.start();
            let task_mutex_clone = Arc::clone(task_mutex);
            let risk_clone = Arc::clone(&risk);
            handles.insert(
                task.id.clone(),
                task::spawn(async move {
                    if let Err(e) = upload_impl(Arc::clone(&task_mutex_clone), risk_clone).await {
                        error!("{}", crate::tr!("upload.task_failed", error = e));
                        task_mutex_clone.lock().await.fail(e.to_string());
                    }
                }),
            );
        }
        remain -= 1;
    }
}

/// 上传进度条结构体
#[derive(Clone)]
pub struct ChunkedBuffer {
    bytes: Bytes,
    tx: mpsc::UnboundedSender<u64>,
}

impl ChunkedBuffer {
    pub fn new(bytes: Bytes, tx: mpsc::UnboundedSender<u64>) -> Self {
        Self { bytes, tx }
    }

    /// 发送进度并返回分片
    pub fn progress(&mut self) -> Result<Option<Bytes>> {
        let pb = &self.tx;
        let content_bytes = &mut self.bytes;
        let n = content_bytes.remaining();
        let pc = 1 << 20;
        if n == 0 {
            Ok(None)
        } else if n < pc {
            pb.send(n as u64)?;
            Ok(Some(content_bytes.copy_to_bytes(n)))
        } else {
            pb.send(pc as u64)?;
            Ok(Some(content_bytes.copy_to_bytes(pc)))
        }
    }
}

impl Stream for ChunkedBuffer {
    type Item = Result<Bytes>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.progress()? {
            None => Poll::Ready(None),
            Some(s) => Poll::Ready(Some(Ok(s))),
        }
    }
}

impl From<ChunkedBuffer> for Body {
    fn from(async_stream: ChunkedBuffer) -> Self {
        Body::wrap_stream(async_stream)
    }
}

async fn upload_impl(
    task_mutex: Arc<Mutex<UploadTask>>,
    risk: Arc<UploadRiskService>,
) -> Result<()> {
    info!(
        "{}",
        crate::tr!("upload.task_started", task_id = task_title!(task_mutex))
    );
    let uid = task_mutex.lock().await.user.uid;

    let (line, proxy, limit) = {
        let lock = task_mutex.lock().await;
        let config_root = lock.config().lock().await;
        let config = config_root
            .config
            .get(&uid)
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("error.user_not_found")))?;

        (config.line.clone(), config.proxy.clone(), config.limit)
    };

    anyhow::ensure!(
        (1..=12).contains(&limit),
        crate::i18n::text("upload.invalid_concurrency")
    );

    let client = &task_mutex
        .lock()
        .await
        .clients()
        .lock()
        .await
        .get(&uid)
        .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("error.user_not_found")))?
        .bilibili
        .clone();

    let retry_count = task_mutex.lock().await.retry_count();
    wait_for_account_verification(&task_mutex, &risk).await?;

    // 定义可用的上传线路列表
    let available_lines = [
        "auto", "bda2", "bldsa", "tx", "txa", "alia", "cnbd", "anbd", "atbd", "cnbldsa", "andsa",
        "atdsa", "cntx", "antx", "attx", "estx", "akbd",
    ];

    // 根据重试次数自动选择线路
    let selected_line = if let Some(config_line) = &line {
        // 如果用户配置了特定线路，第一次使用配置线路，之后重试时自动切换
        if retry_count == 0 {
            config_line.clone()
        } else {
            // 根据重试次数选择不同的线路，避免重复使用失败的线路
            let line_index = (retry_count as usize) % available_lines.len();
            available_lines[line_index].to_string()
        }
    } else {
        // 如果没有配置线路，根据重试次数自动选择
        let line_index = (retry_count as usize) % available_lines.len();
        available_lines[line_index].to_string()
    };

    info!(
        "{}",
        crate::tr!(
            "upload.route_selected",
            route = selected_line,
            retry_count = retry_count
        )
    );

    let probe = match selected_line.as_str() {
        "bda2" => line::bda2(),
        "bldsa" => line::bldsa(),
        "tx" => line::tx(),
        "txa" => line::txa(),
        "alia" => line::alia(),
        "cnbd" => line::cnbd(),
        "anbd" => line::anbd(),
        "atbd" => line::atbd(),
        "cnbldsa" => line::cnbldsa(),
        "andsa" => line::andsa(),
        "atdsa" => line::atdsa(),
        "cntx" => line::cntx(),
        "antx" => line::antx(),
        "attx" => line::attx(),
        "estx" => line::estx(),
        "akbd" => line::akbd(),
        _ => line::Probe::probe(&client.client).await?,
    };

    let filepath = PathBuf::from(&task_mutex.lock().await.video.path);
    let video_file = VideoFile::new(&filepath)?;
    let total_size = task_mutex.lock().await.total_size;
    wait_for_account_verification(&task_mutex, &risk).await?;
    let upload_line = risk
        .recent_token(uid)
        .map(|token| verified_line(&probe, &token))
        .transpose()?
        .unwrap_or_else(|| probe.clone());
    let parcel = match upload_line.pre_upload(client, video_file).await {
        Ok(parcel) => parcel,
        Err(biliup::error::Kind::RateLimit { .. }) => {
            {
                let mut task = task_mutex.lock().await;
                if !task.is_paused() {
                    task.status = TaskStatus::AwaitingVerification;
                }
            }
            let token = risk
                .verify(uid, client.clone(), probe.clone(), filepath.clone(), proxy)
                .await?;
            resume_after_verification(&task_mutex).await?;
            let result = verified_line(&probe, &token)?
                .pre_upload(client, VideoFile::new(&filepath)?)
                .await;
            if matches!(&result, Err(biliup::error::Kind::RateLimit { .. })) {
                risk.hold(uid, "安全验证后仍被限流，后续上传已暂停；请稍后手动重试");
            }
            result?
        }
        Err(error) => return Err(error.into()),
    };
    {
        let mut task = task_mutex.lock().await;
        task.transferring = true;
        task.last_activity = std::time::Instant::now();
    }
    let (file_read_tx, mut file_read_rx) = mpsc::unbounded_channel();
    let (net_send_tx, mut net_send_rx) = mpsc::unbounded_channel();
    let mut uploaded = 0;
    let mut video_fut = Box::pin(
        parcel.upload(StatelessClient::default(), limit as usize, |vs| {
            vs.map(|chunk| {
                // parcel.upload 将文件分成各个chunk
                // 用ChunkedBuffer 将每个chunk再拆分成1MB一小段
                // 用ChunkedBuffer 内的net_send_tx来计算实际网络速度
                // 用tx来计算总的上传进度，防止重传的数据进入net_send_tx导致进度条计算出错
                let chunk = chunk?;
                let len = chunk.len();
                uploaded += len;
                file_read_tx.send(uploaded).unwrap();
                let chunked_buffer = ChunkedBuffer::new(chunk, net_send_tx.clone());
                Ok((chunked_buffer, len))
            })
        }),
    );

    let mut paused = false;
    loop {
        wait_for_account_verification(&task_mutex, &risk).await?;
        while {
            let task = task_mutex.lock().await;
            task.is_paused() || task.is_pending()
        } {
            paused = true;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        if paused {
            paused = false;
            if task_mutex.lock().await.is_cancelled() {
                info!(
                    "{}",
                    crate::tr!("upload.task_canceled", task_id = task_title!(task_mutex))
                );
                return Ok(());
            }
            info!(
                "{}",
                crate::tr!("upload.task_resumed", task_id = task_title!(task_mutex))
            );
        }

        select! {
            Some(total_transmit) = net_send_rx.recv() => {
                // 处理分片
                task_mutex.lock().await.update_total_transmit_bytes(total_transmit);
            }
            Some(process_so_far) = file_read_rx.recv() => {
                // 处理上传进度
                task_mutex.lock().await.update_progress(process_so_far as f64/total_size as f64 * 100.0);
            }
            result = &mut video_fut => {
                match result {
                    Ok(return_video) => {
                        // 处理视频上传完成
                        debug!("{}", crate::tr!("upload.video_completed", video = format!("{:?}", return_video)));
                        // rewrite the stored id & name
                        task_mutex.lock().await.video.filename = return_video.filename.clone();
                        task_mutex.lock().await.video.path.clear();
                        info!("{}", crate::tr!("upload.task_completed", title = task_title!(task_mutex), filename = return_video.filename));
                        task_mutex.lock().await.complete();

                        return Ok(())
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("{}", crate::tr!("upload.task_failed", error = e)));
                    }
                }
            }
        }
    }
}

async fn wait_for_account_verification(
    task: &Arc<Mutex<UploadTask>>,
    risk: &UploadRiskService,
) -> Result<()> {
    let uid = task.lock().await.user.uid;
    if let Some(receiver) = risk.subscribe(uid) {
        {
            let mut task = task.lock().await;
            if !task.is_paused() {
                task.status = TaskStatus::AwaitingVerification;
            }
        }
        UploadRiskService::wait(receiver).await?;
        resume_after_verification(task).await?;
    }
    Ok(())
}

async fn resume_after_verification(task: &Arc<Mutex<UploadTask>>) -> Result<()> {
    {
        let mut task = task.lock().await;
        if matches!(task.status, TaskStatus::AwaitingVerification) {
            task.pending();
        }
    }
    loop {
        {
            let task = task.lock().await;
            if task.is_running() {
                return Ok(());
            }
            if task.is_cancelled() {
                return Err(anyhow::anyhow!("上传任务已取消"));
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str) -> UploadTask {
        let video: VideoInfo = serde_json::from_value(serde_json::json!({"id": id})).unwrap();
        UploadTask::new(
            &User::new(1, "测试账号".into(), String::new(), false),
            "测试模板",
            &video,
            Arc::new(Mutex::new(ConfigRoot::default())),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    #[tokio::test]
    async fn verification_blocks_only_its_account_and_does_not_timeout() {
        let risk = Arc::new(UploadRiskService::default());
        risk.hold(1, "等待人工验证");
        let mut waiting = task("verification");
        waiting.status = TaskStatus::AwaitingVerification;
        waiting.last_activity -= std::time::Duration::from_secs(1000);
        let mut blocked = task("blocked");
        blocked.pending();
        let mut other = task("other-account");
        other.user.uid = 2;
        other.pending();
        let waiting = Arc::new(Mutex::new(waiting));
        let blocked = Arc::new(Mutex::new(blocked));
        let other = Arc::new(Mutex::new(other));
        let queue = Arc::new(Mutex::new(IndexMap::from([
            ("verification".into(), Arc::clone(&waiting)),
            ("blocked".into(), Arc::clone(&blocked)),
            ("other-account".into(), Arc::clone(&other)),
        ])));
        let handles = Arc::new(Mutex::new(HashMap::from([
            ("verification".into(), tokio::spawn(std::future::pending())),
            ("blocked".into(), tokio::spawn(std::future::pending())),
            ("other-account".into(), tokio::spawn(std::future::pending())),
        ])));
        upload_background_interval(queue, Arc::clone(&handles), Arc::new(Mutex::new(1)), risk)
            .await;
        assert!(matches!(
            waiting.lock().await.status,
            TaskStatus::AwaitingVerification
        ));
        assert_eq!(waiting.lock().await.retry_count, 0);
        assert!(blocked.lock().await.is_pending());
        assert!(other.lock().await.is_running());
        for (_, handle) in handles.lock().await.drain() {
            handle.abort();
        }
    }

    #[tokio::test]
    async fn verification_does_not_resume_a_manually_paused_task() {
        let mut paused = task("manual-pause");
        paused.pause();
        let paused = Arc::new(Mutex::new(paused));
        let worker_task = Arc::clone(&paused);
        let worker = tokio::spawn(async move { resume_after_verification(&worker_task).await });
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        assert!(paused.lock().await.is_paused());
        assert!(!worker.is_finished());
        paused.lock().await.resume();
        tokio::time::timeout(tokio::time::Duration::from_secs(1), worker)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn restart_failed_task_without_querying_queue() {
        let service = UploadService::new(0);
        let mut failed = task("failed");
        failed.fail("测试错误".into());
        service
            .upload_queue
            .lock()
            .await
            .insert("failed".into(), Arc::new(Mutex::new(failed)));
        service
            .upload_handle
            .lock()
            .await
            .insert("failed".into(), tokio::spawn(std::future::pending()));
        assert!(service.start_upload("failed").await.unwrap());
        assert!(!service.upload_handle.lock().await.contains_key("failed"));
        let queue = service.get_upload_queue().await.unwrap();
        assert!(queue[0].is_pending());
        assert!(queue[0].error_message.is_none());
        assert_eq!(queue[0].retry_count, 1);
    }

    #[tokio::test]
    async fn paused_worker_releases_slot_and_resume_waits_for_scheduler() {
        let mut paused = task("paused");
        paused.pause();
        let mut pending = task("pending");
        pending.pending();
        let paused = Arc::new(Mutex::new(paused));
        let pending = Arc::new(Mutex::new(pending));
        let queue = Arc::new(Mutex::new(IndexMap::from([
            ("paused".into(), Arc::clone(&paused)),
            ("pending".into(), Arc::clone(&pending)),
        ])));
        let handles = Arc::new(Mutex::new(HashMap::from([
            ("paused".into(), tokio::spawn(std::future::pending())),
            ("pending".into(), tokio::spawn(std::future::pending())),
        ])));
        let maximum = Arc::new(Mutex::new(1));
        upload_background_interval(
            Arc::clone(&queue),
            Arc::clone(&handles),
            Arc::clone(&maximum),
            Arc::new(UploadRiskService::default()),
        )
        .await;
        assert!(pending.lock().await.is_running());
        paused.lock().await.pending();
        upload_background_interval(
            queue,
            Arc::clone(&handles),
            maximum,
            Arc::new(UploadRiskService::default()),
        )
        .await;
        assert!(paused.lock().await.is_pending());
        for (_, handle) in handles.lock().await.drain() {
            handle.abort();
        }
    }
    #[tokio::test]
    async fn retry_mid_upload_stall_without_frontend_polling() {
        let mut stalled = task("stalled");
        stalled.start();
        stalled.transferring = true;
        stalled.total_size = 100;
        stalled.total_transmit_bytes = 1;
        stalled.last_activity -= std::time::Duration::from_secs(301);
        let stalled = Arc::new(Mutex::new(stalled));
        let queue = Arc::new(Mutex::new(IndexMap::from([(
            "stalled".into(),
            Arc::clone(&stalled),
        )])));
        let handles = Arc::new(Mutex::new(HashMap::from([(
            "stalled".into(),
            tokio::spawn(std::future::pending()),
        )])));
        upload_background_interval(
            queue,
            handles,
            Arc::new(Mutex::new(0)),
            Arc::new(UploadRiskService::default()),
        )
        .await;
        let task = stalled.lock().await;
        assert!(task.is_pending());
        assert_eq!(task.retry_count, 1);
    }

    #[test]
    fn preparation_merge_and_pause_do_not_use_transfer_timeout() {
        let mut task = task("phases");
        task.start();
        task.last_activity -= std::time::Duration::from_secs(31);
        assert!(!task.has_stalled());
        task.transferring = true;
        task.last_activity -= std::time::Duration::from_secs(268);
        assert!(!task.has_stalled());
        task.total_size = 100;
        task.total_transmit_bytes = 100;
        task.progress = 100.0;
        assert!(!task.has_stalled());
        task.total_transmit_bytes = 1;
        assert!(!task.has_stalled());
        task.last_activity -= std::time::Duration::from_secs(2);
        assert!(task.has_stalled());
        task.pause();
        assert!(!task.has_stalled());
        task.resume();
        assert!(!task.has_stalled());
    }
}
