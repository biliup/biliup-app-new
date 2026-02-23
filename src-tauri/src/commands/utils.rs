use biliup::bilibili::BiliBili;
use serde_json::{Value, json};
use std::str::FromStr;
use std::{fs::File, io::Read, path::Path};
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::utils::crypto::encode_base64;
use crate::utils::file_utils::{self, FileEntry};
use crate::{AppData, models::TemplateConfig};

#[tauri::command]
pub async fn get_current_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// 获取文件大小
#[tauri::command]
pub async fn get_file_size(file_path: String) -> Result<u64, String> {
    let path = Path::new(&file_path);
    file_utils::get_file_size(path).map_err(|e| format!("获取文件大小失败: {e}"))
}

/// 递归读取目录
#[tauri::command]
pub async fn read_dir_recursive(
    dir_path: String,
    include_subdirs: bool,
    max_depth: Option<u32>,
) -> Result<Vec<FileEntry>, String> {
    let path = Path::new(&dir_path);
    file_utils::read_dir_recursive(path, include_subdirs, max_depth)
        .map_err(|e| format!("读取目录失败: {e}"))
}

/// 上传封面并进行返回url
#[tauri::command]
pub async fn upload_cover(app: tauri::AppHandle, uid: u64, file: String) -> Result<String, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let mut cover_file = File::open(file).map_err(|e| format!("打开文件失败: {e}"))?;
    let mut cover_buf = vec![];

    cover_file
        .read_to_end(&mut cover_buf)
        .map_err(|e| format!("读取文件失败: {e}"))?;

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .cover_up(&cover_buf)
        .await
    {
        Ok(url) => {
            info!("封面上传成功: {}", url);
            Ok(url)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// 下载封面并进行base64编码
#[tauri::command]
pub async fn download_cover(
    app: tauri::AppHandle,
    uid: u64,
    url: String,
) -> Result<String, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .client
        .get(&url)
        .send()
        .await
    {
        Ok(res) => {
            let bytes = res.bytes().await.map_err(|e| e.to_string())?;
            Ok(encode_base64(&bytes))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_type_list(app: tauri::AppHandle, uid: u64) -> Result<Value, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .archive_pre()
        .await
    {
        Ok(res) => {
            debug!("获取分区列表成功: {}", res);
            Ok(res["data"]["typelist"].clone())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_topic_list(app: tauri::AppHandle, uid: u64) -> Result<Value, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .client
        .get("https://member.bilibili.com/x/vupre/web/topic/type?pn=0&ps=999")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
    {
        Ok(res) => {
            debug!("获取话题列表成功: {}", res);
            Ok(res["data"]["topics"].clone())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn search_topics(
    app: tauri::AppHandle,
    uid: u64,
    query: String,
) -> Result<Value, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    match app_data.clients.lock().await.get(&uid).ok_or("用户未登录或不存在")?
            .bilibili
            .client
            .get(format!("https://member.bilibili.com/x/vupre/web/topic/search?keywords={query}&page_size=50&offset=0&t={}", chrono::Utc::now().timestamp()))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
        {
            Ok(res) => {
                debug!("搜索话题成功: {}", res);
                Ok(res["data"]["result"]["topics"].clone())
            },
            Err(e) => Err(e.to_string()),
        }
}

#[tauri::command]
pub async fn get_season_list(app: tauri::AppHandle, uid: u64) -> Result<Value, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    match app_data.clients.lock().await.get(&uid).ok_or("用户未登录或不存在")?
            .bilibili
            .client
            .get(format!("https://member.bilibili.com/x2/creative/web/seasons?pn=1&ps=50&order=desc&sort=mtime&filter=1&t={}", chrono::Utc::now().timestamp()))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
        {
            Ok(res) => {
                debug!("获取合集列表成功: {}", res);
                let mut season_vec = Vec::new();

                let seasons = res["data"]["seasons"].as_array()
                    .ok_or("用户没有创建合集").unwrap_or(&season_vec).to_owned();
                for season in &seasons {
                    let season_id = season["season"]["id"].as_u64().unwrap_or(0);
                    let season_title = season["season"]["title"].as_str().unwrap_or("").to_string();
                    let mut sections_vec = Vec::new();

                    if let Some(sections) = season["sections"]["sections"].as_array() {
                        for section in sections {
                            let section_id = section["id"].as_u64().unwrap_or(0);
                            let section_title = section["title"]
                                .as_str()
                                .unwrap_or(&season_title)
                                .to_string();

                            sections_vec.push(serde_json::json!({
                                "section_id": if section_id != 0 { Some(section_id) } else { None },
                                "title": section_title,
                            }));
                        }
                    }

                    let default_section_id = sections_vec
                        .first()
                        .and_then(|item| item["section_id"].as_u64())
                        .unwrap_or(0);

                    season_vec.push(serde_json::json!({
                        "season_id": if season_id != 0 { Some(season_id) } else { None },
                        "section_id": if default_section_id != 0 { Some(default_section_id) } else { None },
                        "title": season_title,
                        "sections": sections_vec,
                    }));
                }

                Ok(serde_json::json!({
                    "seasons": season_vec,
                }))
            },
            Err(e) => Err(e.to_string()),
        }
}

#[tauri::command]
pub async fn get_video_detail(
    app: tauri::AppHandle,
    uid: u64,
    video_id: String,
) -> Result<TemplateConfig, String> {
    let vid = biliup::uploader::bilibili::Vid::from_str(&video_id)
        .map_err(|e| format!("解析视频 ID 失败: {e}"))?;

    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    let true_desc = match app_data.clients.lock().await.get(&uid) {
        Some(client) => {
            match client
                .bilibili
                .client
                .get(format!(
                    "https://api.bilibili.com/x/web-interface/view?{vid}",
                ))
                .send()
                .await
            {
                Ok(response) => match response.json::<Value>().await {
                    Ok(res) => res["data"]["desc"].as_str().unwrap_or("").to_string(),
                    Err(e) => {
                        error!("解析稿件描述响应失败: {:?}", e);
                        "".to_string()
                    }
                },
                Err(e) => {
                    error!("获取稿件描述请求失败: {:?}", e);
                    "".to_string()
                }
            }
        }
        None => {
            error!("用户未登录或不存在，无法获取稿件描述");
            "".to_string()
        }
    };

    let proxy: Option<String> = app_data
        .config
        .lock()
        .await
        .config
        .get(&uid)
        .and_then(|c| c.proxy.clone());

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .video_data(&vid, proxy.as_deref())
        .await
    {
        Ok(res) => {
            let mut template_config =
                TemplateConfig::from_bilibili_res(res).map_err(|e| e.to_string())?;
            if !true_desc.is_empty() {
                template_config.desc = true_desc;
            }
            Ok(template_config)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_video_season(app: tauri::AppHandle, uid: u64, aid: u64) -> Result<u64, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .client
        .get(format!(
            "https://member.bilibili.com/x2/creative/web/season/aid?id={}&t={}",
            aid,
            chrono::Utc::now().timestamp()
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
    {
        Ok(res) => {
            debug!("获取稿件合集信息成功: {}", res);
            Ok(res["data"]["id"].as_u64().unwrap_or(0))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn switch_season(
    app: tauri::AppHandle,
    uid: u64,
    aid: u64,
    cid: u64,
    season_id: u64,
    section_id: u64,
    title: String,
    add: bool,
) -> Result<bool, String> {
    let app_lock = app.state::<Mutex<AppData>>();
    let app_data = app_lock.lock().await;

    fn get_csrf(bilibili: &BiliBili) -> Result<String, String> {
        let csrf = bilibili
            .login_info
            .cookie_info
            .get("cookies")
            .and_then(|c| c.as_array())
            .ok_or("Cookie错误")?
            .iter()
            .filter_map(|c| c.as_object())
            .find(|c| c["name"] == "bili_jct")
            .ok_or("JCT错误")?
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("CSRF错误")?;
        Ok(csrf.to_string())
    }

    let csrf = get_csrf(
        &app_data
            .clients
            .lock()
            .await
            .get(&uid)
            .ok_or("用户未登录或不存在")?
            .bilibili,
    )
    .map_err(|e| e.to_string())?;

    if add {
        match app_data
        .clients
        .lock()
        .await
        .get(&uid)
        .ok_or("用户未登录或不存在")?
        .bilibili
        .client
        .post(format!(
            "https://member.bilibili.com/x2/creative/web/season/section/episodes/add?t={}&csrf={}",
            chrono::Utc::now().timestamp(),
            csrf
        ))
        .json(&json!({
            "episodes": [
                {
                    "title": title,
                    "aid": aid,
                    "cid": cid
                }
            ],
            "sectionId": section_id,
            "csrf": csrf
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        {
            Ok(res) => {
                debug!("设置合集成功：{res}");
                Ok(true)
            },
            Err(e) => Err(e.to_string()),
        }
    } else {
        match app_data
            .clients
            .lock()
            .await
            .get(&uid)
            .ok_or("用户未登录或不存在")?
            .bilibili
            .client
            .post(format!(
                "https://member.bilibili.com/x2/creative/web/season/switch?t={}&csrf={}",
                chrono::Utc::now().timestamp(),
                csrf
            ))
            .json(&json!({
                "season_id": if season_id != 0 { Some(season_id) } else { None },
                "section_id": if section_id != 0 { Some(section_id) } else { None },
                "title": title,
                "aid": aid,
                "cid": cid,
                "csrf": csrf
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
        {
            Ok(res) => {
                debug!("修改合集成功：{res}");
                if res["code"].as_i64() != Some(0) {
                    return Err(serde_json::to_string(&res).unwrap_or("未知错误".to_string()));
                }
                Ok(true)
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// 导出日志
#[tauri::command]
pub async fn export_logs() -> Result<String, String> {
    use std::fs;
    use std::io::Write;
    use zip::ZipWriter;

    let log_dir = crate::utils::get_log_path().map_err(|e| format!("获取日志目录失败: {e}"))?;

    // 创建临时zip文件
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let zip_path = log_dir.join(format!("logs_export_{timestamp}.zip"));

    let zip_file = fs::File::create(&zip_path).map_err(|e| format!("创建ZIP文件失败: {e}"))?;
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
                        zip.start_file(&file_name, options)
                            .map_err(|e| format!("创建ZIP条目失败: {e}"))?;
                        zip.write_all(&content)
                            .map_err(|e| format!("写入ZIP文件失败: {e}"))?;
                    }
                }
            }
        }
    }

    zip.finish().map_err(|e| format!("完成ZIP文件失败: {e}"))?;

    Ok(zip_path.to_string_lossy().to_string())
}

/// 检查更新
#[tauri::command]
pub async fn check_update() -> Result<Option<String>, String> {
    use reqwest;
    use serde_json::Value;

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/biliup/biliup-app-new/releases/latest")
        .header("User-Agent", "biliup-app")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?;

    let release_info: Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;

    let latest_tag = release_info["tag_name"]
        .as_str()
        .ok_or("无法获取最新版本标签")?;

    info!("最新版本：{latest_tag}");
    // 解析版本号 (格式: app-va.b.c)
    let latest_version = latest_tag.strip_prefix("app-v").ok_or("版本标签格式错误")?;

    let current_version = env!("CARGO_PKG_VERSION");

    if is_newer_version(latest_version, current_version)? {
        Ok(Some(latest_tag.to_string()))
    } else {
        Ok(None)
    }
}

/// 比较版本号
fn is_newer_version(latest: &str, current: &str) -> Result<bool, String> {
    let parse_version = |v: &str| -> Result<Vec<u32>, String> {
        v.split('.')
            .map(|part| {
                part.parse::<u32>()
                    .map_err(|_| "版本号格式错误".to_string())
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

/// 检查更新
#[tauri::command]
pub async fn console_log(
    _app: tauri::AppHandle,
    level: String,
    messages: Vec<String>,
) -> Result<(), String> {
    let message = messages.join(" ");
    match level.as_str() {
        "log" => info!("Webconsole: {}", message),
        "error" => error!("Webconsole: {}", message),
        "warn" => warn!("Webconsole: {}", message),
        _ => info!("Webconsole: {}", message),
    }
    Ok(())
}
