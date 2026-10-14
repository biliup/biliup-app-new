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

const MEMBER: &str = "https://member.bilibili.com";
const CALLBACK_PATH: &str = "/__biliup_upload_verification__";
const CANCELLED: &str = "安全验证失败或已取消，后续上传已暂停；点击开始或重试重新验证";
// Windows 下 wry 使用 Cookie::domain()，前导点会被移除；按实际主机写入避免子域丢失登录态。
#[cfg(all(not(test), target_os = "windows"))]
const COOKIE_DOMAINS: &[&str] = &["member.bilibili.com", "api.bilibili.com"];
#[cfg(all(not(test), not(target_os = "windows")))]
const COOKIE_DOMAINS: &[&str] = &[".bilibili.com"];

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
            .is_some_and(|session| matches!(*session.borrow(), VerificationState::Failed(_)))
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
                    receiver.changed().await.map_err(|_| anyhow!(CANCELLED))?
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
                        Err(anyhow!(CANCELLED))
                    },
                    result = async {
                        let _window_lock = service.window_lock.lock().await;
                        tracing::info!(uid, "上传风控：开始恢复验证挑战");
                        let voucher = recover_voucher(&bili, &line, &path).await?;
                        let app = service.app.get().ok_or_else(|| anyhow!("应用窗口尚未初始化"))?;
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
        .ok_or_else(|| anyhow!("无法读取上传线路"))?;
    let response = bili
        .client
        .get(format!("{MEMBER}/preupload?{query}"))
        .query(&json!({
            "name": file.file_name, "size": file.total_size, "r": "upos", "profile": "ugcupos/bup",
            "ssl": 0, "version": "2.14.0", "build": 2140000,
        }))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("恢复验证挑战请求失败")?;
    let status = response.status().as_u16();
    let response: Value = response.json().await.context("恢复验证挑战响应不是 JSON")?;
    tracing::info!(status, code = ?response["code"].as_i64(), has_voucher = response["detail"]["v_voucher"].as_str().is_some_and(|v| !v.trim().is_empty()), "上传风控：恢复挑战响应");
    ensure!(
        response["code"].as_i64() == Some(601),
        "未取得安全验证挑战，请稍后手动重试"
    );
    response["detail"]["v_voucher"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("服务器未提供验证码挑战，请稍后手动重试"))
}

pub fn verified_line(line: &Line, token: &str) -> Result<Line> {
    ensure!(!token.trim().is_empty(), "安全验证结果为空");
    let mut serialized = serde_json::to_value(line)?;
    let query = serialized["query"]
        .as_str()
        .ok_or_else(|| anyhow!("无法读取上传线路"))?;
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
                    "origin" => "验证页面来源检查失败",
                    "account" => "验证窗口登录账号检查失败",
                    "sdk_load" => "官方验证码 SDK 加载失败",
                    "sdk_verify" => "官方验证码 SDK 验证失败或取消",
                    "sdk_result" => "官方验证码 SDK 返回无效凭据",
                    _ => CANCELLED,
                };
                anyhow!(reason)
            }),
    )
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
async fn verify_window(
    app: &AppHandle,
    uid: u64,
    bili: &BiliBili,
    voucher: &str,
    proxy: Option<&str>,
) -> Result<String> {
    let session = format!(
        "{uid}-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let callback = format!("{MEMBER}{CALLBACK_PATH}#{session}");
    let options = json!({"riskParams": {"v_voucher": voucher}}).to_string();
    let script = include_str!("upload_risk_verification.js")
        .replace("__RISK_OPTIONS__", &options)
        .replace("__EXPECTED_UID__", &uid.to_string());
    let callback_json = serde_json::to_string(&callback)?;
    let script = format!(
        "{script}.then(token => {{ location.href = {callback_json} + '&token=' + encodeURIComponent(token) }}, error => {{ console.error('[上传风控] 验证失败，阶段：', window.__biliupRiskStage, error); location.href = {callback_json} + '&cancelled=1&stage=' + encodeURIComponent(window.__biliupRiskStage || '') }})"
    );
    let (sender, receiver) = oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let nav_sender = Arc::clone(&sender);
    let load_sender = Arc::clone(&sender);
    let started = AtomicBool::new(false);
    let mut builder = WebviewWindowBuilder::new(
        app,
        "upload-risk",
        WebviewUrl::External(Url::parse("about:blank")?),
    )
    .title(format!("上传安全验证 · {uid}"))
    .inner_size(900.0, 720.0)
    .incognito(true)
    .on_navigation(move |url| {
        if url.origin().ascii_serialization() == MEMBER && url.path() == CALLBACK_PATH {
            if let Some(result) = verification_result(url, &session) {
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
                finish(&load_sender, Err(anyhow!("无法启动安全验证脚本")));
            }
        }
    });
    if let Some(proxy) = proxy {
        builder = builder.proxy_url(Url::parse(proxy)?);
    }
    let window = VerificationWindow(builder.build().context("创建安全验证窗口失败")?);
    tracing::info!(uid, "上传风控：验证窗口已创建");
    #[cfg(debug_assertions)]
    window.0.open_devtools();
    #[cfg(debug_assertions)]
    let (closed_sender, closed_receiver) = oneshot::channel();
    #[cfg(debug_assertions)]
    let closed_sender = Mutex::new(Some(closed_sender));
    let close_sender = Arc::clone(&sender);
    window.0.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
        ) {
            tracing::info!(uid, event = ?event, "上传风控：窗口关闭事件");
            #[cfg(debug_assertions)]
            {
                if let Some(sender) = closed_sender.lock().unwrap().take() {
                    let _ = sender.send(());
                }
            }
            finish(&close_sender, Err(anyhow!(CANCELLED)));
        }
    });
    let result: Result<String> = async {
        let cookies = bili.login_info.cookie_info["cookies"]
            .as_array()
            .ok_or_else(|| anyhow!("账号 Cookie 无效"))?;
        let sessdata = cookies
            .iter()
            .find(|cookie| cookie["name"] == "SESSDATA")
            .and_then(|cookie| cookie["value"].as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("上传账号缺少 SESSDATA，无法初始化验证窗口登录态"))?;
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
                        .context("写入验证窗口 Cookie 失败")?;
                }
            }
        }
        // set_cookie 仅投递到 UI 线程；读回既确认写入已处理，也检查子域是否可见。
        for host in ["member.bilibili.com", "api.bilibili.com"] {
            let webview = window.0.clone();
            let url = Url::parse(&format!("https://{host}/"))?;
            let stored = tokio::task::spawn_blocking(move || webview.cookies_for_url(url))
                .await
                .context("读取验证窗口 Cookie 任务失败")?
                .context("读取验证窗口 Cookie 失败")?;
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
            ensure!(matches, "验证窗口登录 Cookie 未正确写入 {host}");
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
            .context("导航到安全验证页面失败")?;
        receiver.await.map_err(|_| anyhow!(CANCELLED))?
    }
    .await;
    #[cfg(debug_assertions)]
    if let Err(error) = &result {
        tracing::warn!(uid, error = %error, "上传风控：调试模式保留失败窗口，请收集 Console 和 Network 后手动关闭");
        let _ = window
            .0
            .set_title("上传安全验证失败 · 请收集 Console / Network 后关闭窗口");
        let _ = closed_receiver.await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
        finish(&sender, Err(anyhow!(CANCELLED)));
        assert_eq!(receiver.await.unwrap().unwrap(), "token");
    }

    #[test]
    fn script_failure_reports_known_stage_without_echoing_arbitrary_data() {
        let callback = format!("{MEMBER}{CALLBACK_PATH}#session&cancelled=1&stage=account");
        let error = verification_result(&Url::parse(&callback).unwrap(), "session")
            .unwrap()
            .unwrap_err();
        assert_eq!(error.to_string(), "验证窗口登录账号检查失败");
        let callback = callback.replace("stage=account", "stage=secret-voucher");
        let error = verification_result(&Url::parse(&callback).unwrap(), "session")
            .unwrap()
            .unwrap_err();
        assert_eq!(error.to_string(), CANCELLED);
    }

    #[test]
    fn failure_requires_manual_retry_and_tokens_are_account_scoped_and_expire() {
        let service = UploadRiskService::default();
        service.hold(1, CANCELLED);
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
