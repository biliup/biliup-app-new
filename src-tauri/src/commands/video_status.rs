use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;

use crate::{AppData, error::AppError};

fn invalid_response() -> AppError {
    AppError::Custom(crate::i18n::text("video.processing_invalid_response").into())
}

async fn request_data(
    client: &reqwest::Client,
    url: &str,
    query: &[(&str, String)],
) -> Result<Value, AppError> {
    let response: Value = client
        .get(url)
        .query(query)
        .header(reqwest::header::REFERER, "https://member.bilibili.com/")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    match response.get("code").and_then(Value::as_i64) {
        Some(0) => response
            .get("data")
            .filter(|data| data.is_object())
            .cloned()
            .ok_or_else(invalid_response),
        Some(code) => Err(AppError::Biliup(format!(
            "{} ({code})",
            response
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
        ))),
        None => Err(invalid_response()),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessingVideo {
    cid: u64,
    index: u32,
    xcode_state: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessingPage {
    total: u64,
    pn: u32,
    ps: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessingVideos {
    page: ProcessingPage,
    videos: Vec<ProcessingVideo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuditProcess {
    archive_audit: i64,
    business_audit: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessingAudit {
    aegis_state: i64,
    audit_process: Option<AuditProcess>,
}

#[derive(Debug, Serialize)]
pub struct ArchiveProcessing {
    bvid: String,
    #[serde(flatten)]
    videos: ProcessingVideos,
    audit: ProcessingAudit,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TranscodeResolution {
    resolution: String,
    status: String,
    #[serde(default)]
    estimated_time: f64,
    #[serde(default)]
    start_time: f64,
    #[serde(default)]
    time_now: f64,
    #[serde(default)]
    progress: Option<f64>,
    #[serde(default)]
    failure_reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TranscodeDetail {
    cid: u64,
    xcode_state: i64,
    #[serde(default)]
    xcode_tip: String,
    #[serde(default)]
    fail_tip: String,
    transcode_list: Option<Vec<TranscodeResolution>>,
}

#[tauri::command]
pub async fn get_archive_processing(
    app: tauri::AppHandle,
    uid: u64,
    aid: u64,
    page: u32,
) -> Result<ArchiveProcessing, AppError> {
    if aid == 0 || page == 0 {
        return Err(invalid_response());
    }
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    load_archive_processing(&bili.client, "https://member.bilibili.com", aid, page).await
}

async fn load_archive_processing(
    client: &reqwest::Client,
    member: &str,
    aid: u64,
    page: u32,
) -> Result<ArchiveProcessing, AppError> {
    let archive = request_data(
        client,
        &format!("{member}/x/vupre/web/archive/view"),
        &[("aid", aid.to_string())],
    )
    .await?;
    let bvid = archive
        .get("archive")
        .unwrap_or(&archive)
        .get("bvid")
        .and_then(Value::as_str)
        .filter(|id| id.starts_with("BV"))
        .ok_or_else(invalid_response)?
        .to_owned();
    let query = [("bvid", bvid.clone()), ("web_location", "0.0".into())];
    let mut list_query = query.to_vec();
    list_query.extend([
        ("mode", "0".into()),
        ("ps", "20".into()),
        ("pn", page.to_string()),
    ]);
    let videos_url = format!("{member}/x/web/detail/videos");
    let audit_url = format!("{member}/x/web/detail/audit");
    let (videos, audit) = tokio::try_join!(
        request_data(client, &videos_url, &list_query),
        request_data(client, &audit_url, &query)
    )?;
    Ok(ArchiveProcessing {
        bvid,
        videos: serde_json::from_value(videos).map_err(|_| invalid_response())?,
        audit: serde_json::from_value(audit).map_err(|_| invalid_response())?,
    })
}

#[tauri::command]
pub async fn get_video_transcode(
    app: tauri::AppHandle,
    uid: u64,
    bvid: String,
    cid: u64,
) -> Result<TranscodeDetail, AppError> {
    if !bvid.starts_with("BV") || cid == 0 {
        return Err(invalid_response());
    }
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    load_video_transcode(&bili.client, "https://member.bilibili.com", &bvid, cid).await
}

async fn load_video_transcode(
    client: &reqwest::Client,
    member: &str,
    bvid: &str,
    cid: u64,
) -> Result<TranscodeDetail, AppError> {
    let data = request_data(
        client,
        &format!("{member}/x/web/detail/xcode"),
        &[
            ("bvid", bvid.to_owned()),
            ("cid", cid.to_string()),
            ("web_location", "0.0".into()),
        ],
    )
    .await?;
    let detail: TranscodeDetail = serde_json::from_value(data).map_err(|_| invalid_response())?;
    if detail.cid != cid {
        return Err(invalid_response());
    }
    Ok(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn processing_reads_creator_parts_audit_and_resolution_progress() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let member = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for _ in 0..5 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let header = crate::test_support::read_headers(&mut socket).await;
                let url = reqwest::Url::parse(&format!(
                    "http://test{}",
                    header.split_whitespace().nth(1).unwrap()
                ))
                .unwrap();
                let params: std::collections::HashMap<_, _> =
                    url.query_pairs().into_owned().collect();
                assert!(header.starts_with("GET "));
                assert!(
                    header
                        .to_lowercase()
                        .contains("referer: https://member.bilibili.com/")
                );
                let data = match url.path() {
                    "/x/vupre/web/archive/view" => {
                        assert_eq!(params["aid"], "123");
                        json!({"archive": {"aid": 123, "bvid": "BVtest"}})
                    }
                    "/x/web/detail/videos" => {
                        assert_eq!(params["pn"], "2");
                        assert_eq!(params["ps"], "20");
                        assert_eq!(params["mode"], "0");
                        json!({"page": {"total": 21, "pn": 2, "ps": 1}, "videos": [{"cid": 101, "index": 21, "xcode_state": 1}]})
                    }
                    "/x/web/detail/audit" => json!({"aegis_state": 2, "audit_process": null}),
                    "/x/web/detail/xcode" => {
                        json!({"cid": 101, "xcode_state": 1, "xcode_tip": "4K", "fail_tip": "", "transcode_list": [
                            {"resolution": "360P", "status": "success", "estimated_time": 0, "start_time": 0, "time_now": 120},
                            {"resolution": "4K", "status": "processing", "estimated_time": 200, "start_time": 100, "time_now": 120}
                        ]})
                    }
                    _ => panic!("unexpected request"),
                };
                if url.path() != "/x/vupre/web/archive/view" {
                    assert_eq!(params["bvid"], "BVtest");
                    assert_eq!(params["web_location"], "0.0");
                }
                let body = json!({"code": 0, "data": data}).to_string();
                crate::test_support::respond(&mut socket, "200 OK", &body).await;
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let result = load_archive_processing(&client, &member, 123, 2)
            .await
            .unwrap();
        assert_eq!(result.videos.page.total, 21);
        assert_eq!(result.videos.videos[0].index, 21);
        assert_eq!(result.audit.aegis_state, 2);
        assert!(result.audit.audit_process.is_none());
        let detail = load_video_transcode(&client, &member, "BVtest", 101)
            .await
            .unwrap();
        assert_eq!(detail.xcode_state, 1);
        assert_eq!(detail.transcode_list.unwrap()[1].time_now, 120.0);
        assert!(
            load_video_transcode(&client, &member, "BVtest", 999)
                .await
                .is_err()
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn processing_rejects_api_errors_missing_data_and_http_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for (status, body) in [
                ("200 OK", r#"{"code":-101,"message":"not logged in"}"#),
                ("200 OK", r#"{"code":0,"data":null}"#),
                ("200 OK", r#"{"data":{}}"#),
                ("429 Too Many Requests", "<html>429</html>"),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                crate::test_support::read_headers(&mut socket).await;
                crate::test_support::respond(&mut socket, status, &body).await;
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for _ in 0..4 {
            assert!(request_data(&client, &url, &[]).await.is_err());
        }
        server.await.unwrap();
    }
}
