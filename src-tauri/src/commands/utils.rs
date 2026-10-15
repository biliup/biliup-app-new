use biliup::{
    bilibili::{BiliBili, Vid},
    uploader::season::{Episode, EpisodeAdd, SeasonApi, SeasonEntry},
};
use futures::{StreamExt, stream};
use serde_json::{Value, json};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use std::{fs::File, io::Read, path::Path};
use tauri::Manager;
use tokio::sync::Mutex;

use tracing::{debug, error, info, warn};

use crate::{AppData, error::AppError, models::TemplateConfig};
use crate::{models::user_config::Credit, utils::crypto::encode_base64};
use crate::{
    models::user_config::Staff,
    utils::{
        file_utils::{self, FileEntry},
        get_avatar_cache_path,
    },
};

#[derive(serde::Serialize)]
pub struct MentionUserItem {
    face: String,
    fans: u64,
    name: String,
    official_verify_type: i64,
    uid: String,
}

#[derive(serde::Serialize)]
pub struct MentionUserGroup {
    group_name: String,
    group_type: i64,
    items: Vec<MentionUserItem>,
}

fn extract_avatar_filename(face_url: &str, uid: &str) -> String {
    let raw_name = face_url
        .split('?')
        .next()
        .unwrap_or(face_url)
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim();

    if raw_name.is_empty() {
        return format!("{uid}.jpg");
    }

    // 仅保留文件名安全字符，避免路径穿越
    let safe_name: String = raw_name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '_' || *ch == '-')
        .collect();

    if safe_name.is_empty() {
        format!("{uid}.jpg")
    } else {
        safe_name
    }
}

fn normalize_face_url(face_url: &str) -> String {
    if face_url.starts_with("//") {
        format!("https:{face_url}")
    } else if face_url.starts_with("http://") {
        face_url.replacen("http://", "https://", 1)
    } else {
        face_url.to_string()
    }
}

fn normalize_desc_v2_tokens(tokens: Vec<Credit>) -> Vec<Credit> {
    let mut normalized: Vec<Credit> = Vec::with_capacity(tokens.len());

    for token in tokens {
        if let Some(last) = normalized.last_mut()
            && last.r#type == 1
            && token.r#type == 1
        {
            last.raw_text.push_str(&token.raw_text);
            continue;
        }

        normalized.push(token);
    }

    normalized
}

/// 补齐创作者 API 缺少的投稿设置；缺失、null 或类型异常时保留原值，0/false 正常覆盖。
fn apply_web_video_settings(template: &mut TemplateConfig, data: &Value) {
    let archive = data.get("archive").unwrap_or(data);

    // 新版分区由 Web 接口的 human_type2.id 提供，不覆盖旧版分区 tid。
    if let Some(id) = archive["human_type2"]["id"]
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
    {
        template.tid_v2 = id;
    }

    // 杜比和无损音质优先使用 archive 顶层开关，缺失时兼容 attrs 中的同名字段。
    let audio_flag = |key: &str| {
        archive
            .get(key)
            .and_then(Value::as_u64)
            .or_else(|| archive["attrs"][key].as_u64())
            .filter(|value| *value <= 1)
            .map(|value| value as u8)
    };
    if let Some(dolby) = audio_flag("is_dolby") {
        template.dolby = dolby;
    }
    if let Some(lossless_music) = audio_flag("lossless_music") {
        template.lossless_music = lossless_music;
    }

    // data.subtitle.allow 对应字幕开关，同时同步内部 subtitle，lan 保留接口语言值。
    if let Some(allow) = data["subtitle"]["allow"].as_bool() {
        template.open_subtitle = allow;
        template.subtitle.open = u8::from(allow);
    }
    if let Some(lan) = data["subtitle"]["lan"].as_str() {
        template.subtitle.lan = lan.to_string();
    }
    // 水印状态和精选评论均位于 data 层，不在 archive 内；false 必须清除原来的开启值。
    if let Some(state) = data["watermark"]["state"]
        .as_u64()
        .filter(|state| *state <= 1)
    {
        template.watermark = state as u8;
    }
    if let Some(selection) = data["reply"]["up_selection"].as_bool() {
        template.up_selection_reply = u8::from(selection);
    }
}

#[tauri::command]
pub async fn get_avatar_cache_dir() -> Result<String, AppError> {
    Ok(get_avatar_cache_path()
        .map_err(AppError::Internal)?
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub async fn get_current_version() -> Result<String, AppError> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// 获取文件大小
#[tauri::command]
pub async fn get_file_size(file_path: String) -> Result<u64, AppError> {
    let path = Path::new(&file_path);
    Ok(file_utils::get_file_size(path).map_err(AppError::Internal)?)
}

/// 递归读取目录
#[tauri::command]
pub async fn read_dir_recursive(
    dir_path: String,
    include_subdirs: bool,
    max_depth: Option<u32>,
) -> Result<Vec<FileEntry>, AppError> {
    let path = Path::new(&dir_path);
    Ok(
        file_utils::read_dir_recursive(path, include_subdirs, max_depth)
            .map_err(AppError::Internal)?,
    )
}

/// 上传封面并进行返回url
#[tauri::command]
pub async fn upload_cover(
    app: tauri::AppHandle,
    uid: u64,
    file: String,
) -> Result<String, AppError> {
    let app_data = app.state::<AppData>();

    let mut cover_file = File::open(file)?;
    let mut cover_buf = vec![];

    cover_file.read_to_end(&mut cover_buf)?;

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or_else(|| AppError::UserNotFound(uid))?
        .bilibili
        .cover_up(&cover_buf)
        .await
    {
        Ok(url) => {
            info!("{}", crate::tr!("cover.uploaded", url = url));
            Ok(url)
        }
        Err(e) => Err(AppError::Internal(anyhow::anyhow!("{}", e))),
    }
}

/// 下载封面并进行base64编码
#[tauri::command]
pub async fn download_cover(
    app: tauri::AppHandle,
    uid: u64,
    url: String,
) -> Result<String, AppError> {
    let app_data = app.state::<AppData>();

    let res = app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or_else(|| AppError::UserNotFound(uid))?
        .bilibili
        .client
        .get(&url)
        .send()
        .await?;
    let bytes = res.bytes().await?;
    Ok(encode_base64(&bytes))
}

#[tauri::command]
pub async fn get_archive_pre(app: tauri::AppHandle, uid: u64) -> Result<Value, AppError> {
    let app_data = app.state::<AppData>();
    let bilibili = app_data.get_bilibili(uid).await?;

    let archive_pre_res = bilibili
        .archive_pre()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
    let mut archive_pre_data = archive_pre_res["data"].clone();

    let type2_url = format!(
        "https://member.bilibili.com/x/vupre/web/archive/human/type2/list?t={}",
        chrono::Utc::now().timestamp(),
    );

    let type2_res = bilibili.client.get(type2_url).send().await;
    match type2_res {
        Ok(response) => match response.json::<Value>().await {
            Ok(payload) => {
                let code = payload["code"].as_i64().unwrap_or(-1);
                if code == 0 {
                    archive_pre_data["type_list_v2"] = payload["data"]["type_list"].clone();
                } else {
                    warn!(
                        "{}",
                        crate::tr!(
                            "category.unexpected_response",
                            code = code,
                            payload = payload
                        )
                    );
                }
            }
            Err(e) => {
                warn!("{}", crate::tr!("category.parse_failed", error = e));
            }
        },
        Err(e) => {
            warn!("{}", crate::tr!("category.fetch_failed", error = e));
        }
    }

    Ok(archive_pre_data)
}

#[tauri::command]
pub async fn get_topic_list(app: tauri::AppHandle, uid: u64) -> Result<Value, AppError> {
    let app_data = app.state::<AppData>();

    let bilibili = app_data.get_bilibili(uid).await?;

    match bilibili
        .client
        .get("https://member.bilibili.com/x/vupre/web/topic/type?pn=0&ps=999")
        .send()
        .await?
        .json::<Value>()
        .await
    {
        Ok(res) => {
            // debug!("获取话题列表成功: {}", res);
            Ok(res["data"]["topics"].clone())
        }
        Err(e) => Err(AppError::Internal(anyhow::anyhow!("{}", e))),
    }
}

#[tauri::command]
pub async fn search_topics(
    app: tauri::AppHandle,
    uid: u64,
    query: String,
) -> Result<Value, AppError> {
    let app_data = app.state::<AppData>();

    let bilibili = app_data.get_bilibili(uid).await?;

    match bilibili
        .client
        .get("https://member.bilibili.com/x/vupre/web/topic/search")
        .query(&[
            ("keywords", &query),
            ("page_size", &"50".to_string()),
            ("offset", &"0".to_string()),
            ("t", &chrono::Utc::now().timestamp().to_string()),
        ])
        .send()
        .await?
        .json::<Value>()
        .await
    {
        Ok(res) => {
            debug!("{}", crate::tr!("topic.search_completed", response = res));
            Ok(res["data"]["result"]["topics"].clone())
        }
        Err(e) => Err(AppError::Internal(anyhow::anyhow!("{}", e))),
    }
}

#[tauri::command]
pub async fn search_mention(
    app: tauri::AppHandle,
    uid: u64,
    keyword: Option<String>,
) -> Result<Vec<MentionUserGroup>, AppError> {
    let app_data = app.state::<AppData>();
    let client = app_data.get_bilibili(uid).await?.client.clone();

    let mut request = client
        .get("https://api.bilibili.com/x/polymer/web-dynamic/v1/mention/search")
        .query(&[("uid", uid.to_string())]);

    if let Some(keyword) = keyword
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        request = request.query(&[("keyword", keyword)]);
    }

    let res = request.send().await?.json::<Value>().await?;

    if res["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(AppError::Biliup(
            res["message"]
                .as_str()
                .unwrap_or(crate::i18n::text("mention.search_failed"))
                .to_string(),
        ));
    }

    let groups = res["data"]["groups"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut avatar_jobs: Vec<(String, String)> = Vec::new();

    let parsed_groups = groups
        .iter()
        .map(|group| {
            let items = group["items"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|item| {
                    let uid = item["uid"].as_str().unwrap_or("0").to_string();
                    let face_url = normalize_face_url(item["face"].as_str().unwrap_or(""));
                    let face = extract_avatar_filename(&face_url, &uid);

                    if !face_url.is_empty() {
                        avatar_jobs.push((face_url, face.clone()));
                    }

                    MentionUserItem {
                        face,
                        fans: item["fans"].as_u64().unwrap_or(0),
                        name: item["name"].as_str().unwrap_or("").to_string(),
                        official_verify_type: item["official_verify_type"].as_i64().unwrap_or(-1),
                        uid,
                    }
                })
                .collect::<Vec<_>>();

            MentionUserGroup {
                group_name: group["group_name"]
                    .as_str()
                    .unwrap_or(crate::i18n::text("common.other"))
                    .to_string(),
                group_type: group["group_type"].as_i64().unwrap_or(0),
                items,
            }
        })
        .collect::<Vec<_>>();

    let download_client = client.clone();
    tokio::spawn(async move {
        debug!("{}", crate::tr!("avatar.download_started"));
        let cache_dir = match get_avatar_cache_path() {
            Ok(path) => path,
            Err(e) => {
                error!("{}", crate::tr!("avatar.cache_dir_failed", error = e));
                return;
            }
        };

        for (face_url, file_name) in avatar_jobs {
            let save_path = cache_dir.join(&file_name);

            // 命中近 1 天缓存时直接复用，减少重复下载请求
            let is_fresh_cache = match tokio::fs::metadata(&save_path).await {
                Ok(meta) => match meta.modified() {
                    Ok(modified_at) => match SystemTime::now().duration_since(modified_at) {
                        Ok(elapsed) => elapsed < Duration::from_secs(24 * 60 * 60),
                        Err(_) => false,
                    },
                    Err(_) => false,
                },
                Err(_) => false,
            };

            if is_fresh_cache {
                continue;
            }

            match download_client.get(&face_url).send().await {
                Ok(response) => match response.bytes().await {
                    Ok(bytes) => {
                        if let Err(e) = tokio::fs::write(&save_path, bytes).await {
                            warn!(
                                "{}",
                                crate::tr!(
                                    "avatar.cache_write_failed",
                                    uid = face_url,
                                    path = save_path.display(),
                                    error = e
                                )
                            );
                        }
                    }
                    Err(e) => warn!(
                        "{}",
                        crate::tr!("avatar.response_failed", uid = face_url, error = e)
                    ),
                },
                Err(e) => warn!(
                    "{}",
                    crate::tr!("avatar.download_failed", uid = face_url, error = e)
                ),
            }
        }
        debug!("{}", crate::tr!("avatar.download_finished"));
    });

    Ok(parsed_groups)
}

#[tauri::command]
pub async fn get_season_list(app: tauri::AppHandle, uid: u64) -> Result<Value, AppError> {
    let app_data = app.state::<AppData>();
    let bilibili = app_data.get_bilibili(uid).await?;
    let seasons = load_seasons(&bilibili.season_api()).await?;
    let seasons: Vec<Value> = seasons
        .into_iter()
        .map(|entry| {
            let sections: Vec<Value> = entry
                .sections
                .sections
                .into_iter()
                .map(|section| {
                    json!({
                        "section_id": section.id,
                        "title": if section.title.is_empty() { &entry.season.title } else { &section.title },
                    })
                })
                .collect();
            json!({
                "season_id": entry.season.id,
                "section_id": sections.first().map(|section| &section["section_id"]),
                "title": entry.season.title,
                "sections": sections,
            })
        })
        .collect();
    Ok(json!({ "seasons": seasons }))
}

pub(super) async fn load_seasons(api: &SeasonApi<'_>) -> Result<Vec<SeasonEntry>, AppError> {
    let mut seasons = Vec::new();
    for page_number in 1.. {
        let page = api
            .seasons(page_number, 50)
            .await
            .map_err(|e| AppError::Biliup(e.to_string()))?;
        let page_len = page.seasons.len();
        seasons.extend(page.seasons);
        if page_len < 50
            || page
                .total
                .is_some_and(|total| seasons.len() as u64 >= total)
        {
            break;
        }
    }
    Ok(seasons)
}

async fn video_season_id(bilibili: &BiliBili, member: &str, aid: u64) -> Result<u64, AppError> {
    let response: Value = bilibili
        .client
        .get(format!("{member}/x2/creative/web/season/aid"))
        .query(&[("id", aid), ("t", chrono::Utc::now().timestamp() as u64)])
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    match response["code"].as_i64() {
        Some(0) => {}
        Some(-404) => return Ok(0),
        Some(code) => {
            return Err(AppError::Biliup(format!(
                "查询稿件合集失败：code {code}，{}",
                response["message"].as_str().unwrap_or_default()
            )));
        }
        None => return Err(AppError::Custom("稿件合集响应缺少有效的 code".to_string())),
    }
    match response.get("data") {
        Some(Value::Null) => Ok(0),
        Some(data) => data["id"]
            .as_u64()
            .ok_or_else(|| AppError::Custom("稿件合集响应缺少有效的合集 ID".to_string())),
        None => Err(AppError::Custom("稿件合集响应缺少 data".to_string())),
    }
}

async fn find_video_season(
    api: &SeasonApi<'_>,
    seasons: &[SeasonEntry],
    aid: u64,
    season_id: u64,
) -> Result<Option<(u64, u64, Episode)>, AppError> {
    if season_id == 0 {
        return Ok(None);
    }
    let entry = seasons
        .iter()
        .find(|entry| entry.season.id == season_id)
        .ok_or_else(|| AppError::Custom("稿件所属合集不在合集列表中，请刷新后重试".to_string()))?;
    for section in &entry.sections.sections {
        let detail = api
            .season_section(section.id, Some("desc"))
            .await
            .map_err(|e| AppError::Biliup(e.to_string()))?;
        if let Some(episode) = detail
            .episodes
            .into_iter()
            .find(|episode| episode.aid == aid)
        {
            return Ok(Some((entry.season.id, section.id, episode)));
        }
    }
    Err(AppError::Custom(
        "未在所属合集的小节中找到稿件，请刷新后重试".to_string(),
    ))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatorVideo {
    aid: u64,
    #[serde(default)]
    bvid: String,
    title: String,
    #[serde(default)]
    cover: String,
    #[serde(default)]
    season_add_state: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct CreatorVideoPage {
    items: Vec<CreatorVideo>,
    total: u64,
}

type CandidateSnapshot = Arc<Mutex<Option<Vec<CreatorVideo>>>>;

#[derive(Default)]
pub struct CreatorVideoSnapshots {
    next_id: AtomicU64,
    entries: Mutex<HashMap<(u64, String), CandidateSnapshot>>,
}

impl CreatorVideoSnapshots {
    async fn open(&self, uid: u64) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        self.entries
            .lock()
            .await
            .insert((uid, id.clone()), Arc::default());
        id
    }

    async fn close(&self, uid: u64, id: String) {
        self.entries.lock().await.remove(&(uid, id));
    }

    async fn page(
        &self,
        client: &reqwest::Client,
        member: &str,
        uid: u64,
        id: String,
        page: u32,
        page_size: u32,
    ) -> Result<CreatorVideoPage, AppError> {
        let snapshot = self
            .entries
            .lock()
            .await
            .get(&(uid, id))
            .cloned()
            .ok_or_else(|| {
                AppError::Custom(crate::i18n::text("video.list_snapshot_expired").into())
            })?;
        // 同一弹窗的并发翻页共享一次初始化；关闭只移除入口，不会被旧请求重新创建。
        let mut videos = snapshot.lock().await;
        if videos.is_none() {
            *videos = Some(load_season_candidates(client, member).await?);
        }
        Ok(candidate_page(videos.as_ref().unwrap(), page, page_size))
    }
}

#[tauri::command]
pub async fn open_video_list_snapshot(app: tauri::AppHandle, uid: u64) -> Result<String, AppError> {
    app.state::<AppData>().get_bilibili(uid).await?;
    Ok(app.state::<CreatorVideoSnapshots>().open(uid).await)
}

#[tauri::command]
pub async fn close_video_list_snapshot(app: tauri::AppHandle, uid: u64, snapshot_id: String) {
    app.state::<CreatorVideoSnapshots>()
        .close(uid, snapshot_id)
        .await;
}

fn parse_video_list(response: Value) -> Result<CreatorVideoPage, AppError> {
    let invalid = || AppError::Custom(crate::i18n::text("video.list_invalid_response").into());
    match response["code"].as_i64() {
        Some(0) => {}
        Some(code) => {
            return Err(AppError::Custom(crate::tr!(
                "video.list_api_failed",
                code = code,
                message = response["message"].as_str().unwrap_or_default()
            )));
        }
        None => return Err(invalid()),
    }
    let data = &response["data"];
    let total = data["page"]["count"].as_u64().ok_or_else(invalid)?;
    let entries = match data["arc_audits"].as_array() {
        Some(entries) => entries.as_slice(),
        None if total == 0 && data["arc_audits"].is_null() => &[],
        None => return Err(invalid()),
    };
    let items = entries
        .iter()
        .map(|entry| {
            let mut video: CreatorVideo =
                serde_json::from_value(entry["Archive"].clone()).map_err(|_| invalid())?;
            if video.aid == 0 {
                return Err(invalid());
            }
            // 此字段在 arc_audits 条目上，不在 Archive 内；缺失时不能视为可添加。
            video.season_add_state = entry["season_add_state"].as_i64();
            // 列表接口的封面只有相对路径，补齐图片域名。
            video.cover = if video.cover.is_empty()
                || video.cover.starts_with("http://")
                || video.cover.starts_with("https://")
            {
                video.cover
            } else if video.cover.starts_with("//") {
                format!("http:{}", video.cover)
            } else {
                format!(
                    "http://i1.hdslb.com/{}",
                    video.cover.trim_start_matches('/')
                )
            };
            Ok(video)
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(CreatorVideoPage { items, total })
}

async fn download_video_covers(client: &reqwest::Client, page: &mut CreatorVideoPage) {
    stream::iter(page.items.iter_mut())
        .for_each_concurrent(4, |video| async move {
            let url = std::mem::take(&mut video.cover);
            if url.is_empty() {
                return;
            }
            let result = async {
                let bytes = client
                    .get(&url)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;
                Ok::<_, reqwest::Error>(encode_base64(&bytes))
            }
            .await;
            match result {
                Ok(cover) => video.cover = cover,
                Err(error) => warn!(aid = video.aid, %error, "下载稿件封面失败"),
            }
        })
        .await;
}

async fn fetch_video_list_page(
    client: &reqwest::Client,
    member: &str,
    page: u32,
    page_size: u32,
) -> Result<CreatorVideoPage, AppError> {
    let response = client
        .get(format!("{member}/x2/creative/web/archives/sp"))
        .header(
            "Referer",
            "https://member.bilibili.com/platform/upload-manager/article",
        )
        .query(&[("pn", page), ("ps", page_size)])
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    parse_video_list(response)
}

async fn load_creator_video_page(
    client: &reqwest::Client,
    member: &str,
    page: u32,
    page_size: u32,
    season_add_only: bool,
) -> Result<CreatorVideoPage, AppError> {
    if !season_add_only {
        return fetch_video_list_page(client, member, page, page_size).await;
    }
    let eligible = load_season_candidates(client, member).await?;
    Ok(candidate_page(&eligible, page, page_size))
}

async fn load_season_candidates(
    client: &reqwest::Client,
    member: &str,
) -> Result<Vec<CreatorVideo>, AppError> {
    // 上游按全部稿件分页：先取完整列表并筛选，再按可添加稿件计算分页和总数。
    let mut eligible = Vec::new();
    let mut received = 0_u64;
    let mut batch_size = 300;
    for upstream_page in 1.. {
        let batch = fetch_video_list_page(client, member, upstream_page, batch_size).await?;
        let count = batch.items.len() as u64;
        received += count;
        // 若上游限制每批数量，后续页采用实际批量，避免按较大步长漏查。
        if upstream_page == 1
            && count > 0
            && count < u64::from(batch_size)
            && received < batch.total
        {
            batch_size = count as u32;
        }
        eligible.extend(
            batch
                .items
                .into_iter()
                .filter(|video| video.season_add_state == Some(0)),
        );
        if count == 0 || received >= batch.total {
            break;
        }
    }
    Ok(eligible)
}

fn candidate_page(eligible: &[CreatorVideo], page: u32, page_size: u32) -> CreatorVideoPage {
    let total = eligible.len() as u64;
    let offset = usize::try_from(u64::from(page - 1) * u64::from(page_size)).unwrap_or(usize::MAX);
    let items = eligible
        .iter()
        .skip(offset)
        .take(page_size as usize)
        .cloned()
        .collect();
    CreatorVideoPage { items, total }
}

#[tauri::command]
pub async fn get_video_list(
    app: tauri::AppHandle,
    uid: u64,
    page: u32,
    page_size: u32,
    season_add_only: Option<bool>,
    snapshot_id: Option<String>,
) -> Result<CreatorVideoPage, AppError> {
    if page == 0 || !(1..=100).contains(&page_size) {
        return Err(AppError::Custom(
            crate::i18n::text("video.list_invalid_page").into(),
        ));
    }
    let bilibili = app.state::<AppData>().get_bilibili(uid).await?;
    let mut page = if let Some(id) = snapshot_id.filter(|_| season_add_only.unwrap_or(false)) {
        app.state::<CreatorVideoSnapshots>()
            .page(
                &bilibili.client,
                "https://member.bilibili.com",
                uid,
                id,
                page,
                page_size,
            )
            .await?
    } else {
        load_creator_video_page(
            &bilibili.client,
            "https://member.bilibili.com",
            page,
            page_size,
            season_add_only.unwrap_or(false),
        )
        .await?
    };
    // 只下载最终显示的这一页封面。
    download_video_covers(&bilibili.client, &mut page).await;
    Ok(page)
}

#[tauri::command]
pub async fn get_video_detail(
    app: tauri::AppHandle,
    uid: u64,
    video_id: String,
) -> Result<TemplateConfig, AppError> {
    let vid = biliup::uploader::bilibili::Vid::from_str(&video_id)
        .map_err(|e| AppError::Custom(crate::tr!("video.id_parse_failed", error = e)))?;

    let app_data = app.state::<AppData>();

    let (bilibili, proxy) = {
        let proxy = app_data
            .config
            .lock()
            .await
            .config
            .get(&uid)
            .and_then(|c| c.proxy.clone());
        let bilibili = app_data.get_bilibili(uid).await?;
        (bilibili, proxy)
    };

    // 第1步：通过创作者 API 获取基础 TemplateConfig
    let res = bilibili
        .video_data(&vid, proxy.as_deref())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;
    let mut template_config = TemplateConfig::from_bilibili_res(res)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

    match bilibili
        .client
        .get(format!(
            "https://member.bilibili.com/x/vupre/web/archive/view?topic_grey=1&{vid}&t={}",
            chrono::Utc::now().timestamp() * 1000
        ))
        .send()
        .await
    {
        Ok(response) => match response.json::<Value>().await {
            Ok(res) => {
                // debug!("获取稿件 web 接口数据成功: {}", res);
                if let Some(data) = res.get("data").filter(|_| res["code"].as_i64() == Some(0)) {
                    let archive_data = data.get("archive").unwrap_or(data);
                    apply_web_video_settings(&mut template_config, data);

                    if let Some(desc) = archive_data["desc"].as_str().filter(|s| !s.is_empty()) {
                        template_config.desc = desc.to_string();
                    }
                    template_config.state = archive_data["state"].as_i64();
                    template_config.state_desc = archive_data["state_desc"]
                        .as_str()
                        .map(|value| value.to_string())
                        .or_else(|| data["state_desc"].as_str().map(|value| value.to_string()));
                    if !archive_data["desc_v2"].is_null() {
                        match serde_json::from_value::<Vec<Credit>>(archive_data["desc_v2"].clone())
                        {
                            Ok(credits) => {
                                let cleaned_credits = normalize_desc_v2_tokens(credits);
                                if !cleaned_credits.is_empty() {
                                    template_config.desc_v2 = Some(cleaned_credits);
                                }
                            }
                            Err(e) => {
                                warn!("{}", crate::tr!("video.desc_v2_parse_failed", error = e));
                            }
                        }
                    }
                    if let Some(dynamic) =
                        archive_data["dynamic"].as_str().filter(|s| !s.is_empty())
                    {
                        template_config.dynamic = dynamic.to_string();
                    }
                    // mission_id：活动 ID
                    if let Some(mission_id) = archive_data["mission_id"].as_u64() {
                        template_config.mission_id = Some(mission_id as u32);
                    }
                    if let Some(topic_id) = archive_data["topic_id"].as_u64() {
                        template_config.topic_id = Some(topic_id as u32);
                    }
                    if let Some(topic_name) = archive_data["topic_name"]
                        .as_str()
                        .or_else(|| data["topic_name"].as_str())
                        .filter(|s| !s.is_empty())
                    {
                        template_config.topic_name = Some(topic_name.to_string());
                    }
                    if let Some(rights) = archive_data.get("rights").or_else(|| data.get("rights"))
                    {
                        template_config.is_360 =
                            rights.get("is_360").and_then(|v| v.as_i64()).unwrap_or(-1);
                    }
                    if let Some(staff) = data
                        .get("staffs")
                        .or_else(|| archive_data.get("staffs"))
                        .or_else(|| data.get("staff"))
                    {
                        if let Some(arr) = staff.as_array() {
                            let staff_vec: Vec<Staff> = arr
                                .iter()
                                .filter_map(|item| {
                                    let title = item
                                        .get("apply_title")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| item.get("title").and_then(|v| v.as_str()))
                                        .unwrap_or("")
                                        .to_string();
                                    let mid = item
                                        .get("apply_staff_mid")
                                        .and_then(|v| v.as_u64())
                                        .or_else(|| item.get("mid").and_then(|v| v.as_u64()))
                                        .unwrap_or(0);
                                    if title.is_empty() || mid == 0 {
                                        None
                                    } else {
                                        Some(Staff {
                                            title,
                                            mid,
                                            is_del: 0,
                                        })
                                    }
                                })
                                .collect();
                            if !staff_vec.is_empty() {
                                template_config.staff = Some(staff_vec);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "{}",
                    crate::tr!(
                        "video.web_response_parse_failed",
                        error = format!("{:?}", e)
                    )
                );
            }
        },
        Err(e) => {
            error!(
                "{}",
                crate::tr!("video.web_request_failed", error = format!("{:?}", e))
            );
        }
    }

    Ok(template_config)
}

#[tauri::command]
pub async fn get_video_season(
    app: tauri::AppHandle,
    uid: u64,
    aid: u64,
) -> Result<Value, AppError> {
    let app_data = app.state::<AppData>();
    let bilibili = app_data.get_bilibili(uid).await?;
    let season_id = video_season_id(&bilibili, "https://member.bilibili.com", aid).await?;
    if season_id == 0 {
        return Ok(json!({ "season_id": 0, "section_id": 0 }));
    }
    let api = bilibili.season_api();
    let seasons = load_seasons(&api).await?;
    let membership = find_video_season(&api, &seasons, aid, season_id).await?;
    Ok(match membership {
        Some((season_id, section_id, _)) => {
            json!({ "season_id": season_id, "section_id": section_id })
        }
        None => json!({ "season_id": 0, "section_id": 0 }),
    })
}

#[tauri::command]
pub async fn switch_season(
    app: tauri::AppHandle,
    uid: u64,
    aid: u64,
    season_id: u64,
    section_id: u64,
) -> Result<bool, AppError> {
    let app_data = app.state::<AppData>();
    let bilibili = app_data.get_bilibili(uid).await?;
    let current_season_id = video_season_id(&bilibili, "https://member.bilibili.com", aid).await?;
    if current_season_id == 0 && season_id == 0 {
        return Ok(false);
    }
    let api = bilibili.season_api();
    let seasons = load_seasons(&api).await?;
    let target_section = if season_id == 0 {
        None
    } else {
        let entry = seasons
            .iter()
            .find(|entry| entry.season.id == season_id)
            .ok_or_else(|| AppError::Custom("目标合集不存在".to_string()))?;
        let section = entry
            .sections
            .sections
            .iter()
            .find(|section| section_id == 0 || section.id == section_id)
            .filter(|section| section.id > 0)
            .ok_or_else(|| AppError::Custom("目标合集的小节不存在".to_string()))?;
        Some(section.id)
    };
    let current = find_video_season(&api, &seasons, aid, current_season_id).await?;
    if current.as_ref().map(|(_, section_id, _)| *section_id) == target_section {
        return Ok(false);
    }
    let episode = if target_section.is_some() {
        let proxy = app_data
            .config
            .lock()
            .await
            .config
            .get(&uid)
            .and_then(|c| c.proxy.clone());
        Some(season_episode(&bilibili, aid, proxy.as_deref()).await?)
    } else {
        None
    };
    change_season(&api, current, target_section, episode).await
}

async fn season_episode(
    bilibili: &BiliBili,
    aid: u64,
    proxy: Option<&str>,
) -> Result<EpisodeAdd, AppError> {
    if let Ok(view) = bilibili.archive_view(aid).await
        && view.first_cid() > 0
    {
        return Ok(EpisodeAdd::from(&view));
    }
    // 新投稿尚未公开时，通过创作者接口查询 CID，不写入本地视频配置。
    let data = bilibili
        .video_data(&Vid::Aid(aid), proxy)
        .await
        .map_err(|e| AppError::Biliup(e.to_string()))?;
    creator_season_episode(aid, &data)
}

fn creator_season_episode(aid: u64, data: &Value) -> Result<EpisodeAdd, AppError> {
    let cid = data["videos"][0]["cid"]
        .as_u64()
        .filter(|cid| *cid > 0)
        .ok_or_else(|| AppError::Custom("无法获取稿件首个分 P 的 CID".to_string()))?;
    let title = data["archive"]["title"]
        .as_str()
        .ok_or_else(|| AppError::Custom("无法获取稿件标题".to_string()))?;
    Ok(EpisodeAdd {
        aid,
        cid,
        title: title.to_string(),
        charging_pay: data["archive"]["charging_pay"].as_u64().unwrap_or(0) as u8,
    })
}

async fn change_season(
    api: &SeasonApi<'_>,
    current: Option<(u64, u64, Episode)>,
    target_section: Option<u64>,
    episode: Option<EpisodeAdd>,
) -> Result<bool, AppError> {
    if target_section.is_some() && episode.is_none() {
        return Err(AppError::Custom("无法获取稿件信息".to_string()));
    }
    if let Some((_, _, old_episode)) = &current {
        api.remove_from_season(old_episode.id)
            .await
            .map_err(|e| AppError::Biliup(e.to_string()))?;
    }
    if let Some(section_id) = target_section {
        let episode = episode.ok_or_else(|| AppError::Custom("无法获取稿件信息".to_string()))?;
        if let Err(error) = api
            .add_to_season(section_id, std::slice::from_ref(&episode))
            .await
        {
            if let Some((_, old_section, old_episode)) = current {
                let restore = EpisodeAdd {
                    aid: old_episode.aid,
                    cid: if old_episode.cid > 0 {
                        old_episode.cid
                    } else {
                        episode.cid
                    },
                    title: old_episode.title,
                    charging_pay: old_episode
                        .extra
                        .get("charging_pay")
                        .and_then(Value::as_u64)
                        .unwrap_or(episode.charging_pay as u64)
                        as u8,
                };
                if let Err(restore_error) = api.add_to_season(old_section, &[restore]).await {
                    return Err(AppError::Biliup(format!(
                        "加入目标合集失败：{error}；恢复原合集也失败：{restore_error}"
                    )));
                }
            }
            return Err(AppError::Biliup(error.to_string()));
        }
    }
    Ok(true)
}

/// 导出日志
#[tauri::command]
pub async fn export_logs() -> Result<String, AppError> {
    use std::fs;
    use std::io::Write;
    use zip::ZipWriter;

    let log_dir = crate::utils::get_log_path().map_err(AppError::Internal)?;

    // 创建临时zip文件
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let zip_path = log_dir.join(format!("logs_export_{timestamp}.zip"));

    let zip_file = fs::File::create(&zip_path)?;
    let mut zip = ZipWriter::new(zip_file);
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // 添加日志文件
    if let Ok(entries) = fs::read_dir(&log_dir) {
        for entry in entries.flatten() {
            if let Some(extension) = entry.path().extension() {
                if extension == "log" {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if let Ok(content) = fs::read(entry.path()) {
                        zip.start_file(&file_name, options)?;
                        zip.write_all(&content)?;
                    }
                }
            }
        }
    }

    zip.finish()?;

    Ok(zip_path.to_string_lossy().to_string())
}

/// 检查更新
#[tauri::command]
pub async fn check_update() -> Result<Option<String>, AppError> {
    use reqwest;
    use serde_json::Value;

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/biliup/biliup-app-new/releases/latest")
        .header("User-Agent", "biliup-app")
        .send()
        .await?;

    let release_info: Value = response.json().await?;

    let latest_tag = release_info["tag_name"]
        .as_str()
        .ok_or_else(|| AppError::Custom(crate::i18n::text("update.tag_unavailable").to_string()))?;

    info!(
        "{}",
        crate::tr!("update.latest_version", version = latest_tag)
    );
    // 解析版本号 (格式: app-va.b.c)
    let latest_version = latest_tag
        .strip_prefix("app-v")
        .ok_or_else(|| AppError::Custom(crate::i18n::text("update.invalid_tag").to_string()))?;

    let current_version = env!("CARGO_PKG_VERSION");

    if is_newer_version(latest_version, current_version).map_err(AppError::Internal)? {
        Ok(Some(latest_tag.to_string()))
    } else {
        Ok(None)
    }
}

/// 比较版本号
fn is_newer_version(latest: &str, current: &str) -> anyhow::Result<bool> {
    let parse_version = |v: &str| -> anyhow::Result<Vec<u32>> {
        v.split('.')
            .map(|part| {
                part.parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("{}", crate::tr!("update.invalid_version")))
            })
            .collect()
    };

    let latest_parts = parse_version(latest)?;
    let current_parts = parse_version(current)?;

    for (latest_part, current_part) in latest_parts.iter().zip(current_parts.iter()) {
        if latest_part > current_part {
            return Ok(true);
        } else if latest_part < current_part {
            return Ok(false);
        }
    }

    // 如果前面的部分都相等，比较长度
    Ok(latest_parts.len() > current_parts.len())
}

/// 前端日志转发
#[tauri::command]
pub async fn console_log(
    _app: tauri::AppHandle,
    level: String,
    messages: Vec<String>,
) -> Result<(), AppError> {
    let message = messages.join(" ");
    match level.as_str() {
        "log" => info!("Webconsole: {}", message),
        "error" => error!("Webconsole: {}", message),
        "warn" => warn!("Webconsole: {}", message),
        _ => info!("Webconsole: {}", message),
    }
    Ok(())
}

#[cfg(test)]
mod video_list_tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_reuses_metadata_across_pages_and_rejects_closed_or_foreign_sessions() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let member = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicU64::new(0));
        let requests_for_server = requests.clone();
        let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = tokio::select! {
                    result = listener.accept() => result.unwrap(),
                    _ = &mut stopped => break,
                };
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                }
                requests_for_server.fetch_add(1, Ordering::SeqCst);
                let request = String::from_utf8(request).unwrap();
                let url = reqwest::Url::parse(&format!(
                    "http://test{}",
                    request.split_whitespace().nth(1).unwrap()
                ))
                .unwrap();
                let pn = url
                    .query_pairs()
                    .find(|(key, _)| key == "pn")
                    .unwrap()
                    .1
                    .parse::<u64>()
                    .unwrap();
                let ps = url
                    .query_pairs()
                    .find(|(key, _)| key == "ps")
                    .unwrap()
                    .1
                    .parse::<u64>()
                    .unwrap();
                let entries: Vec<_> = ((pn - 1) * ps + 1..=(pn * ps).min(650)).map(|aid|
                    json!({"Archive": {"aid": aid, "title": format!("稿件 {aid}")}, "season_add_state": aid % 2})
                ).collect();
                let body =
                    json!({"code": 0, "data": {"page": {"count": 650}, "arc_audits": entries}})
                        .to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let snapshots = CreatorVideoSnapshots::default();
        let id = snapshots.open(1).await;
        let (first, second) = tokio::join!(
            snapshots.page(&client, &member, 1, id.clone(), 1, 5),
            snapshots.page(&client, &member, 1, id.clone(), 2, 5)
        );
        assert_eq!(
            first
                .unwrap()
                .items
                .iter()
                .map(|video| video.aid)
                .collect::<Vec<_>>(),
            vec![2, 4, 6, 8, 10]
        );
        let second = second.unwrap();
        assert_eq!(second.total, 325);
        assert_eq!(
            second
                .items
                .iter()
                .map(|video| video.aid)
                .collect::<Vec<_>>(),
            vec![12, 14, 16, 18, 20]
        );
        let initial_requests = requests.load(Ordering::SeqCst);
        assert_eq!(initial_requests, 650_u64.div_ceil(300));
        assert!(
            snapshots
                .page(&client, &member, 2, id.clone(), 1, 5)
                .await
                .is_err()
        );
        snapshots.close(1, id.clone()).await;
        assert!(snapshots.page(&client, &member, 1, id, 1, 5).await.is_err());
        assert_eq!(requests.load(Ordering::SeqCst), initial_requests);
        let refreshed = snapshots.open(1).await;
        assert_eq!(
            snapshots
                .page(&client, &member, 1, refreshed.clone(), 1, 5)
                .await
                .unwrap()
                .total,
            325
        );
        assert_eq!(requests.load(Ordering::SeqCst), initial_requests * 2);
        snapshots.close(1, refreshed).await;
        assert!(snapshots.entries.lock().await.is_empty());
        let _ = stop.send(());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn season_candidates_are_filtered_across_upstream_pages_before_pagination() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let member = format!("http://{}", listener.local_addr().unwrap());
        let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = tokio::select! {
                    connection = listener.accept() => connection.unwrap(),
                    _ = &mut stopped => break,
                };
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                }
                let request = String::from_utf8(request).unwrap();
                let path = request
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap();
                let url = reqwest::Url::parse(&format!("http://test{path}")).unwrap();
                let parameter = |name: &str| {
                    url.query_pairs()
                        .find(|(key, _)| key == name)
                        .unwrap()
                        .1
                        .parse::<u64>()
                        .unwrap()
                };
                let pn = parameter("pn");
                let ps = parameter("ps");
                if url.path().starts_with("/capped/") {
                    assert_eq!(ps, if pn == 1 { 300 } else { 100 });
                } else {
                    assert!(ps == 300 || ps == 10);
                }
                let ps = if url.path().starts_with("/capped/") {
                    ps.min(100)
                } else {
                    ps
                };
                let total = if url.path().starts_with("/empty/") {
                    0
                } else {
                    1235
                };
                let entries: Vec<_> = ((pn - 1) * ps + 1..=(pn * ps).min(total)).map(|aid| {
                    let state = if !url.path().starts_with("/none/") && aid > 1100 && aid % 3 == 0 { 0 } else { 1 };
                    json!({"Archive": {"aid": aid, "title": format!("稿件 {aid}")}, "season_add_state": state})
                }).collect();
                let body =
                    json!({"code": 0, "data": {"page": {"count": total}, "arc_audits": entries}})
                        .to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for (page, expected) in [
            (1, (1101..=1128).step_by(3).collect::<Vec<_>>()),
            (2, (1131..=1158).step_by(3).collect()),
            (5, (1221..=1233).step_by(3).collect()),
            (6, vec![]),
        ] {
            let result = load_creator_video_page(&client, &member, page, 10, true)
                .await
                .unwrap();
            assert_eq!(result.total, 45);
            assert_eq!(
                result
                    .items
                    .iter()
                    .map(|video| video.aid)
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(
                result
                    .items
                    .iter()
                    .all(|video| video.season_add_state == Some(0))
            );
        }
        let ordinary = load_creator_video_page(&client, &member, 2, 10, false)
            .await
            .unwrap();
        assert_eq!(ordinary.total, 1235);
        assert_eq!(
            ordinary
                .items
                .iter()
                .map(|video| video.aid)
                .collect::<Vec<_>>(),
            (11..=20).collect::<Vec<_>>()
        );
        for path in ["none", "empty"] {
            let result = load_creator_video_page(&client, &format!("{member}/{path}"), 1, 10, true)
                .await
                .unwrap();
            assert!(result.items.is_empty());
            assert_eq!(result.total, 0);
        }
        let capped = load_creator_video_page(&client, &format!("{member}/capped"), 1, 5, true)
            .await
            .unwrap();
        assert_eq!(capped.total, 45);
        assert_eq!(
            capped
                .items
                .iter()
                .map(|video| video.aid)
                .collect::<Vec<_>>(),
            vec![1101, 1104, 1107, 1110, 1113]
        );
        let _ = stop.send(());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn downloads_covers_as_base64_and_keeps_list_when_one_download_fails() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let image = b"\xff\xd8\xff\xe0test-cover";
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 2048];
                let size = socket.read(&mut request).await.unwrap();
                if request[..size].starts_with(b"GET /cover.jpg ") {
                    socket
                        .write_all(
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                image.len()
                            )
                            .as_bytes(),
                        )
                        .await
                        .unwrap();
                    socket.write_all(image).await.unwrap();
                } else {
                    socket
                        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        .await
                        .unwrap();
                }
            }
        });
        let mut page = CreatorVideoPage {
            items: vec![
                CreatorVideo {
                    aid: 1,
                    bvid: String::new(),
                    title: "封面成功".into(),
                    season_add_state: None,
                    cover: format!("http://{address}/cover.jpg"),
                },
                CreatorVideo {
                    aid: 2,
                    bvid: String::new(),
                    title: "封面失败".into(),
                    season_add_state: None,
                    cover: format!("http://{address}/missing.jpg"),
                },
                CreatorVideo {
                    aid: 3,
                    bvid: String::new(),
                    title: "无封面".into(),
                    season_add_state: None,
                    cover: String::new(),
                },
            ],
            total: 3,
        };
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        tokio::time::timeout(
            Duration::from_secs(5),
            download_video_covers(&client, &mut page),
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(page.items[0].cover, encode_base64(image));
        assert!(page.items[1].cover.is_empty());
        assert!(page.items[2].cover.is_empty());
        let serialized = serde_json::to_value(&page).unwrap();
        assert_eq!(serialized["items"][0]["cover"], encode_base64(image));
        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 3);
    }

    #[test]
    fn season_add_state_is_read_from_audit_entry_without_defaulting_to_zero() {
        let page = parse_video_list(json!({
            "code": 0,
            "data": {
                "arc_audits": [
                    {"Archive": {"aid": 1, "title": "可加入"}, "season_add_state": 0},
                    {"Archive": {"aid": 2, "title": "不可加入"}, "season_add_state": 1},
                    {"Archive": {"aid": 3, "title": "字段缺失", "season_add_state": 0}},
                    {"Archive": {"aid": 4, "title": "字段为空"}, "season_add_state": null}
                ], "page": {"count": 4}
            }
        }))
        .unwrap();
        assert_eq!(
            page.items
                .iter()
                .map(|video| video.season_add_state)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1), None, None]
        );
        assert_eq!(
            serde_json::to_value(&page).unwrap()["items"][0]["season_add_state"],
            0
        );
    }

    #[test]
    fn parses_documented_archive_shape_and_pagination() {
        let page = parse_video_list(json!({
            "code": 0,
            "data": {
                "arc_audits": [
                    {"Archive": {"aid": 123, "bvid": "BVtest", "title": "测试稿件", "cover": "/bfs/archive/test.jpg"}},
                    {"Archive": {"aid": 456, "title": "无 BV 号稿件", "cover": "//i1.hdslb.com/test.jpg"}}
                ],
                "page": {"pn": 2, "ps": 10, "count": 25}
            }
        }))
        .unwrap();
        assert_eq!(page.total, 25);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].aid, 123);
        assert_eq!(page.items[0].title, "测试稿件");
        assert_eq!(
            page.items[0].cover,
            "http://i1.hdslb.com/bfs/archive/test.jpg"
        );
        assert_eq!(page.items[1].bvid, "");
        assert_eq!(page.items[1].cover, "http://i1.hdslb.com/test.jpg");
    }

    #[test]
    fn cover_paths_use_image_domain_and_preserve_absolute_urls() {
        for (cover, expected) in [
            (
                "bfs/archive/test.jpg",
                "http://i1.hdslb.com/bfs/archive/test.jpg",
            ),
            (
                "http://i1.hdslb.com/test.jpg",
                "http://i1.hdslb.com/test.jpg",
            ),
            (
                "https://i2.hdslb.com/test.jpg",
                "https://i2.hdslb.com/test.jpg",
            ),
            ("", ""),
        ] {
            let page = parse_video_list(json!({
                "code": 0,
                "data": {
                    "arc_audits": [{"Archive": {"aid": 123, "title": "测试稿件", "cover": cover}}],
                    "page": {"count": 1}
                }
            }))
            .unwrap();
            assert_eq!(page.items[0].cover, expected);
        }
    }

    #[test]
    fn empty_list_is_allowed_but_api_errors_and_invalid_lists_are_rejected() {
        for entries in [json!([]), Value::Null] {
            let page = parse_video_list(json!({
                "code": 0, "data": {"arc_audits": entries, "page": {"count": 0}}
            }))
            .unwrap();
            assert!(page.items.is_empty());
            assert_eq!(page.total, 0);
        }
        let error = parse_video_list(json!({"code": -101, "message": "账号未登录"}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("-101"));
        assert!(error.contains("账号未登录"));
        for response in [
            json!({}),
            json!({"code": 0, "data": {}}),
            json!({"code": 0, "data": {"page": {"count": 1}, "arc_audits": null}}),
            json!({"code": 0, "data": {"page": {"count": 1}, "arc_audits": [{"Archive": {"aid": 0, "title": "无效稿件"}}]}}),
        ] {
            assert!(parse_video_list(response).is_err());
        }
    }
}

#[cfg(test)]
mod video_settings_tests {
    use super::*;

    #[test]
    fn web_settings_override_creator_values_including_false_and_zero() {
        let mut template: TemplateConfig = serde_json::from_value(json!({
            "tid": 29, "tid_v2": 9, "dolby": 1, "lossless_music": 0,
            "open_subtitle": true, "subtitle": {"open": 1, "lan": "en"},
            "watermark": 0, "up_selection_reply": 1, "title": "原始标题"
        }))
        .unwrap();
        // 使用附件的字段层级和值，验证异名参数及 data 层的设置能补齐第一步结果。
        apply_web_video_settings(
            &mut template,
            &json!({
                "archive": {
                    "human_type2": {"id": 1003, "name": "音乐"},
                    "is_dolby": 0, "lossless_music": 1,
                    "attrs": {"is_dolby": 1, "lossless_music": 0},
                    "title": "不在本次覆盖范围"
                },
                "subtitle": {"allow": false, "lan": ""},
                "watermark": {"state": 1},
                "reply": {"up_selection": false}
            }),
        );
        assert_eq!(template.tid, 29);
        assert_eq!(template.tid_v2, 1003);
        assert_eq!(template.dolby, 0);
        assert_eq!(template.lossless_music, 1);
        assert!(!template.open_subtitle);
        assert_eq!(template.subtitle.open, 0);
        assert_eq!(template.subtitle.lan, "");
        assert_eq!(template.watermark, 1);
        assert_eq!(template.up_selection_reply, 0);
        assert_eq!(template.title, "原始标题");
    }

    #[test]
    fn web_settings_support_attrs_and_enabled_boolean_settings() {
        let mut template: TemplateConfig = serde_json::from_value(json!({"watermark": 1})).unwrap();
        apply_web_video_settings(
            &mut template,
            &json!({
                "archive": {"attrs": {"is_dolby": 1, "lossless_music": 1}},
                "subtitle": {"allow": true, "lan": "zh-CN"},
                "watermark": {"state": 0},
                "reply": {"up_selection": true}
            }),
        );
        assert_eq!(template.dolby, 1);
        assert_eq!(template.lossless_music, 1);
        assert!(template.open_subtitle);
        assert_eq!(template.subtitle.open, 1);
        assert_eq!(template.subtitle.lan, "zh-CN");
        assert_eq!(template.watermark, 0);
        assert_eq!(template.up_selection_reply, 1);
    }

    #[test]
    fn missing_or_invalid_web_settings_preserve_creator_values() {
        let mut template: TemplateConfig = serde_json::from_value(json!({
            "tid_v2": 1003, "dolby": 1, "lossless_music": 1,
            "open_subtitle": true, "subtitle": {"open": 1, "lan": "zh-CN"},
            "watermark": 1, "up_selection_reply": 1
        }))
        .unwrap();
        let original = serde_json::to_value(&template).unwrap();
        apply_web_video_settings(&mut template, &json!({}));
        apply_web_video_settings(
            &mut template,
            &json!({
                "archive": {"human_type2": {"id": 4294967296_u64}, "is_dolby": -1, "lossless_music": 256},
                "subtitle": {"allow": null, "lan": false},
                "watermark": {"state": 2}, "reply": {"up_selection": 1}
            }),
        );
        assert_eq!(serde_json::to_value(&template).unwrap(), original);
    }
}

#[cfg(test)]
mod season_tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn bilibili() -> BiliBili {
        BiliBili {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            login_info: serde_json::from_value(json!({
                "cookie_info": {"cookies": [{"name": "bili_jct", "value": "test-csrf"}]},
                "sso": [],
                "token_info": {"access_token": "", "expires_in": 0, "mid": 1, "refresh_token": ""},
                "platform": null
            }))
            .unwrap(),
        }
    }

    // 只使用本地假服务，记录请求顺序和参数，不访问真实账号。
    async fn server(responses: Vec<Value>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0; 4096];
                    let len =
                        tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
                            .await
                            .unwrap()
                            .unwrap();
                    assert!(len > 0);
                    bytes.extend_from_slice(&buffer[..len]);
                    if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let content_len = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + content_len {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let body = response.to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        (host, handle)
    }

    fn current() -> Option<(u64, u64, Episode)> {
        Some((
            1,
            10,
            serde_json::from_value(json!({
                "id": 999, "aid": 123, "cid": 456, "title": "原稿件"
            }))
            .unwrap(),
        ))
    }

    fn episode() -> EpisodeAdd {
        EpisodeAdd {
            aid: 123,
            cid: 789,
            title: "更新稿件".to_string(),
            charging_pay: 0,
        }
    }

    fn request_json(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[tokio::test]
    async fn season_move_uses_episode_id_and_restores_original_on_add_failure() {
        let (host, requests) = server(vec![
            json!({"code": 0, "data": null}),
            json!({"code": -400, "message": "拒绝加入"}),
            json!({"code": 0, "data": null}),
        ])
        .await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let error = change_season(&api, current(), Some(20), Some(episode()))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("拒绝加入"));
        let requests = requests.await.unwrap();
        assert!(requests[0].starts_with("POST /x2/creative/web/season/section/episode/del "));
        assert!(requests[0].contains("id=999"));
        assert_eq!(request_json(&requests[1])["sectionId"], 20);
        let restored = request_json(&requests[2]);
        assert_eq!(restored["sectionId"], 10);
        assert_eq!(restored["episodes"][0]["cid"], 456);
        assert_eq!(restored["episodes"][0]["title"], "原稿件");
    }

    #[tokio::test]
    async fn season_move_adds_to_target_section_after_removal() {
        let (host, requests) = server(vec![
            json!({"code": 0, "data": null}),
            json!({"code": 0, "data": null}),
        ])
        .await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        assert!(
            change_season(&api, current(), Some(20), Some(episode()))
                .await
                .unwrap()
        );
        let requests = requests.await.unwrap();
        assert!(requests[0].contains("id=999"));
        let added = request_json(&requests[1]);
        assert_eq!(added["sectionId"], 20);
        assert_eq!(added["episodes"][0]["cid"], 789);
        assert_eq!(added["episodes"][0]["title"], "更新稿件");
    }

    #[tokio::test]
    async fn season_move_reports_both_add_and_restore_failures() {
        let (host, requests) = server(vec![
            json!({"code": 0, "data": null}),
            json!({"code": -400, "message": "目标拒绝"}),
            json!({"code": -500, "message": "原合集拒绝"}),
        ])
        .await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let error = change_season(&api, current(), Some(20), Some(episode()))
            .await
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("目标拒绝"));
        assert!(message.contains("原合集拒绝"));
        assert_eq!(requests.await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn season_remove_failure_does_not_attempt_to_add() {
        let (host, requests) = server(vec![json!({"code": -400, "message": "移出失败"})]).await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        assert!(
            change_season(&api, current(), Some(20), Some(episode()))
                .await
                .is_err()
        );
        assert_eq!(requests.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn season_remove_does_not_require_cid() {
        let (host, requests) = server(vec![json!({"code": 0, "data": null})]).await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let mut current = current();
        current.as_mut().unwrap().2.cid = 0;
        assert!(change_season(&api, current, None, None).await.unwrap());
        assert!(requests.await.unwrap()[0].contains("id=999"));
    }

    #[tokio::test]
    async fn season_move_rejects_missing_episode_before_removing() {
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, "http://127.0.0.1:1", "http://127.0.0.1:1");
        let error = change_season(&api, current(), Some(20), None)
            .await
            .unwrap_err();
        assert!(matches!(error, AppError::Custom(_)));
    }

    #[tokio::test]
    async fn season_membership_preserves_non_default_section() {
        let (host, requests) = server(vec![
            json!({"code": 0, "data": {"id": 1}}),
            json!({"code": 0, "data": {"section": {"id": 10}, "episodes": []}}),
            json!({"code": 0, "data": {"section": {"id": 11}, "episodes": [
                {"id": 999, "aid": 123, "cid": 456}
            ]}}),
        ])
        .await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let seasons: Vec<SeasonEntry> = serde_json::from_value(json!([
            {"season": {"id": 2}, "sections": {"sections": [{"id": 20}]}},
            {"season": {"id": 1}, "sections": {"sections": [{"id": 10}, {"id": 11}]}},
            {"season": {"id": 3}, "sections": {"sections": [{"id": 30}]}}
        ]))
        .unwrap();
        let membership_id = video_season_id(&bili, &host, 123).await.unwrap();
        let (season_id, section_id, episode) =
            find_video_season(&api, &seasons, 123, membership_id)
                .await
                .unwrap()
                .unwrap();
        assert_eq!((season_id, section_id, episode.id), (1, 11, 999));
        let requests = requests.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with("GET /x2/creative/web/season/aid?id=123&t="));
        assert!(requests[1].starts_with("GET /x2/creative/web/season/section?id=10&"));
        assert!(requests[2].starts_with("GET /x2/creative/web/season/section?id=11&"));
    }

    #[tokio::test]
    async fn season_membership_without_collection_does_not_query_sections() {
        for data in [json!({"id": 0}), Value::Null] {
            let (host, requests) = server(vec![json!({"code": 0, "data": data})]).await;
            let bili = bilibili();
            let api = SeasonApi::with_hosts(&bili, "http://127.0.0.1:1", "http://127.0.0.1:1");
            let seasons: Vec<SeasonEntry> = serde_json::from_value(json!([{
                "season": {"id": 1}, "sections": {"sections": [{"id": 10}]}
            }]))
            .unwrap();
            let season_id = video_season_id(&bili, &host, 123).await.unwrap();
            assert!(
                find_video_season(&api, &seasons, 123, season_id)
                    .await
                    .unwrap()
                    .is_none()
            );
            let requests = requests.await.unwrap();
            assert_eq!(requests.len(), 1);
            assert!(requests[0].starts_with("GET /x2/creative/web/season/aid?id=123&t="));
        }
    }

    #[tokio::test]
    async fn season_membership_not_found_is_unassigned_without_data() {
        for response in [
            json!({"code": -404, "message": "未加入合集"}),
            json!({"code": -404, "data": null}),
            json!({"code": -404, "data": {}}),
        ] {
            let (host, requests) = server(vec![response]).await;
            assert_eq!(video_season_id(&bilibili(), &host, 123).await.unwrap(), 0);
            assert_eq!(requests.await.unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn season_membership_rejects_api_errors_and_malformed_responses() {
        for response in [
            json!({"code": -101, "message": "账号未登录", "data": null}),
            json!({"data": {"id": 0}}),
            json!({"code": 0}),
            json!({"code": 0, "data": {}}),
            json!({"code": 0, "data": {"id": "1"}}),
            json!({"code": 0, "data": {"id": -1}}),
        ] {
            let (host, requests) = server(vec![response]).await;
            assert!(video_season_id(&bilibili(), &host, 123).await.is_err());
            assert_eq!(requests.await.unwrap().len(), 1);
        }
        assert!(
            video_season_id(&bilibili(), "http://127.0.0.1:1", 123)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn season_membership_inconsistency_is_not_treated_as_unassigned() {
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, "http://127.0.0.1:1", "http://127.0.0.1:1");
        assert!(find_video_season(&api, &[], 123, 1).await.is_err());

        let (host, requests) = server(vec![
            json!({"code": 0, "data": {"section": {"id": 10}, "episodes": []}}),
        ])
        .await;
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let seasons: Vec<SeasonEntry> = serde_json::from_value(json!([{
            "season": {"id": 1}, "sections": {"sections": [{"id": 10}]}
        }]))
        .unwrap();
        assert!(find_video_season(&api, &seasons, 123, 1).await.is_err());
        assert_eq!(requests.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn season_list_fetches_more_than_fifty_entries() {
        let entries: Vec<_> = (1..=50).map(|id| json!({"season": {"id": id}})).collect();
        let (host, requests) = server(vec![
            json!({"code": 0, "data": {"seasons": entries, "total": 51}}),
            json!({"code": 0, "data": {"seasons": [{"season": {"id": 51}}], "total": 51}}),
        ])
        .await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        assert_eq!(load_seasons(&api).await.unwrap().len(), 51);
        let requests = requests.await.unwrap();
        assert!(requests[0].contains("pn=1"));
        assert!(requests[1].contains("pn=2"));
    }

    #[test]
    fn season_creator_lookup_uses_first_part_and_rejects_missing_cid() {
        let mut data = json!({
            "archive": {"title": "审核中稿件", "charging_pay": 1},
            "videos": [{"cid": 456}, {"cid": 789}]
        });
        let episode = creator_season_episode(123, &data).unwrap();
        assert_eq!(
            (episode.aid, episode.cid, episode.charging_pay),
            (123, 456, 1)
        );
        assert_eq!(episode.title, "审核中稿件");
        data["videos"][0]["cid"] = json!(0);
        assert!(creator_season_episode(123, &data).is_err());
        data["videos"] = json!([]);
        assert!(creator_season_episode(123, &data).is_err());
    }
}
