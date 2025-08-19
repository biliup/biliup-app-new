use std::path::PathBuf;

use crate::{models::User, utils::crypto::encode_base64};
use anyhow::Result;
use biliup::{
    bilibili::BiliBili,
    credential::{Credential, LoginInfo},
};
use qrcode::QrCode;
use serde_json::Value;
use tracing::{debug, info};

pub struct AuthService {
    credential: Option<Credential>,
    qr_key: Option<Value>,
    proxy: Option<String>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            credential: None,
            qr_key: None,
            proxy: None,
        }
    }

    pub fn init(&mut self, proxy: Option<&str>) {
        self.proxy = proxy.map(String::from);
        self.credential = Some(Credential::new(proxy));
    }

    pub fn get_proxy(&self) -> Option<String> {
        self.proxy.clone()
    }

    /// 用户名密码登录
    pub async fn login_with_username_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(BiliBili, User)> {
        let credential = self
            .credential
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("AuthService 未初始化"))?;

        let user = credential
            .login_by_password(username, password)
            .await
            .unwrap_or_else(|e| {
                info!("登录失败: {}", e);
                // User::default()
                // e = JSON.parse(e);
                // {"code":0,"data":{"cookie_info":null,"message":
                // "本次登录环境存在风险, 需使用手机号进行验证或绑定",
                // "sso":null,"status":2,
                // "token_info":null,
                // "url":"https://passport.bilibili.com/account/mobile/security/managephone/phone/verify?tmp_token=&requestId=&source=risk"},
                // "message":"0","ttl":1}
                // const webview = new WebviewWindow('theUniqueLabel', {
                //     url: e.data.url
                // })
                panic!("登录失败") // 这里可以根据需要返回默认用户或错误
            });
        // let login_info = credential.login_by_cookies(user.cookie_info).await?;
        info!("{}", serde_json::to_string(&user.cookie_info)?);

        // let user_name = user.

        // Ok(User::default())
        todo!("实现用户名密码登录")
    }

    pub async fn get_qr_code(&mut self) -> Result<String> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AuthService 未初始化"))?;

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

        self.qr_key = Some(qr_response);

        Ok(data_url)
    }

    /// 二维码登录
    pub async fn qr_login(&self) -> Result<(BiliBili, User)> {
        info!("开始二维码登录");

        match self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AuthService 未初始化"))?
            .login_by_qrcode(self.qr_key.clone().unwrap())
            .await
        {
            Err(e) => Err(anyhow::anyhow!("二维码登录失败: {}", e)),
            Ok(login_info) => self.login_done(&login_info).await,
        }
    }

    /// Cookie 登录
    pub async fn _cookie_login(&self, _cookie: &str) -> Result<(BiliBili, User)> {
        // TODO: 实现 Cookie 登录验证
        todo!("实现 Cookie 登录")
    }

    pub fn destroy(&mut self) {
        self.credential = None;
        self.qr_key = None;
        self.proxy = None;
    }

    async fn login_done(&self, login_info: &LoginInfo) -> Result<(BiliBili, User)> {
        info!("登录完成，开始获取用户信息");
        let bilibili =
            biliup::credential::bilibili_from_info(login_info.clone(), self.proxy.as_deref())?;

        let myinfo = bilibili.my_info().await?;
        let username = myinfo["data"]["name"].as_str().unwrap().to_owned();
        let uid = myinfo["data"]["mid"].as_u64().unwrap_or(0);
        let avatar_url = myinfo["data"]["face"].as_str().unwrap_or("").to_string();
        debug!("头像 URL: {}", avatar_url);

        let avatar = bilibili
            .client
            .get(&avatar_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("获取用户头像失败: {}", e))?
            .bytes()
            .await?;
        let avatar = encode_base64(&avatar);
        debug!("头像 URL: {}", avatar_url);
        Ok((bilibili, User::new(uid, username, avatar)))
    }
}

pub async fn validate_cookie_in_old_config(cookie: &PathBuf) -> Result<(BiliBili, User)> {
    let bilibili = biliup::credential::login_by_cookies(cookie, None).await?;

    let myinfo = bilibili.my_info().await?;

    let user_name = myinfo["data"]["name"].as_str().unwrap().to_owned();
    let uid = myinfo["data"]["mid"].as_u64().unwrap_or(0);
    let avatar_url = myinfo["data"]["face"].as_str().unwrap_or("").to_string();

    info!("验证用户: {}", user_name);
    Ok((bilibili, User::new(uid, user_name, avatar_url)))
}
