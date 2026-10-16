use std::{collections::HashSet, str::FromStr};

use biliup::{
    bilibili::Vid,
    uploader::season::{EpisodeAdd, EpisodeSort, SeasonApi, SeasonEntry, SectionDetail},
};
use futures::{StreamExt, stream};
use serde_json::{Value, json};
use tauri::Manager;

use super::utils::load_seasons;
use crate::{AppData, error::AppError};

fn invalid_section() -> AppError {
    AppError::Custom(crate::i18n::text("season.manager.section_missing").into())
}

// 校验小节所属合集，排序时始终使用远端的小节标题。
async fn checked_section(
    api: &SeasonApi<'_>,
    season_id: u64,
    section_id: u64,
) -> Result<SectionDetail, AppError> {
    let seasons = load_seasons(api).await?;
    let section = seasons
        .iter()
        .find(|entry| entry.season.id == season_id)
        .and_then(|entry| entry.sections.sections.iter().find(|s| s.id == section_id))
        .ok_or_else(invalid_section)?;
    let mut detail = api
        // 不指定 sort，保留合集的默认顺序；desc 会把展示顺序反转。
        .season_section(section_id, None)
        .await
        .map_err(|error| AppError::Biliup(error.to_string()))?;
    if detail.section.id != section_id || detail.section.season_id.is_some_and(|id| id != season_id)
    {
        return Err(invalid_section());
    }
    if detail.section.title.is_empty() {
        detail.section.title = section.title.clone();
    }
    Ok(detail)
}

fn episode_sorts(
    detail: &SectionDetail,
    episode_ids: &[u64],
) -> Result<Vec<EpisodeSort>, AppError> {
    let expected: HashSet<_> = detail.episodes.iter().map(|episode| episode.id).collect();
    let requested: HashSet<_> = episode_ids.iter().copied().collect();
    if episode_ids.is_empty() || requested.len() != episode_ids.len() || expected != requested {
        return Err(AppError::Custom(
            crate::i18n::text("season.manager.stale_order").into(),
        ));
    }
    Ok(episode_ids
        .iter()
        .enumerate()
        .map(|(index, id)| EpisodeSort {
            id: *id,
            sort: index as u32 + 1,
        })
        .collect())
}

#[tauri::command]
pub async fn list_managed_seasons(
    app: tauri::AppHandle,
    uid: u64,
) -> Result<Vec<SeasonEntry>, AppError> {
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    load_seasons(&bili.season_api()).await
}

#[tauri::command]
pub async fn get_managed_section(
    app: tauri::AppHandle,
    uid: u64,
    season_id: u64,
    section_id: u64,
) -> Result<SectionDetail, AppError> {
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    let api = bili.season_api();
    let mut detail = checked_section(&api, season_id, section_id).await?;
    detail.episodes = stream::iter(detail.episodes)
        .map(|mut episode| {
            let api = &api;
            async move {
                if episode
                    .extra
                    .get("archiveTitle")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                {
                    // 单集标题可独立修改，不能用它冒充稿件标题。
                    if let Ok(view) = api.archive_view(episode.aid).await {
                        episode
                            .extra
                            .insert("archiveTitle".into(), json!(view.title));
                        episode.extra.insert("bvid".into(), json!(view.bvid));
                    }
                }
                episode
            }
        })
        .buffered(4)
        .collect()
        .await;
    Ok(detail)
}

fn episode_title_payload(
    detail: &SectionDetail,
    season_id: u64,
    episode_id: u64,
    title: &str,
) -> Result<Value, AppError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Custom(
            crate::i18n::text("season.manager.title_empty").into(),
        ));
    }
    let (index, episode) = detail
        .episodes
        .iter()
        .enumerate()
        .find(|(_, episode)| episode.id == episode_id)
        .ok_or_else(|| {
            AppError::Custom(crate::i18n::text("season.manager.episode_missing").into())
        })?;
    if episode.cid == 0 {
        return Err(AppError::Custom(
            crate::i18n::text("season.manager.cid_missing").into(),
        ));
    }
    let ids: Vec<_> = detail.episodes.iter().map(|episode| episode.id).collect();
    Ok(json!({
        "id": episode.id,
        "aid": episode.aid,
        "cid": episode.cid,
        "title": title,
        "seasonId": season_id,
        "sectionId": detail.section.id,
        "order": index + 1,
        "sorts": episode_sorts(detail, &ids)?,
    }))
}

async fn send_episode_title(
    bili: &biliup::bilibili::BiliBili,
    url: &str,
    payload: &Value,
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
    let response: Value = bili
        .client
        .post(url)
        .query(&[("csrf", csrf)])
        .json(payload)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if response["code"].as_i64() != Some(0) {
        return Err(AppError::Biliup(format!(
            "{}: {}",
            response["code"],
            response["message"].as_str().unwrap_or_default()
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn edit_season_episode_title(
    app: tauri::AppHandle,
    uid: u64,
    season_id: u64,
    section_id: u64,
    episode_id: u64,
    title: String,
) -> Result<(), AppError> {
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    let detail = checked_section(&bili.season_api(), season_id, section_id).await?;
    let payload = episode_title_payload(&detail, season_id, episode_id, &title)?;
    send_episode_title(
        &bili,
        "https://member.bilibili.com/x2/creative/web/season/section/episode/edit",
        &payload,
    )
    .await
}

#[tauri::command]
pub async fn add_season_episodes(
    app: tauri::AppHandle,
    uid: u64,
    season_id: u64,
    section_id: u64,
    video_ids: Vec<String>,
) -> Result<Vec<u64>, AppError> {
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    add_episodes(&bili.season_api(), season_id, section_id, &video_ids).await
}

async fn add_episodes(
    api: &SeasonApi<'_>,
    season_id: u64,
    section_id: u64,
    video_ids: &[String],
) -> Result<Vec<u64>, AppError> {
    if video_ids.is_empty() {
        return Err(AppError::Custom(
            crate::i18n::text("season.manager.no_selection").into(),
        ));
    }
    let vids = video_ids
        .iter()
        .map(|id| {
            Vid::from_str(id.trim()).map_err(|error| {
                AppError::Custom(crate::tr!("video.id_parse_failed", error = error))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let detail = checked_section(api, season_id, section_id).await?;
    let existing: HashSet<_> = detail.episodes.iter().map(|episode| episode.aid).collect();
    let mut seen = HashSet::new();
    let mut episodes = Vec::new();
    for vid in vids {
        let view = api
            .archive_view_by_vid(&vid)
            .await
            .map_err(|error| AppError::Biliup(error.to_string()))?;
        if existing.contains(&view.aid) {
            return Err(AppError::Custom(
                crate::i18n::text("season.manager.already_added").into(),
            ));
        }
        if view.first_cid() == 0 {
            return Err(AppError::Custom(
                crate::i18n::text("season.manager.cid_missing").into(),
            ));
        }
        if seen.insert(view.aid) {
            episodes.push(EpisodeAdd::from(&view));
        }
    }
    // 全部稿件校验通过后只发一次添加请求；接口报错仍由前端刷新确认远端结果。
    api.add_to_season(section_id, &episodes)
        .await
        .map_err(|error| AppError::Biliup(error.to_string()))?;
    Ok(episodes.iter().map(|episode| episode.aid).collect())
}

#[tauri::command]
pub async fn remove_season_episode(
    app: tauri::AppHandle,
    uid: u64,
    season_id: u64,
    section_id: u64,
    episode_id: u64,
) -> Result<(), AppError> {
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    let api = bili.season_api();
    let detail = checked_section(&api, season_id, section_id).await?;
    if !detail
        .episodes
        .iter()
        .any(|episode| episode.id == episode_id)
    {
        return Err(AppError::Custom(
            crate::i18n::text("season.manager.episode_missing").into(),
        ));
    }
    api.remove_from_season(episode_id)
        .await
        .map_err(|error| AppError::Biliup(error.to_string()))
}

#[tauri::command]
pub async fn reorder_season_episodes(
    app: tauri::AppHandle,
    uid: u64,
    season_id: u64,
    section_id: u64,
    episode_ids: Vec<u64>,
) -> Result<(), AppError> {
    let bili = app.state::<AppData>().get_bilibili(uid).await?;
    let api = bili.season_api();
    let detail = checked_section(&api, season_id, section_id).await?;
    let sorts = episode_sorts(&detail, &episode_ids)?;
    api.sort_season_episodes(section_id, season_id, &sorts, &detail.section.title)
        .await
        .map_err(|error| AppError::Biliup(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn batch_add_validates_all_videos_before_one_write_and_reports_api_failure() {
        use biliup::bilibili::BiliBili;
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let mut writes = Vec::new();
            let mut reads = 0;
            loop {
                let (mut socket, _) = tokio::select! {
                    result = listener.accept() => result.unwrap(),
                    _ = &mut stopped => break,
                };
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    header.push(socket.read_u8().await.unwrap());
                }
                let header = String::from_utf8(header).unwrap();
                let path = header.split_whitespace().nth(1).unwrap();
                let url = reqwest::Url::parse(&format!("http://test{path}")).unwrap();
                let data = if header.starts_with("POST ") {
                    let length: usize = header
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length: ")
                                .map(str::to_owned)
                        })
                        .unwrap()
                        .parse()
                        .unwrap();
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).await.unwrap();
                    let body: Value = serde_json::from_slice(&body).unwrap();
                    assert!(url.path().ends_with("/episodes/add"));
                    assert_eq!(body["sectionId"], 11);
                    assert_eq!(body["episodes"].as_array().unwrap().len(), 2);
                    assert_eq!(body["episodes"][0]["aid"], 123);
                    assert_eq!(body["episodes"][1]["aid"], 125);
                    writes.push(body);
                    Value::Null
                } else if url.path().ends_with("/seasons") {
                    reads += 1;
                    json!({"total": 1, "seasons": [{"season": {"id": 1}, "sections": {"sections": [{"id": 11}]}}]})
                } else if url.path().ends_with("/section") {
                    reads += 1;
                    json!({"section": {"id": 11, "seasonId": 1}, "episodes": []})
                } else {
                    let aid = url
                        .query_pairs()
                        .find(|(key, _)| key == "aid")
                        .unwrap()
                        .1
                        .parse::<u64>()
                        .unwrap();
                    json!({"aid": aid, "cid": if aid == 124 { 0 } else { aid + 1000 }, "title": format!("稿件 {aid}")})
                };
                let response = if header.starts_with("POST /fail/") {
                    json!({"code": -400, "message": "写入失败"})
                } else {
                    json!({"code": 0, "data": data})
                };
                let body = response.to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
            (reads, writes.len())
        });
        let bili = BiliBili {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            login_info: serde_json::from_value(json!({
                "cookie_info": {"cookies": [{"name": "bili_jct", "value": "test-csrf"}]}, "sso": [],
                "token_info": {"access_token": "", "expires_in": 0, "mid": 1, "refresh_token": ""}, "platform": "bilibili"
            })).unwrap(),
        };
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        assert!(add_episodes(&api, 1, 11, &[]).await.is_err());
        assert!(
            add_episodes(&api, 1, 11, &["av123".into(), "av124".into()])
                .await
                .is_err()
        );
        let ids = vec!["av123".into(), "av125".into(), "av123".into()];
        assert!(add_episodes(&api, 1, 11, &ids).await.is_ok());
        let failing_host = format!("{host}/fail");
        let api = SeasonApi::with_hosts(&bili, &failing_host, &host);
        assert!(add_episodes(&api, 1, 11, &ids).await.is_err());
        let _ = stop.send(());
        let (reads, writes) = server.await.unwrap();
        assert_eq!(reads, 6); // 每批只查一次合集、一次小节。
        assert_eq!(writes, 2); // CID 缺失的批次完全不写入，成功与 API 失败各一次请求。
    }

    #[test]
    fn title_edit_preserves_archive_identity_and_current_order() {
        let detail: SectionDetail = serde_json::from_value(json!({
            "section": {"id": 11, "seasonId": 1},
            "episodes": [
                {"id": 22, "aid": 122, "cid": 322, "title": "旧单集标题", "archiveTitle": "稿件标题"},
                {"id": 21, "aid": 121, "cid": 321, "title": "另一单集"}
            ]
        })).unwrap();
        let payload = episode_title_payload(&detail, 1, 21, " 新单集标题 ").unwrap();
        assert_eq!(
            payload,
            json!({
                "id": 21, "aid": 121, "cid": 321, "title": "新单集标题",
                "seasonId": 1, "sectionId": 11, "order": 2,
                "sorts": [{"id": 22, "sort": 1}, {"id": 21, "sort": 2}]
            })
        );
        assert_eq!(detail.episodes[0].extra["archiveTitle"], "稿件标题");
        assert!(episode_title_payload(&detail, 1, 21, "  ").is_err());
        assert!(episode_title_payload(&detail, 1, 121, "新标题").is_err());
        let mut missing_cid = detail;
        missing_cid.episodes[1].cid = 0;
        assert!(episode_title_payload(&missing_cid, 1, 21, "新标题").is_err());
    }

    #[tokio::test]
    async fn title_edit_posts_json_with_csrf_and_checks_response_code() {
        use biliup::bilibili::BiliBili;
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/episode/edit", listener.local_addr().unwrap());
        let payload = json!({"id": 21, "title": "新单集标题"});
        let expected = payload.clone();
        let server = tokio::spawn(async move {
            for response in [
                json!({"code": 0, "data": null}),
                json!({"code": -400, "message": "错误"}),
                json!({"data": null}),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    header.push(socket.read_u8().await.unwrap());
                }
                let header = String::from_utf8(header).unwrap();
                assert!(header.starts_with("POST /episode/edit?csrf=test-csrf "));
                assert!(
                    header
                        .to_lowercase()
                        .contains("content-type: application/json")
                );
                let length: usize = header
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .unwrap()
                    .parse()
                    .unwrap();
                let mut body = vec![0; length];
                socket.read_exact(&mut body).await.unwrap();
                assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), expected);
                let body = response.to_string();
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let mut bili = BiliBili {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            login_info: serde_json::from_value(json!({
                "cookie_info": {"cookies": [{"name": "bili_jct", "value": "test-csrf"}]}, "sso": [],
                "token_info": {"access_token": "", "expires_in": 0, "mid": 1, "refresh_token": ""},
                "platform": "bilibili"
            }))
            .unwrap(),
        };
        assert!(send_episode_title(&bili, &url, &payload).await.is_ok());
        assert!(send_episode_title(&bili, &url, &payload).await.is_err());
        assert!(send_episode_title(&bili, &url, &payload).await.is_err());
        server.await.unwrap();
        bili.login_info.cookie_info = json!({"cookies": []});
        assert!(send_episode_title(&bili, &url, &payload).await.is_err());
    }

    #[tokio::test]
    async fn section_query_preserves_default_order_without_desc_sort() {
        use biliup::bilibili::BiliBili;
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for data in [
                json!({"seasons": [{"season": {"id": 1}, "sections": {"sections": [{"id": 11, "title": "正片"}]}}], "total": 1}),
                json!({"section": {"id": 11, "seasonId": 1, "title": "正片"}, "episodes": [{"id": 22, "aid": 122, "order": 1}, {"id": 21, "aid": 121, "order": 2}]}),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                }
                let request = String::from_utf8(request).unwrap();
                let request_line = request.lines().next().unwrap();
                assert!(!request_line.contains("sort="));
                let body = json!({"code": 0, "data": data}).to_string();
                socket.write_all(format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
                ).as_bytes()).await.unwrap();
            }
        });
        let bili = BiliBili {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            login_info: serde_json::from_value(json!({
                "cookie_info": {"cookies": []}, "sso": [],
                "token_info": {"access_token": "", "expires_in": 0, "mid": 0, "refresh_token": ""},
                "platform": "bilibili"
            }))
            .unwrap(),
        };
        let api = SeasonApi::with_hosts(&bili, &host, &host);
        let detail = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            checked_section(&api, 1, 11),
        )
        .await
        .unwrap()
        .unwrap();
        server.await.unwrap();
        assert_eq!(
            detail
                .episodes
                .iter()
                .map(|episode| episode.id)
                .collect::<Vec<_>>(),
            vec![22, 21]
        );
        assert_eq!(
            episode_sorts(&detail, &[22, 21]).unwrap(),
            vec![
                EpisodeSort { id: 22, sort: 1 },
                EpisodeSort { id: 21, sort: 2 }
            ]
        );
    }

    #[test]
    fn sorting_uses_episode_ids_and_one_based_positions() {
        let detail: SectionDetail = serde_json::from_value(json!({
            "section": {"id": 11, "seasonId": 1, "title": "正片"},
            "episodes": [{"id": 21, "aid": 121}, {"id": 22, "aid": 122}]
        }))
        .unwrap();
        assert_eq!(
            episode_sorts(&detail, &[22, 21]).unwrap(),
            vec![
                EpisodeSort { id: 22, sort: 1 },
                EpisodeSort { id: 21, sort: 2 }
            ]
        );
        for ids in [
            vec![],
            vec![21],
            vec![21, 21],
            vec![121, 122],
            vec![21, 22, 23],
        ] {
            assert!(episode_sorts(&detail, &ids).is_err());
        }
    }
}
