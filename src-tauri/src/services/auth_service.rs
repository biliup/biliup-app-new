use std::{path::PathBuf, sync::Arc};

use crate::{models::User, utils::crypto::encode_base64};
use anyhow::Result;
use biliup::{
    bilibili::BiliBili,
    credential::{Credential, LoginInfo},
    error::Kind,
};
use qrcode::QrCode;
use serde_json::Value;
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::{debug, info};
use url::Url;

pub enum SmsSendState {
    Sent,
    NeedRecaptcha(String),
}

#[derive(Clone)]
pub enum QrLoginStatus {
    Pending,
    Success(LoginInfo),
    Expired(String),
    Error(String),
}

pub enum QrLoginCheckResult {
    Pending,
    Success(LoginInfo),
    Expired(String),
    Error(String),
    Idle,
}

struct SmsRecaptchaContext {
    phone: u64,
    country_code: u32,
    recaptcha_url: String,
}

struct QrSession {
    status: Arc<Mutex<QrLoginStatus>>,
    handle: JoinHandle<()>,
}

pub struct AuthService {
    credential: Option<Credential>,
    qr_session: Option<QrSession>,
    sms_payload: Option<Value>,
    sms_recaptcha_context: Option<SmsRecaptchaContext>,
    proxy: Option<String>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            credential: None,
            qr_session: None,
            sms_payload: None,
            sms_recaptcha_context: None,
            proxy: None,
        }
    }

    pub fn init(&mut self, proxy: Option<&str>) {
        self.reset_qr_session();
        self.proxy = proxy.map(String::from);
        self.credential = Some(Credential::new(proxy));
        self.sms_payload = None;
        self.sms_recaptcha_context = None;
    }

    /// 仅更新代理和 credential，不清除 SMS 相关状态
    pub fn set_proxy_and_credential(&mut self, proxy: Option<&str>) {
        self.proxy = proxy.map(String::from);
        self.credential = Some(Credential::new(proxy));
    }

    pub fn get_proxy(&self) -> Option<String> {
        self.proxy.clone()
    }

    pub async fn send_sms_code(&mut self, phone: u64, country_code: u32) -> Result<SmsSendState> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.service_uninitialized")))?;

        match credential.send_sms(phone, country_code).await {
            Ok(payload) => {
                self.sms_payload = Some(payload);
                self.sms_recaptcha_context = None;
                Ok(SmsSendState::Sent)
            }
            Err(Kind::NeedRecaptcha(url)) => {
                self.sms_payload = None;
                self.sms_recaptcha_context = Some(SmsRecaptchaContext {
                    phone,
                    country_code,
                    recaptcha_url: url.clone(),
                });
                Ok(SmsSendState::NeedRecaptcha(url))
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn submit_sms_recaptcha(&mut self, challenge: &str, validate: &str) -> Result<()> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.service_uninitialized")))?;

        let context = self
            .sms_recaptcha_context
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.sms_request_required")))?;

        let recaptcha_token = Url::parse(&context.recaptcha_url)
            .map_err(|_| anyhow::anyhow!("{}", crate::tr!("auth.slider_url_invalid")))?
            .query_pairs()
            .find(|(key, _)| key == "recaptcha_token")
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.slider_token_missing")))?;

        let payload = credential
            .send_sms_with_recaptcha(
                context.phone,
                context.country_code,
                Some(challenge),
                Some(validate),
                Some(&recaptcha_token),
            )
            .await?;

        self.sms_payload = Some(payload);
        self.sms_recaptcha_context = None;
        Ok(())
    }

    pub async fn login_with_sms_code_and_phone(
        &mut self,
        code: u32,
        phone: Option<u64>,
        country_code: Option<u32>,
    ) -> Result<LoginInfo> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.service_uninitialized")))?;

        let mut payload = self
            .sms_payload
            .clone()
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.sms_send_required")))?;

        if let (Some(phone), Some(country_code)) = (phone, country_code) {
            payload["tel"] = Value::from(phone);
            payload["cid"] = Value::from(country_code);
        }

        let login_info = credential.login_by_sms(code, payload).await?;
        self.sms_payload = None;
        self.sms_recaptcha_context = None;
        Ok(login_info)
    }

    pub async fn get_qr_code(&mut self) -> Result<String> {
        self.reset_qr_session();

        let credential = Credential::new(self.proxy.as_deref());
        let qr_response = credential.get_qrcode().await?;
        let qr_url = qr_response["data"]["url"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let qr_code = QrCode::new(&qr_url)?;
        let image = qr_code.render::<image::Luma<u8>>().build();

        // 将图片转换为base64
        let mut buffer = Vec::new();
        {
            use image::codecs::png::PngEncoder;
            use image::{ExtendedColorType, ImageEncoder};

            let encoder = PngEncoder::new(&mut buffer);
            encoder.write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::L8,
            )?;
        }

        let base64_image = encode_base64(&buffer);
        let data_url = format!("data:image/png;base64,{base64_image}");

        let status = Arc::new(Mutex::new(QrLoginStatus::Pending));
        let handle = Self::spawn_qr_polling(status.clone(), credential, qr_response);
        self.qr_session = Some(QrSession { status, handle });

        Ok(data_url)
    }

    /// 二维码登录
    pub async fn check_qr_login(&self) -> Result<QrLoginCheckResult> {
        let status = match &self.qr_session {
            Some(session) => session.status.lock().await.clone(),
            None => return Ok(QrLoginCheckResult::Idle),
        };

        match status {
            QrLoginStatus::Pending => Ok(QrLoginCheckResult::Pending),
            QrLoginStatus::Success(login_info) => Ok(QrLoginCheckResult::Success(login_info)),
            QrLoginStatus::Expired(message) => Ok(QrLoginCheckResult::Expired(message)),
            QrLoginStatus::Error(message) => Ok(QrLoginCheckResult::Error(message)),
        }
    }

    fn spawn_qr_polling(
        status: Arc<Mutex<QrLoginStatus>>,
        credential: Credential,
        qr_response: Value,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            info!("{}", crate::tr!("auth.qr_polling_started"));

            let next_status = match credential.login_by_qrcode(qr_response).await {
                Ok(login_info) => QrLoginStatus::Success(login_info),
                Err(error) => {
                    let message = error.to_string();
                    if message.contains("86038") {
                        QrLoginStatus::Expired(crate::i18n::text("auth.qr_expired").to_string())
                    } else {
                        QrLoginStatus::Error(crate::tr!("auth.qr_login_failed", message = message))
                    }
                }
            };

            *status.lock().await = next_status;
        })
    }

    fn reset_qr_session(&mut self) {
        if let Some(session) = self.qr_session.take() {
            session.handle.abort();
        }
    }

    pub fn destroy(&mut self) {
        self.reset_qr_session();
        self.credential = None;
        self.sms_payload = None;
        self.sms_recaptcha_context = None;
        self.proxy = None;
    }

    pub async fn login_done_with_proxy(
        login_info: &LoginInfo,
        proxy: Option<&str>,
    ) -> Result<(BiliBili, User)> {
        info!("{}", crate::tr!("auth.user_fetch_started"));
        let bilibili = biliup::credential::bilibili_from_info(login_info.clone(), proxy)?;

        let myinfo = bilibili.my_info().await?;
        let data = myinfo
            .get("data")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.user_data_missing")))?;
        let username = data
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.user_name_missing")))?
            .to_owned();
        let uid = data
            .get("mid")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("auth.user_id_missing")))?;
        let avatar_url = data
            .get("face")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        debug!("{}", crate::tr!("avatar.url", url = avatar_url));

        let avatar = if avatar_url.is_empty() {
            String::new()
        } else {
            let avatar = bilibili
                .client
                .get(&avatar_url)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", crate::tr!("avatar.fetch_failed", error = e)))?
                .bytes()
                .await?;
            encode_base64(&avatar)
        };
        Ok((bilibili, User::new(uid, username, avatar, false)))
    }
}

pub async fn validate_cookie_in_old_config(cookie: &PathBuf) -> Result<(BiliBili, User)> {
    let bilibili = biliup::credential::login_by_cookies(cookie, None).await?;

    let myinfo = bilibili.my_info().await?;

    let uid = myinfo["data"]["mid"].as_u64().unwrap_or(0);

    if uid > 0 {
        let username = myinfo["data"]["name"].as_str().unwrap_or("").to_owned();
        let avatar_url = myinfo["data"]["face"].as_str().unwrap_or("").to_string();
        let avatar = bilibili
            .client
            .get(avatar_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}", crate::tr!("avatar.fetch_failed", error = e)))?
            .bytes()
            .await?;
        let avatar = encode_base64(&avatar);

        info!("{}", crate::tr!("auth.verifying_user", username = username));
        Ok((bilibili, User::new(uid, username, avatar, false)))
    } else {
        info!("{}", crate::tr!("auth.legacy_login_expired"));
        Ok((
            bilibili,
            User::new(
                uid,
                crate::i18n::text("auth.unknown_expired_user").to_owned(),
                String::new(),
                true,
            ),
        ))
    }
}
