use biliup::{
    bilibili::{BiliBili, Vid},
    uploader::season::{Episode, EpisodeAdd, SeasonApi, SeasonEntry},
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use std::{fs::File, io::Read, path::Path};
use tauri::Manager;

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

async fn load_seasons(api: &SeasonApi<'_>) -> Result<Vec<SeasonEntry>, AppError> {
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

async fn find_video_season(
    api: &SeasonApi<'_>,
    seasons: &[SeasonEntry],
    aid: u64,
) -> Result<Option<(u64, u64, Episode)>, AppError> {
    for entry in seasons {
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
    }
    Ok(None)
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
                if let Some(data) = res.get("data") {
                    let archive_data = data.get("archive").unwrap_or(data);

                    if let Some(desc) = archive_data["desc"].as_str().filter(|s| !s.is_empty()) {
                        template_config.desc = desc.to_string();
                    }
                    if let Some(human_type2) = archive_data.get("human_type2") {
                        if let Some(id) = human_type2.get("id").and_then(|v| v.as_u64()) {
                            template_config.tid_v2 = id as u32;
                        }
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
    let api = bilibili.season_api();
    let seasons = load_seasons(&api).await?;
    let membership = find_video_season(&api, &seasons, aid).await?;
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
    let current = find_video_season(&api, &seasons, aid).await?;
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
            json!({"code": 0, "data": {"section": {"id": 10}, "episodes": []}}),
            json!({"code": 0, "data": {"section": {"id": 11}, "episodes": [
                {"id": 999, "aid": 123, "cid": 456}
            ]}}),
        ])
        .await;
        let bili = bilibili();
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let seasons: Vec<SeasonEntry> = serde_json::from_value(json!([{
            "season": {"id": 1}, "sections": {"sections": [{"id": 10}, {"id": 11}]}
        }]))
        .unwrap();
        let (season_id, section_id, episode) = find_video_season(&api, &seasons, 123)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((season_id, section_id, episode.id), (1, 11, 999));
        assert_eq!(requests.await.unwrap().len(), 2);
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
