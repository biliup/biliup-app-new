use biliup::bilibili::BiliBili;
use serde_json::{Value, json};
use std::str::FromStr;
use std::{fs::File, io::Read};
use tauri::Manager;
use tokio::sync::Mutex;
use tracing::info;

use crate::utils::crypto::encode_base64;
use crate::{AppData, models::TemplateConfig};

#[tauri::command]
pub async fn get_current_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
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
        Ok(url) => Ok(url),
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
        Ok(res) => Ok(res["data"]["typelist"].clone()),
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
        Ok(res) => Ok(res["data"]["topics"].clone()),
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
            Ok(res) => Ok(res["data"]["result"]["topics"].clone()),
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
                let mut season_vec = Vec::new();

                let seasons = res["data"]["seasons"].as_array()
                    .ok_or("用户没有创建合集").unwrap_or_else(|_| &season_vec).to_owned();
                for season in &seasons {
                    let season_id = season["season"]["id"].as_u64().unwrap_or(0);
                    let section_id = season["sections"]["sections"][0]["id"].as_u64().unwrap_or(0);
                    let season_title = season["season"]["title"].as_str().unwrap_or("").to_string();
                    season_vec.push(serde_json::json!({
                        "season_id": if season_id != 0 { Some(season_id) } else { None },
                        "section_id": if section_id != 0 { Some(section_id) } else { None },
                        "title": season_title,
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
        Ok(res) => Ok(TemplateConfig::from_bilibili_res(res).map_err(|e| e.to_string())?),
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
        Ok(res) => Ok(res["data"]["id"].as_u64().unwrap_or(0)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn switch_season(
    app: tauri::AppHandle,
    uid: u64,
    aid: u64,
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
            .ok_or("cookie error")?
            .iter()
            .filter_map(|c| c.as_object())
            .find(|c| c["name"] == "bili_jct")
            .ok_or("jct error")?
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("csrf error")?;
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
                    "aid": aid
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
            Ok(_) => Ok(true),
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
                "csrf": csrf
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
        {
            Ok(res) => {
                info!("{res}");
                if res["code"].as_i64() != Some(0) {
                    return Err(serde_json::to_string(&res).unwrap_or("未知错误".to_string()));
                }
                Ok(true)
            }
            Err(e) => Err(e.to_string()),
        }
    }
}
