use std::{collections::HashMap, path::PathBuf, pin::Pin, sync::Arc, task::Poll};

use crate::{
    MyClient,
    models::{ConfigRoot, UploadTask, User, VideoInfo},
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
use tauri::async_runtime::block_on;
use tokio::{
    select,
    sync::{Mutex, mpsc},
    task,
};
use tracing::{debug, info, warn};

// define a macro :  task_title!(task_mutex)
macro_rules! task_title {
    ($task_mutex:expr) => {
        $task_mutex.lock().await.title()
    };
}

pub struct UploadService {
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

        debug!("Updated max current uploads from {} to {}", old, max_curr);
    }

    pub fn new(max_curr: u32) -> Self {
        let max_running = Arc::new(Mutex::new(max_curr));
        let max_running_clone = Arc::clone(&max_running);

        let upload_queue = Arc::new(Mutex::new(IndexMap::new()));
        let upload_queue_clone = Arc::clone(&upload_queue);

        let upload_handle = Arc::new(Mutex::new(HashMap::new()));
        let upload_handle_clone = Arc::clone(&upload_handle);

        let (stop_tx, stop_rx) = mpsc::channel(1);

        Self {
            upload_queue,
            upload_handle,
            _upload_backgnd: task::spawn(async move {
                upload_background(
                    upload_queue_clone,
                    upload_handle_clone,
                    max_running_clone,
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
        &mut self,
        user: &User,
        video: &VideoInfo,
        config_copy: Arc<Mutex<ConfigRoot>>,
        clients: Arc<Mutex<HashMap<u64, MyClient>>>,
    ) -> Result<bool> {
        let task = UploadTask::new(user, video, config_copy, clients);
        if self.upload_queue.lock().await.contains_key(&task.id) {
            warn!("任务已存在: {:?}", task);
            return Ok(false); // 任务已存在
        }
        // 将任务添加到上传队列
        let title = task.title().clone();
        self.upload_queue
            .lock()
            .await
            .insert(task.id.clone(), Arc::new(Mutex::new(task)));
        info!("创建上传任务: {:?}", title);
        Ok(true)
    }

    pub async fn get_upload_queue(&mut self) -> Result<Vec<UploadTask>> {
        let mut tasks = Vec::new();
        // trace!("获取上传队列");
        for task_mutex in self.upload_queue.lock().await.values() {
            tasks.push(task_mutex.lock().await.clone());

            if task_mutex.lock().await.is_completed() {
                let title = task_title!(task_mutex);
                // pull out from the handles
                if self
                    .upload_handle
                    .lock()
                    .await
                    .remove(&task_mutex.lock().await.id)
                    .is_some()
                {
                    info!("清理后台任务: {}", title)
                }
            }
        }
        Ok(tasks)
    }

    /// 上传视频
    pub async fn start_upload(&mut self, task_id: &str) -> Result<bool> {
        info!("尝试开始任务: {}", task_id);
        if let Some(task_mutex) = self.upload_queue.lock().await.get(task_id) {
            if task_mutex.lock().await.is_running() || task_mutex.lock().await.is_completed() {
                return Ok(false);
            }
            if task_mutex.lock().await.is_paused()
                || task_mutex.lock().await.is_failed()
                || task_mutex.lock().await.is_waiting()
            {
                info!("任务切换至pending: {}", task_title!(task_mutex));
                task_mutex.lock().await.pending();
                return Ok(true);
            }
            Ok(false)
        } else {
            Err(anyhow::anyhow!("任务ID不存在: {}", task_id))
        }
    }

    /// 暂停上传
    pub async fn pause_upload(&mut self, task_id: &str) -> Result<bool> {
        if let Some(task_mutex) = self.upload_queue.lock().await.get(task_id) {
            info!("任务已暂停: {}", task_title!(task_mutex));
            task_mutex.lock().await.pause();
            Ok(true)
        } else {
            Err(anyhow::anyhow!("任务ID不存在: {}", task_id))
        }
    }

    /// 取消上传
    pub async fn cancel_upload(&self, task_id: &str) -> Result<bool> {
        if let Some(task_mutex) = self.upload_queue.lock().await.shift_remove(task_id) {
            task_mutex.lock().await.cancel();

            let handle = self.upload_handle.lock().await.remove(task_id);
            if let Some(handle) = handle {
                handle.abort();
                info!("结束后台任务: {}", task_title!(task_mutex));
            }
            info!("取消任务成功: {}", task_title!(task_mutex));
            Ok(true)
        } else {
            Err(anyhow::anyhow!("任务ID不存在: {}", task_id))
        }
    }

    pub async fn retry_upload(&self, task_id: &str) -> Result<bool> {
        if let Some(task_mutex) = self.upload_queue.lock().await.get(task_id) {
            task_mutex.lock().await.cancel();

            let handle = self.upload_handle.lock().await.remove(task_id);
            if let Some(handle) = handle {
                handle.abort();
                info!("结束后台任务: {}", task_title!(task_mutex));
            }
            task_mutex.lock().await.pending();
            info!("任务切换至pending: {}", task_title!(task_mutex));
            Ok(true)
        } else {
            Err(anyhow::anyhow!("任务ID不存在: {}", task_id))
        }
    }
}

impl Drop for UploadService {
    fn drop(&mut self) {
        let _ = block_on(self.stop_tx.send(()));
        info!("上传服务已停止");
    }
}

async fn upload_background(
    queue: Arc<Mutex<IndexMap<String, Arc<Mutex<UploadTask>>>>>,
    handle: Arc<Mutex<HashMap<String, task::JoinHandle<()>>>>,
    max_running: Arc<Mutex<u32>>,
    mut stop_rx: mpsc::Receiver<()>,
) {
    let mut one_sec = tokio::time::interval(tokio::time::Duration::from_secs(1));
    loop {
        let queue_clone = Arc::clone(&queue);
        let handle_clone = Arc::clone(&handle);
        let max_running_clone = Arc::clone(&max_running);
        select! {
            _ = stop_rx.recv() => {
                info!("上传服务已停止");
                return;
            }
            _ = one_sec.tick() => {
                upload_background_interval(queue_clone, handle_clone, max_running_clone).await;
            }
        }
    }
}

async fn upload_background_interval(
    queue: Arc<Mutex<IndexMap<String, Arc<Mutex<UploadTask>>>>>,
    handle: Arc<Mutex<HashMap<String, task::JoinHandle<()>>>>,
    max_running: Arc<Mutex<u32>>,
) {
    let current_running = handle.lock().await.len() as u32;
    let mut remain = *max_running.lock().await - current_running;
    if remain > 0 {
        // debug!(
        //     "当前运行任务数: {}, 最大并行数: {}",
        //     current_running,
        //     max_running.lock().await
        // );
    } else {
        debug!("当前运行任务数已达最大并行数: {}", current_running);
        return;
    }

    let task_mutexes: Vec<_> = queue.lock().await.values().cloned().collect();
    for task_mutex in task_mutexes {
        // let task_guard = task_mutex.lock().await;
        // info!(
        //     "任务：{}, 状态: {:?}",
        //     task_guard.title(),
        //     task_guard.status
        // );
        // // let is_pending = task_guard.is_pending();
        // drop(task_guard); // 显式释放锁

        if task_mutex.lock().await.is_pending() && remain > 0 {
            let task_id = task_mutex.lock().await.id.to_string();
            if handle.lock().await.get(&task_id).is_some() {
                continue;
            }
            let task_mutex_clone = Arc::clone(&task_mutex);
            handle.lock().await.insert(
                task_id,
                task::spawn(async move {
                    let _ = upload_impl(task_mutex_clone).await;
                }),
            );
            remain -= 1;
        }

        if remain == 0 {
            break;
        }
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

async fn upload_impl(task_mutex: Arc<Mutex<UploadTask>>) -> Result<()> {
    info!("开始上传任务: {}", task_title!(task_mutex));
    task_mutex.lock().await.start();
    let uid = task_mutex.lock().await.user.uid;

    let (line, _proxy, limit) = {
        let lock = task_mutex.lock().await;
        let config_root = lock.config().lock().await;
        let config = config_root
            .config
            .get(&uid)
            .ok_or_else(|| anyhow::anyhow!("用户未登录或不存在"))?;

        (config.line.clone(), config.proxy.clone(), config.limit)
    };

    let client = &task_mutex
        .lock()
        .await
        .clients()
        .lock()
        .await
        .get(&uid)
        .ok_or_else(|| anyhow::anyhow!("用户未登录或不存在"))?
        .bilibili
        .clone();

    let probe = if let Some(line) = line {
        match line.as_str() {
            "bda2" => line::bda2(),
            "ws" => line::ws(),
            "qn" => line::qn(),
            _ => unreachable!(),
        }
    } else {
        line::Probe::probe(&client.client).await?
    };

    let filepath = PathBuf::from(&task_mutex.lock().await.video.path);
    let video_file = VideoFile::new(&filepath)?;
    let total_size = task_mutex.lock().await.total_size;
    let parcel = probe.pre_upload(client, video_file).await?;
    let (file_read_tx, mut file_read_rx) = mpsc::unbounded_channel();
    let (net_send_tx, mut net_send_rx) = mpsc::unbounded_channel();
    let mut uploaded = 0;
    let mut video_fut = Box::pin(parcel.upload(
        StatelessClient::default(),
        limit as usize,
        |vs| {
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
        },
        3,
    ));

    let mut paused = false;
    loop {
        while task_mutex.lock().await.is_paused() {
            paused = true;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        if paused {
            paused = false;
            if task_mutex.lock().await.is_cancelled() {
                info!("任务已取消: {}", task_title!(task_mutex));
                return Ok(());
            }
            info!("任务已恢复: {}", task_title!(task_mutex));
            task_mutex.lock().await.start();
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
            Ok(return_video) = &mut video_fut => {
                // 处理视频上传完成

                // rewrite the stored id & name
                task_mutex.lock().await.video.filename = return_video.filename.clone();
                task_mutex.lock().await.video.path.clear();
                info!("上传任务完成: {} -> {}", task_title!(task_mutex), return_video.filename);
                task_mutex.lock().await.complete();

                return Ok(())
            }
        }
    }
}
