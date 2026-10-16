use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, ensure};
use biliup::{
    bilibili::BiliBili,
    uploader::{VideoFile, line::Line},
};
use serde_json::{Value, json};
#[cfg(not(test))]
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
#[cfg(not(test))]
use tauri::{
    WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
    webview::{Cookie, PageLoadEvent},
};
use tokio::sync::{Mutex as AsyncMutex, oneshot, watch};
use url::Url;

#[cfg(not(test))]
use super::upload_risk_proxy::VerificationProxy;

const MEMBER: &str = "https://member.bilibili.com";
const CALLBACK_PATH: &str = "/__biliup_upload_verification__";
fn cancelled() -> &'static str {
    crate::i18n::text("risk.cancelled")
}
// 按实际主机写入，兼容 Windows 的域处理和 macOS 的 Cookie 读回精确匹配。
#[cfg(not(test))]
const COOKIE_DOMAINS: &[&str] = &["member.bilibili.com", "api.bilibili.com"];

#[derive(Clone)]
pub(super) enum VerificationState {
    Pending,
    Verified(String, Instant),
    Failed(String),
}

#[derive(Default)]
pub struct UploadRiskService {
    app: OnceLock<AppHandle>,
    sessions: Mutex<HashMap<u64, Arc<watch::Sender<VerificationState>>>>,
    window_lock: AsyncMutex<()>,
}

impl UploadRiskService {
    pub fn set_app_handle(&self, app: AppHandle) {
        let _ = self.app.set(app);
    }

    pub fn is_blocked(&self, uid: u64) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(&uid)
            .is_some_and(|session| !matches!(*session.borrow(), VerificationState::Verified(_, _)))
    }

    pub(super) fn subscribe(&self, uid: u64) -> Option<watch::Receiver<VerificationState>> {
        self.sessions
            .lock()
            .unwrap()
            .get(&uid)
            .filter(|session| !matches!(*session.borrow(), VerificationState::Verified(_, _)))
            .map(|session| session.subscribe())
    }

    pub fn recent_token(&self, uid: u64) -> Option<String> {
        let sessions = self.sessions.lock().unwrap();
        match &*sessions.get(&uid)?.borrow() {
            VerificationState::Verified(token, at) if at.elapsed() < Duration::from_secs(120) => {
                Some(token.clone())
            }
            _ => None,
        }
    }

    pub fn clear(&self, uid: u64) {
        self.sessions.lock().unwrap().remove(&uid);
    }

    pub fn prepare_retry(&self, uid: u64) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions
            .get(&uid)
            .is_some_and(|session| match *session.borrow() {
                VerificationState::Failed(_) => true,
                VerificationState::Pending => session.receiver_count() == 0,
                VerificationState::Verified(_, _) => false,
            })
        {
            sessions.remove(&uid);
            return true;
        }
        false
    }

    pub fn hold(&self, uid: u64, message: &str) {
        let (sender, _) = watch::channel(VerificationState::Failed(message.to_string()));
        self.sessions.lock().unwrap().insert(uid, Arc::new(sender));
    }

    pub(super) async fn wait(mut receiver: watch::Receiver<VerificationState>) -> Result<String> {
        loop {
            let state = receiver.borrow_and_update().clone();
            match state {
                VerificationState::Verified(token, _) => return Ok(token),
                VerificationState::Failed(message) => return Err(anyhow!(message)),
                VerificationState::Pending => {
                    receiver.changed().await.map_err(|_| anyhow!(cancelled()))?
                }
            }
        }
    }

    pub async fn verify(
        self: &Arc<Self>,
        uid: u64,
        bili: BiliBili,
        line: Line,
        path: PathBuf,
        proxy: Option<String>,
    ) -> Result<String> {
        let (receiver, leader) = {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session) = sessions
                .get(&uid)
                .filter(|session| !matches!(*session.borrow(), VerificationState::Verified(_, _)))
            {
                (session.subscribe(), None)
            } else {
                let (sender, receiver) = watch::channel(VerificationState::Pending);
                let sender = Arc::new(sender);
                sessions.insert(uid, Arc::clone(&sender));
                (receiver, Some(sender))
            }
        };
        if let Some(sender) = leader {
            tracing::info!(uid, "上传风控：创建验证会话，阻塞账号调度");
            let service = Arc::clone(self);
            // 验证窗口独立于某个上传 worker；最后一个等待任务取消时关闭窗口。
            tokio::spawn(async move {
                let result = tokio::select! {
                    _ = sender.closed() => {
                        tracing::warn!(uid, "上传风控：所有等待任务已取消，结束验证");
                        Err(anyhow!(cancelled()))
                    },
                    result = async {
                        let _window_lock = service.window_lock.lock().await;
                        tracing::info!(uid, "上传风控：开始恢复验证挑战");
                        let voucher = recover_voucher(&bili, &line, &path).await?;
                        let app = service.app.get().ok_or_else(|| anyhow!(crate::i18n::text("risk.app_unavailable")))?;
                        verify_window(app, uid, &bili, &voucher, proxy.as_deref()).await
                    } => result,
                };
                let state = match result {
                    Ok(token) => {
                        tracing::info!(uid, "上传风控：验证成功，解除账号阻塞");
                        VerificationState::Verified(token, Instant::now())
                    }
                    Err(error) => {
                        tracing::warn!(uid, error = %error, "上传风控：验证结束，账号继续阻塞");
                        VerificationState::Failed(error.to_string())
                    }
                };
                let sessions = service.sessions.lock().unwrap();
                if sessions
                    .get(&uid)
                    .is_some_and(|current| Arc::ptr_eq(current, &sender))
                {
                    sender.send_replace(state);
                }
            });
        }
        Self::wait(receiver).await
    }
}

async fn recover_voucher(bili: &BiliBili, line: &Line, path: &PathBuf) -> Result<String> {
    let file = VideoFile::new(path)?;
    let serialized = serde_json::to_value(line)?;
    let query = serialized["query"]
        .as_str()
        .ok_or_else(|| anyhow!(crate::i18n::text("risk.route_unavailable")))?;
    let request = bili
        .client
        .get(format!("{MEMBER}/preupload?{query}"))
        .query(&json!({
            "name": file.file_name, "size": file.total_size, "r": "upos", "profile": "ugcupos/bup",
            "ssl": 0, "version": "2.14.0", "build": 2140000,
        }));
    #[cfg(not(debug_assertions))]
    let request = request.timeout(Duration::from_secs(30));
    let response = request
        .send()
        .await
        .context(crate::i18n::text("risk.challenge_request_failed"))?;
    let status = response.status().as_u16();
    let response: Value = response
        .json()
        .await
        .context(crate::i18n::text("risk.challenge_invalid_json"))?;
    tracing::info!(status, code = ?response["code"].as_i64(), has_voucher = response["detail"]["v_voucher"].as_str().is_some_and(|v| !v.trim().is_empty()), "上传风控：恢复挑战响应");
    ensure!(
        response["code"].as_i64() == Some(601),
        crate::i18n::text("risk.challenge_unavailable")
    );
    response["detail"]["v_voucher"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!(crate::i18n::text("risk.challenge_missing")))
}

pub fn verified_line(line: &Line, token: &str) -> Result<Line> {
    ensure!(
        !token.trim().is_empty(),
        crate::i18n::text("risk.token_empty")
    );
    let mut serialized = serde_json::to_value(line)?;
    let query = serialized["query"]
        .as_str()
        .ok_or_else(|| anyhow!(crate::i18n::text("risk.route_unavailable")))?;
    let mut params = url::form_urlencoded::Serializer::new(String::new());
    params.extend_pairs(
        url::form_urlencoded::parse(query.as_bytes()).filter(|(key, _)| key != "gaia_vtoken"),
    );
    params.append_pair("gaia_vtoken", token);
    serialized["query"] = Value::String(params.finish());
    Ok(serde_json::from_value(serialized)?)
}

#[cfg(not(test))]
struct VerificationWindow(WebviewWindow);
#[cfg(not(test))]
impl Drop for VerificationWindow {
    fn drop(&mut self) {
        let _ = self.0.close();
    }
}

#[cfg(all(target_os = "windows", not(test)))]
struct VerificationDataDirectory(PathBuf);

#[cfg(all(target_os = "windows", not(test)))]
impl Drop for VerificationDataDirectory {
    fn drop(&mut self) {
        let path = self.0.clone();
        tokio::spawn(async move {
            // close 仅投递关闭请求，WebView2 退出前可能仍占用目录中的文件。
            for attempt in 0..30 {
                match tokio::fs::remove_dir_all(&path).await {
                    Ok(()) => return,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
                    Err(error) if attempt == 29 => {
                        tracing::warn!(error = %error, "清理安全验证临时目录失败");
                    }
                    Err(_) => {}
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }
}

fn finish(sender: &Mutex<Option<oneshot::Sender<Result<String>>>>, result: Result<String>) {
    if let Some(sender) = sender.lock().unwrap().take() {
        let _ = sender.send(result);
    }
}

fn verification_result(url: &Url, session: &str) -> Option<Result<String>> {
    if url.origin().ascii_serialization() != MEMBER || url.path() != CALLBACK_PATH {
        return None;
    }
    let fragment = url.fragment()?.strip_prefix(&format!("{session}&"))?;
    let params: HashMap<_, _> = url::form_urlencoded::parse(fragment.as_bytes())
        .into_owned()
        .collect();
    Some(
        params
            .get("token")
            .filter(|token| !token.trim().is_empty())
            .cloned()
            .ok_or_else(|| {
                let stage = params.get("stage").map(String::as_str).unwrap_or("");
                let reason = match stage {
                    "origin" => crate::i18n::text("risk.origin_failed"),
                    "account" => crate::i18n::text("risk.account_failed"),
                    "sdk_load" => crate::i18n::text("risk.sdk_load_failed"),
                    "sdk_verify" => crate::i18n::text("risk.sdk_verify_failed"),
                    "sdk_result" => crate::i18n::text("risk.sdk_result_invalid"),
                    _ => cancelled(),
                };
                anyhow!(reason)
            }),
    )
}

fn verification_ready(url: &Url, session: &str) -> bool {
    url.origin().ascii_serialization() == MEMBER
        && url.path() == CALLBACK_PATH
        && url.fragment() == Some(format!("{session}&ready=1").as_str())
}

async fn wait_for_verification(
    mut receiver: oneshot::Receiver<Result<String>>,
    initialize: impl std::future::Future<Output = Result<()>>,
    timeout: Option<Duration>,
) -> Result<String> {
    let initialize = async {
        match timeout {
            Some(timeout) => tokio::time::timeout(timeout, initialize)
                .await
                .context(crate::i18n::text("risk.initialization_timeout"))?,
            None => initialize.await,
        }
    };
    tokio::select! {
        result = &mut receiver => result.map_err(|_| anyhow!(cancelled()))?,
        initialized = initialize => {
            initialized?;
            // SDK 已就绪，之后只等待用户完成验证或关闭窗口。
            receiver.await.map_err(|_| anyhow!(cancelled()))?
        }
    }
}

#[cfg(test)]
async fn verify_window(
    _: &AppHandle,
    _: u64,
    _: &BiliBili,
    _: &str,
    _: Option<&str>,
) -> Result<String> {
    Err(anyhow!("单元测试不打开真实验证窗口"))
}

#[cfg(not(test))]
enum VerificationAction<'a> {
    Upload(&'a str),
    Delete { sms_required: bool },
}

pub(crate) async fn verify_delete_window(
    app: &AppHandle,
    uid: u64,
    bili: &BiliBili,
    proxy: Option<&str>,
    sms_required: bool,
) -> Result<String> {
    #[cfg(test)]
    {
        let _ = (app, uid, bili, proxy, sms_required);
        Err(anyhow!(cancelled()))
    }
    #[cfg(not(test))]
    {
        verify_action_window(
            app,
            uid,
            bili,
            proxy,
            VerificationAction::Delete { sms_required },
        )
        .await
        .map_err(|error| {
            if error.to_string() == cancelled() {
                anyhow!(crate::i18n::text("delete.cancelled"))
            } else {
                error
            }
        })
    }
}

#[cfg(not(test))]
async fn verify_window(
    app: &AppHandle,
    uid: u64,
    bili: &BiliBili,
    voucher: &str,
    proxy: Option<&str>,
) -> Result<String> {
    verify_action_window(app, uid, bili, proxy, VerificationAction::Upload(voucher)).await
}

#[cfg(not(test))]
async fn verify_action_window(
    app: &AppHandle,
    uid: u64,
    bili: &BiliBili,
    proxy: Option<&str>,
    action: VerificationAction<'_>,
) -> Result<String> {
    let (label, source, options, title) = match action {
        VerificationAction::Upload(voucher) => (
            "upload-risk",
            include_str!("upload_risk_verification.js"),
            json!({"riskParams": {"v_voucher": voucher}}),
            crate::tr!("risk.window_title", uid = uid),
        ),
        VerificationAction::Delete { sms_required } => (
            "archive-delete",
            include_str!("archive_delete_verification.js"),
            json!({"smsRequired": sms_required}),
            crate::tr!("delete.window_title", uid = uid),
        ),
    };
    let session = format!(
        "{label}-{uid}-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    // 每次本地代理的端口不同，不能复用主窗口或上一次验证的 WebView2 环境。
    // 在窗口 guard 之前创建，保证先关闭窗口再清理目录；创建窗口失败也会清理。
    #[cfg(target_os = "windows")]
    let data_directory = (proxy.is_some() || label == "archive-delete").then(|| {
        VerificationDataDirectory(
            std::env::temp_dir().join(format!("biliup-verification-{session}")),
        )
    });
    let callback = format!("{MEMBER}{CALLBACK_PATH}#{session}");
    let ready_callback = serde_json::to_string(&format!("{callback}&ready=1"))?;
    let mut messages = json!({
        "origin": crate::i18n::text("risk.script.origin"),
        "account_invalid": crate::i18n::text("risk.script.account_invalid"),
        "account_mismatch": crate::i18n::text("risk.script.account_mismatch"),
        "sdk_load_failed": crate::i18n::text("risk.script.sdk_load_failed"),
        "sdk_timeout": crate::i18n::text("risk.script.sdk_timeout"),
        "token_invalid": crate::i18n::text("risk.script.token_invalid"),
    });
    for key in [
        "delete.script.title",
        "delete.script.warning",
        "delete.script.send_sms",
        "delete.script.confirm_sms",
        "delete.script.sms_placeholder",
        "delete.script.sms_invalid",
        "delete.script.sms_failed",
        "delete.script.sms_sent",
        "delete.script.captcha",
        "delete.script.failed",
        "delete.script.cancel",
    ] {
        messages.as_object_mut().unwrap().insert(
            key.trim_start_matches("delete.script.").into(),
            json!(crate::i18n::text(key)),
        );
    }
    messages.as_object_mut().unwrap().insert(
        "language".into(),
        json!(crate::i18n::text("delete.script.language")),
    );
    let script = source
        .replace("__RISK_MESSAGES__", &messages.to_string())
        .replace("__RISK_OPTIONS__", &options.to_string())
        .replace("__EXPECTED_UID__", &uid.to_string())
        .replace(
            "__RISK_TIMEOUT_MS__",
            if cfg!(debug_assertions) {
                "null"
            } else {
                "30000"
            },
        )
        .replace("__RISK_READY_CALLBACK__", &ready_callback);
    let callback_json = serde_json::to_string(&callback)?;
    let script = format!(
        "{script}.then(token => {{ location.href = {callback_json} + '&token=' + encodeURIComponent(token) }}, error => {{ console.error('[上传风控] 验证失败，阶段：', window.__biliupRiskStage, error); location.href = {callback_json} + '&cancelled=1&stage=' + encodeURIComponent(window.__biliupRiskStage || '') }})"
    );
    let (sender, receiver) = oneshot::channel();
    let (ready_sender, ready_receiver) = oneshot::channel();
    let ready_sender = Mutex::new(Some(ready_sender));
    let sender = Arc::new(Mutex::new(Some(sender)));
    let nav_sender = Arc::clone(&sender);
    let load_sender = Arc::clone(&sender);
    let started = AtomicBool::new(false);
    let mut builder = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External(Url::parse("about:blank")?),
    )
    .title(title)
    .inner_size(900.0, 720.0)
    .incognito(true)
    .on_navigation(move |url| {
        if url.origin().ascii_serialization() == MEMBER && url.path() == CALLBACK_PATH {
            if verification_ready(url, &session) {
                if let Some(sender) = ready_sender.lock().unwrap().take() {
                    let _ = sender.send(());
                }
            } else if let Some(result) = verification_result(url, &session) {
                if let Err(error) = &result {
                    tracing::warn!(uid, error = %error, "上传风控：收到脚本失败回传");
                } else {
                    tracing::info!(uid, "上传风控：收到验证成功回传");
                }
                finish(&nav_sender, result);
            }
            return false;
        }
        let allowed = url.as_str() == "about:blank" || url.origin().ascii_serialization() == MEMBER;
        tracing::info!(uid, origin = %url.origin().ascii_serialization(), path = %url.path(), allowed, "上传风控：页面导航（省略参数）");
        allowed
    })
    .on_page_load(move |window, payload| {
        tracing::info!(uid, event = ?payload.event(), origin = %payload.url().origin().ascii_serialization(), path = %payload.url().path(), "上传风控：页面加载事件");
        if payload.event() == PageLoadEvent::Finished
            && payload.url().origin().ascii_serialization() == MEMBER
            && !started.swap(true, Ordering::SeqCst)
        {
            tracing::info!(uid, "上传风控：注入验证脚本");
            if let Err(error) = window.eval(&script) {
                tracing::warn!(uid, error = %error, "上传风控：脚本注入失败");
                finish(&load_sender, Err(anyhow!(crate::i18n::text("risk.script_failed"))));
            }
        }
    });
    #[cfg(target_os = "windows")]
    if let Some(directory) = &data_directory {
        builder = builder.data_directory(directory.0.clone());
    }
    let _proxy = if let Some(proxy) = proxy {
        let proxy = VerificationProxy::start(proxy).await?;
        builder = builder.proxy_url(proxy.url.clone());
        Some(proxy)
    } else {
        None
    };
    let window = VerificationWindow(
        builder
            .build()
            .context(crate::i18n::text("risk.window_failed"))?,
    );
    tracing::info!(uid, "上传风控：验证窗口已创建");
    #[cfg(debug_assertions)]
    window.0.open_devtools();
    let close_sender = Arc::clone(&sender);
    window.0.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
        ) {
            tracing::info!(uid, event = ?event, "上传风控：窗口关闭事件");
            finish(&close_sender, Err(anyhow!(cancelled())));
        }
    });
    let initialize = async {
        let cookies = bili.login_info.cookie_info["cookies"]
            .as_array()
            .ok_or_else(|| anyhow!(crate::i18n::text("risk.cookies_invalid")))?;
        let sessdata = cookies
            .iter()
            .find(|cookie| cookie["name"] == "SESSDATA")
            .and_then(|cookie| cookie["value"].as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!(crate::i18n::text("risk.sessdata_missing")))?;
        for cookie in cookies {
            if let (Some(name), Some(value)) = (cookie["name"].as_str(), cookie["value"].as_str()) {
                for domain in COOKIE_DOMAINS {
                    window
                        .0
                        .set_cookie(
                            Cookie::build((name, value))
                                .domain(*domain)
                                .path("/")
                                .secure(true)
                                .http_only(name == "SESSDATA")
                                .build(),
                        )
                        .context(crate::i18n::text("risk.cookie_write_failed"))?;
                }
            }
        }
        // set_cookie 仅投递到 UI 线程；读回既确认写入已处理，也检查子域是否可见。
        for host in ["member.bilibili.com", "api.bilibili.com"] {
            let webview = window.0.clone();
            let url = Url::parse(&format!("https://{host}/"))?;
            let stored = tokio::task::spawn_blocking(move || webview.cookies_for_url(url))
                .await
                .context(crate::i18n::text("risk.cookie_task_failed"))?
                .context(crate::i18n::text("risk.cookie_read_failed"))?;
            let matches = stored
                .iter()
                .any(|cookie| cookie.name() == "SESSDATA" && cookie.value() == sessdata);
            tracing::info!(
                uid,
                host,
                count = stored.len(),
                sessdata_matches = matches,
                "上传风控：验证窗口 Cookie 读回检查"
            );
            ensure!(matches, crate::tr!("risk.cookie_not_stored", host = host));
        }
        tracing::info!(
            uid,
            count = cookies.len(),
            "上传风控：Cookie 写入完成，导航到投稿页"
        );
        window
            .0
            .navigate(Url::parse(&format!(
                "{MEMBER}/platform/upload/video/frame"
            ))?)
            .context(crate::i18n::text("risk.navigation_failed"))?;
        ready_receiver
            .await
            .context(crate::i18n::text("risk.initialization_interrupted"))
    };
    let timeout = (!cfg!(debug_assertions)).then_some(Duration::from_secs(60));
    wait_for_verification(receiver, initialize, timeout).await
}

#[cfg(test)]
mod initialization_tests {
    use super::*;

    #[tokio::test]
    async fn initialization_timeout_releases_the_window_lock() {
        let lock = Arc::new(AsyncMutex::new(()));
        let guard = lock.lock().await;
        let (_sender, receiver) = oneshot::channel();
        let result = wait_for_verification(
            receiver,
            async move {
                let _guard = guard;
                std::future::pending::<Result<()>>().await
            },
            Some(Duration::from_millis(10)),
        )
        .await;
        assert!(result.unwrap_err().to_string().contains("初始化超时"));
        assert!(lock.try_lock().is_ok());
    }

    #[tokio::test]
    async fn initialized_verification_has_no_human_timeout() {
        let (sender, receiver) = oneshot::channel();
        let (ready, initialized) = oneshot::channel();
        let worker = tokio::spawn(wait_for_verification(
            receiver,
            async move {
                ready.send(()).unwrap();
                Ok(())
            },
            Some(Duration::from_millis(10)),
        ));
        initialized.await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!worker.is_finished());
        sender.send(Ok("token".into())).unwrap();
        assert_eq!(worker.await.unwrap().unwrap(), "token");
    }

    #[tokio::test]
    async fn closing_window_interrupts_initialization_immediately() {
        let (sender, receiver) = oneshot::channel();
        sender.send(Err(anyhow!(cancelled()))).unwrap();
        let result = wait_for_verification(
            receiver,
            std::future::pending(),
            Some(Duration::from_secs(60)),
        )
        .await;
        assert_eq!(result.unwrap_err().to_string(), cancelled());
    }

    #[test]
    fn readiness_callback_is_bound_to_origin_path_and_session() {
        let callback = format!("{MEMBER}{CALLBACK_PATH}#session&ready=1");
        assert!(verification_ready(
            &Url::parse(&callback).unwrap(),
            "session"
        ));
        for invalid in [
            callback.replace(MEMBER, "https://evil.invalid"),
            callback.replace(CALLBACK_PATH, "/other"),
            callback.replace("#session", "#other"),
            callback.replace("ready=1", "token=value"),
        ] {
            assert!(!verification_ready(
                &Url::parse(&invalid).unwrap(),
                "session"
            ));
        }
    }

    #[tokio::test]
    async fn debug_initialization_waits_until_cancelled() {
        let (sender, receiver) = oneshot::channel();
        let worker = tokio::spawn(wait_for_verification(
            receiver,
            std::future::pending(),
            None,
        ));
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!worker.is_finished());
        sender.send(Err(anyhow!(cancelled()))).unwrap();
        assert_eq!(worker.await.unwrap().unwrap_err().to_string(), cancelled());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_clears_orphaned_verification_but_preserves_other_waiters() {
        let service = UploadRiskService::default();
        let (sender, receiver) = watch::channel(VerificationState::Pending);
        let sender = Arc::new(sender);
        service
            .sessions
            .lock()
            .unwrap()
            .insert(1, Arc::clone(&sender));
        assert!(!service.prepare_retry(1));
        assert!(service.is_blocked(1));

        drop(receiver);
        assert!(service.prepare_retry(1));
        // 旧验证后台随后失败，也不能再阻塞重试任务。
        sender.send_replace(VerificationState::Failed(cancelled().into()));
        assert!(!service.is_blocked(1));
    }

    #[test]
    fn token_is_encoded_without_changing_route_or_duplicating_credentials() {
        let original = biliup::uploader::line::txa();
        let mut value = serde_json::to_value(&original).unwrap();
        let route = value["query"].as_str().unwrap().to_string();
        value["query"] = json!(format!("{route}&gaia_vtoken=old"));
        let original: Line = serde_json::from_value(value).unwrap();
        let verified = verified_line(&original, "+/= &令牌").unwrap();
        let value = serde_json::to_value(&verified).unwrap();
        let params: Vec<_> =
            url::form_urlencoded::parse(value["query"].as_str().unwrap().as_bytes())
                .into_owned()
                .collect();
        assert_eq!(
            params
                .iter()
                .filter(|(key, _)| key == "gaia_vtoken")
                .count(),
            1
        );
        assert!(params.contains(&("gaia_vtoken".into(), "+/= &令牌".into())));
        assert!(params.contains(&("upcdn".into(), "txa".into())));
        assert!(verified_line(&original, " ").is_err());
    }

    #[test]
    fn navigation_result_is_bound_to_origin_path_and_session() {
        let callback = format!("{MEMBER}{CALLBACK_PATH}#session&token=a%2Bb%3D");
        assert_eq!(
            verification_result(&Url::parse(&callback).unwrap(), "session")
                .unwrap()
                .unwrap(),
            "a+b="
        );
        assert!(verification_result(&Url::parse(&callback).unwrap(), "other").is_none());
        assert!(
            verification_result(
                &Url::parse(&callback.replace(MEMBER, "https://evil.invalid")).unwrap(),
                "session"
            )
            .is_none()
        );
        assert!(
            verification_result(
                &Url::parse(&callback.replace(CALLBACK_PATH, "/other")).unwrap(),
                "session"
            )
            .is_none()
        );
        assert!(
            verification_result(
                &Url::parse(&callback.replace("token=a%2Bb%3D", "cancelled=1")).unwrap(),
                "session"
            )
            .unwrap()
            .is_err()
        );
    }

    #[tokio::test]
    async fn concurrent_waiters_share_result_and_close_cannot_override_success() {
        let (sender, receiver) = watch::channel(VerificationState::Pending);
        let second = receiver.clone();
        sender.send_replace(VerificationState::Verified("token".into(), Instant::now()));
        assert_eq!(UploadRiskService::wait(receiver).await.unwrap(), "token");
        assert_eq!(UploadRiskService::wait(second).await.unwrap(), "token");
        let (sender, receiver) = oneshot::channel();
        let sender = Mutex::new(Some(sender));
        finish(&sender, Ok("token".into()));
        finish(&sender, Err(anyhow!(cancelled())));
        assert_eq!(receiver.await.unwrap().unwrap(), "token");
    }

    #[test]
    fn script_failure_reports_known_stage_without_echoing_arbitrary_data() {
        let callback = format!("{MEMBER}{CALLBACK_PATH}#session&cancelled=1&stage=account");
        let error = verification_result(&Url::parse(&callback).unwrap(), "session")
            .unwrap()
            .unwrap_err();
        assert_eq!(error.to_string(), crate::i18n::text("risk.account_failed"));
        let callback = callback.replace("stage=account", "stage=secret-voucher");
        let error = verification_result(&Url::parse(&callback).unwrap(), "session")
            .unwrap()
            .unwrap_err();
        assert_eq!(error.to_string(), cancelled());
    }

    #[test]
    fn failure_requires_manual_retry_and_tokens_are_account_scoped_and_expire() {
        let service = UploadRiskService::default();
        service.hold(1, cancelled());
        assert!(service.is_blocked(1));
        assert!(!service.is_blocked(2));
        assert!(service.prepare_retry(1));
        assert!(!service.is_blocked(1));
        let (sender, _) =
            watch::channel(VerificationState::Verified("token".into(), Instant::now()));
        service.sessions.lock().unwrap().insert(1, Arc::new(sender));
        assert!(!service.is_blocked(1));
        assert!(service.subscribe(1).is_none());
        assert_eq!(service.recent_token(1).as_deref(), Some("token"));
        assert!(service.recent_token(2).is_none());
        service
            .sessions
            .lock()
            .unwrap()
            .get(&1)
            .unwrap()
            .send_replace(VerificationState::Verified(
                "token".into(),
                Instant::now() - Duration::from_secs(121),
            ));
        assert!(service.recent_token(1).is_none());
        service.clear(1);
        assert!(service.sessions.lock().unwrap().is_empty());
    }
}
