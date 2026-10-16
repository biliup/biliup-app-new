use biliup::bilibili::BiliBili;
use serde::Deserialize;
use serde_json::Value;
use tauri::Manager;

use crate::{AppData, error::AppError, services::upload_risk::verify_delete_window};

static DELETE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const MEMBER: &str = "https://member.bilibili.com";

fn invalid() -> AppError {
    AppError::Custom(crate::i18n::text("delete.invalid_response").into())
}

fn api_result(value: &Value) -> Result<(), AppError> {
    match value.get("code").and_then(Value::as_i64) {
        Some(0) => Ok(()),
        Some(code) => Err(AppError::Biliup(format!(
            "{} ({code})",
            value
                .get("message")
                .or_else(|| value.get("msg"))
                .and_then(Value::as_str)
                .unwrap_or("")
        ))),
        None => Err(invalid()),
    }
}

async fn get_data(
    client: &reqwest::Client,
    url: &str,
    params: &[(&str, String)],
) -> Result<Value, AppError> {
    let value: Value = client
        .get(url)
        .query(params)
        .header(
            reqwest::header::REFERER,
            "https://member.bilibili.com/platform/upload-manager/article",
        )
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    api_result(&value)?;
    value.get("data").cloned().ok_or_else(invalid)
}

async fn prepare(client: &reqwest::Client, member: &str, aid: u64) -> Result<bool, AppError> {
    let data = get_data(
        client,
        &format!("{member}/x/vupre/web/archive/view"),
        &[("aid", aid.to_string())],
    )
    .await?;
    let archive = data.get("archive").unwrap_or(&data);
    if archive.get("aid").and_then(Value::as_u64) != Some(aid) {
        return Err(invalid());
    }
    let bvid = archive
        .get("bvid")
        .and_then(Value::as_str)
        .filter(|id| id.starts_with("BV"))
        .ok_or_else(invalid)?;
    let risk = get_data(
        client,
        &format!("{member}/x/risk/archive/del"),
        &[("platform", "web".into()), ("bvid", bvid.into())],
    )
    .await?;
    // 官网 data=true 直接验证极验，data=false 先验证短信；异常不放行。
    risk.as_bool().map(|allowed| !allowed).ok_or_else(invalid)
}

#[derive(Deserialize)]
struct DeletionProof {
    geetest_challenge: String,
    geetest_validate: String,
    geetest_seccode: String,
    success: u8,
    sms_code: Option<String>,
}

impl DeletionProof {
    fn parse(raw: &str, sms_required: bool) -> Result<Self, AppError> {
        let proof: Self = serde_json::from_str(raw).map_err(|_| invalid())?;
        if ![0, 1].contains(&proof.success)
            || [
                &proof.geetest_challenge,
                &proof.geetest_validate,
                &proof.geetest_seccode,
            ]
            .iter()
            .any(|value| value.trim().is_empty() || value.len() > 4096)
            || proof
                .sms_code
                .as_ref()
                .is_some_and(|code| code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()))
            || (sms_required && proof.sms_code.is_none())
        {
            return Err(invalid());
        }
        Ok(proof)
    }
}

async fn submit_delete(
    bili: &BiliBili,
    url: &str,
    aid: u64,
    proof: DeletionProof,
) -> Result<(), AppError> {
    let csrf = bili
        .login_info
        .cookie_info
        .get("cookies")
        .and_then(Value::as_array)
        .and_then(|cookies| cookies.iter().find(|cookie| cookie["name"] == "bili_jct"))
        .and_then(|cookie| cookie["value"].as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Custom(crate::i18n::text("season.manager.csrf_missing").into()))?;
    let mut form = vec![
        ("aid", aid.to_string()),
        ("csrf", csrf.to_owned()),
        ("geetest_challenge", proof.geetest_challenge),
        ("geetest_validate", proof.geetest_validate),
        ("geetest_seccode", proof.geetest_seccode),
        ("success", proof.success.to_string()),
    ];
    if let Some(code) = proof.sms_code {
        form.push(("sms_code", code));
    }
    // 删除请求只发送一次；连接或响应异常时提示核对结果，不自动重试。
    let response = bili
        .client
        .post(url)
        .header(
            reqwest::header::REFERER,
            "https://member.bilibili.com/platform/upload-manager/article",
        )
        .form(&form)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| AppError::Custom(crate::i18n::text("delete.uncertain").into()))?;
    let value: Value = response
        .error_for_status()
        .map_err(|_| AppError::Custom(crate::i18n::text("delete.uncertain").into()))?
        .json()
        .await
        .map_err(|_| AppError::Custom(crate::i18n::text("delete.uncertain").into()))?;
    api_result(&value)
}

#[tauri::command]
pub async fn delete_creator_video(
    app: tauri::AppHandle,
    uid: u64,
    aid: u64,
) -> Result<(), AppError> {
    if aid == 0 {
        return Err(invalid());
    }
    let _guard = DELETE_LOCK
        .try_lock()
        .map_err(|_| AppError::Custom(crate::i18n::text("delete.busy").into()))?;
    let client = app.state::<AppData>().get_client(uid).await?;
    let sms_required = prepare(&client.bilibili.client, MEMBER, aid).await?;
    let raw = verify_delete_window(
        &app,
        uid,
        &client.bilibili,
        client.proxy.as_deref(),
        sms_required,
    )
    .await
    .map_err(AppError::Internal)?;
    let proof = DeletionProof::parse(&raw, sms_required)?;
    app.state::<AppData>().get_client(uid).await?;
    submit_delete(
        &client.bilibili,
        &format!("{MEMBER}/x/web/archive/delete"),
        aid,
        proof,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    fn proof(sms: Option<&str>) -> String {
        json!({"geetest_challenge":"challenge&+", "geetest_validate":"validate&+",
            "geetest_seccode":"sdk-result&+", "success":1, "sms_code":sms})
        .to_string()
    }

    fn bili() -> BiliBili {
        BiliBili {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            login_info: serde_json::from_value(json!({
                "cookie_info":{"cookies":[{"name":"bili_jct","value":"test-csrf"}]},"sso":[],
                "token_info":{"access_token":"","expires_in":0,"mid":1,"refresh_token":""},"platform":"bilibili"
            })).unwrap(),
        }
    }

    async fn respond(socket: &mut tokio::net::TcpStream, value: Value) {
        crate::test_support::respond(socket, "200 OK", &value.to_string()).await;
    }

    #[test]
    fn proof_requires_complete_captcha_and_sms_when_requested() {
        assert!(DeletionProof::parse(&proof(None), false).is_ok());
        assert!(DeletionProof::parse(&proof(None), true).is_err());
        assert!(DeletionProof::parse(&proof(Some("123456")), true).is_ok());
        assert!(DeletionProof::parse(&proof(Some("abcdef")), true).is_err());
        for field in [
            "geetest_challenge",
            "geetest_validate",
            "geetest_seccode",
            "success",
        ] {
            let mut value: Value = serde_json::from_str(&proof(None)).unwrap();
            value.as_object_mut().unwrap().remove(field);
            assert!(DeletionProof::parse(&value.to_string(), false).is_err());
        }
        let mut value: Value = serde_json::from_str(&proof(None)).unwrap();
        value["success"] = json!(2);
        assert!(DeletionProof::parse(&value.to_string(), false).is_err());
    }

    #[tokio::test]
    async fn preflight_requires_sms_for_false_and_rejects_api_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let member = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for response in [
                json!({"code":0,"data":true}),
                json!({"code":0,"data":false}),
                json!({"code":21700,"message":"special restriction"}),
                json!({"code":0,"data":null}),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = crate::test_support::read_headers(&mut socket).await;
                assert!(request.starts_with("GET /x/vupre/web/archive/view?aid=123 "));
                respond(
                    &mut socket,
                    json!({"code":0,"data":{"archive":{"aid":123,"bvid":"BVtest"}}}),
                )
                .await;
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = crate::test_support::read_headers(&mut socket).await;
                assert!(request.starts_with("GET /x/risk/archive/del?platform=web&bvid=BVtest "));
                respond(&mut socket, response).await;
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert!(!prepare(&client, &member, 123).await.unwrap());
        assert!(prepare(&client, &member, 123).await.unwrap());
        assert!(
            prepare(&client, &member, 123)
                .await
                .unwrap_err()
                .to_string()
                .contains("21700")
        );
        assert!(prepare(&client, &member, 123).await.is_err());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn deletion_posts_exact_form_once_and_checks_result() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/x/web/archive/delete",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            for (sms, code) in [(false, 0), (true, -400)] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = crate::test_support::read_headers(&mut socket).await;
                assert!(request.starts_with("POST /x/web/archive/delete "));
                assert!(
                    request
                        .to_lowercase()
                        .contains("content-type: application/x-www-form-urlencoded")
                );
                let body = crate::test_support::read_body(&mut socket, &request).await;
                let form: std::collections::HashMap<_, _> =
                    url::form_urlencoded::parse(&body).into_owned().collect();
                assert_eq!(form["aid"], "123");
                assert_eq!(form["csrf"], "test-csrf");
                assert_eq!(form["geetest_challenge"], "challenge&+");
                assert_eq!(form["geetest_validate"], "validate&+");
                assert_eq!(form["geetest_seccode"], "sdk-result&+");
                assert_eq!(form["success"], "1");
                assert_eq!(form.len(), if sms { 7 } else { 6 });
                if sms {
                    assert_eq!(form["sms_code"], "123456");
                }
                respond(&mut socket, json!({"code":code,"message":"result"})).await;
            }
        });
        let bili = bili();
        assert!(
            submit_delete(
                &bili,
                &url,
                123,
                DeletionProof::parse(&proof(None), false).unwrap()
            )
            .await
            .is_ok()
        );
        assert!(
            submit_delete(
                &bili,
                &url,
                123,
                DeletionProof::parse(&proof(Some("123456")), true).unwrap()
            )
            .await
            .is_err()
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn disconnected_delete_reports_uncertainty_without_retry() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/x/web/archive/delete",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert!(
                crate::test_support::read_headers(&mut socket)
                    .await
                    .starts_with("POST ")
            );
            socket.shutdown().await.unwrap();
            drop(socket);
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                    .await
                    .is_err()
            );
        });
        let error = submit_delete(
            &bili(),
            &url,
            123,
            DeletionProof::parse(&proof(None), false).unwrap(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, AppError::Custom(_)));
        server.await.unwrap();
    }
}
