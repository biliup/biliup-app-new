use anyhow::Result;
use biliup::credential::LoginInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::PathBuf,
};
use tracing::{debug, info, warn};

fn current_timestamp() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: u64,
    pub name: String,
    pub cookie: LoginInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Subtitle {
    #[serde(default)]
    pub open: u8,
    #[serde(default)]
    pub lan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Credit {
    #[serde(rename = "type")]
    pub r#type: u8,
    #[serde(default)]
    pub raw_text: String,
    #[serde(default)]
    pub biz_id: String,
    #[serde(default)]
    pub sub_type: u8,
    #[serde(default)]
    pub sub_biz_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Staff {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub mid: u64,
    #[serde(default)]
    pub is_del: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub finished_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    #[serde(default)]
    pub copyright: u8, // 1: 自制, 2: 转载
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub tid: u32, // 分区ID
    #[serde(default)]
    pub tid_v2: u32,
    #[serde(default)]
    pub cover: String, // 封面URL
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub desc_v2: Option<Vec<Credit>>,
    #[serde(default)]
    pub dynamic: String, // 粉丝动态
    #[serde(default)]
    pub subtitle: Subtitle,
    #[serde(default)]
    pub tag: String, // 逗号分隔的标签
    #[serde(default)]
    pub videos: Vec<VideoInfo>,
    #[serde(default)]
    pub dtime: Option<u32>, // 定时发布时间
    #[serde(default)]
    pub open_subtitle: bool,
    #[serde(default)]
    pub interactive: u8,
    #[serde(default)]
    pub mission_id: Option<u32>,
    #[serde(default)]
    pub topic_id: Option<u32>,
    #[serde(default)]
    pub topic_name: Option<String>,
    #[serde(default)]
    pub season_id: Option<u64>,
    #[serde(default)]
    pub section_id: Option<u64>,
    #[serde(default)]
    pub dolby: u8,
    #[serde(default)]
    pub lossless_music: u8, // Hi-Res无损音质
    #[serde(default)]
    pub no_reprint: u8,
    #[serde(default)]
    pub open_elec: u8,
    #[serde(default)]
    pub no_disturbance: u8,
    #[serde(default)]
    pub auto_edit_after_submit: u8,
    #[serde(default)]
    pub aid: Option<u64>,
    #[serde(default)]
    pub up_selection_reply: u8,
    #[serde(default)]
    pub up_close_reply: u8,
    #[serde(default)]
    pub up_close_danmu: u8,
    #[serde(default)]
    pub atomic_int: u32,
    #[serde(default)]
    pub is_only_self: u8,
    #[serde(default)]
    pub watermark: u8,
    #[serde(default)]
    pub is_360: i64,
    #[serde(default)]
    pub staff: Option<Vec<Staff>>,
    #[serde(default)]
    pub state: Option<i64>,
    #[serde(default)]
    pub state_desc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub user: UserInfo,
    #[serde(default)]
    pub line: Option<String>,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default = "default_upload_limit")]
    pub limit: u32,
    #[serde(default)]
    pub watermark: u8,
    #[serde(default)]
    pub auto_edit: u8,
    #[serde(default)]
    pub templates: HashMap<String, TemplateConfig>, // 匹配config.json中的"templates"字段
    #[serde(default)]
    pub template_order: Vec<String>,
    #[serde(default)]
    pub template_updated_at: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRoot {
    #[serde(skip)]
    pub read_only: bool,
    #[serde(default)]
    pub max_curr: u32,
    #[serde(default)]
    pub auto_upload: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub user_order: Vec<u64>,
    #[serde(default)]
    pub config: HashMap<u64, UserConfig>,
}

fn default_upload_limit() -> u32 {
    3
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_language() -> String {
    "zh-CN".to_string()
}

fn normalize_language(value: &str) -> String {
    crate::i18n::normalize_language(value).to_string()
}

impl ConfigRoot {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let load = |path: &PathBuf| -> Result<Self> {
            let json_content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&json_content)?)
        };
        let mut config = match load(path) {
            Ok(config) => config,
            Err(original_error) => {
                let backup = path.with_extension("json.bak");
                match load(&backup) {
                    Ok(config) => {
                        warn!(
                            "{}",
                            crate::tr!("config.backup_restored", error = original_error)
                        );
                        config
                    }
                    Err(_) => return Err(original_error),
                }
            }
        };
        config.normalize_template_metadata();
        Ok(config)
    }

    pub fn save_to_file(&mut self, path: &PathBuf) -> Result<()> {
        anyhow::ensure!(
            !self.read_only,
            crate::i18n::text("config.recovery_read_only")
        );
        self.normalize_template_metadata();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        fs::create_dir_all(parent)?;
        let json_content = serde_json::to_vec_pretty(self)?;
        if let Ok(previous) = fs::read(path) {
            if serde_json::from_slice::<Self>(&previous).is_ok() {
                atomic_write(&path.with_extension("json.bak"), &previous)?;
            }
        }
        atomic_write(path, &json_content)
    }

    pub fn save_logout_to_file(&mut self, path: &PathBuf) -> Result<()> {
        anyhow::ensure!(
            !self.read_only,
            crate::i18n::text("config.recovery_read_only")
        );
        self.normalize_template_metadata();
        let content = serde_json::to_vec_pretty(self)?;
        // 先清除备份中的登录信息，避免主配置恢复时重新登录已退出的账号。
        atomic_write(&path.with_extension("json.bak"), &content)?;
        atomic_write(path, &content)
    }

    pub fn normalize_template_metadata(&mut self) {
        self.normalize_user_order();
        self.language = normalize_language(&self.language);
        for user_config in self.config.values_mut() {
            if matches!(user_config.line.as_deref(), Some("ws" | "qn" | "bda")) {
                user_config.line = None;
            }
            if !(1..=12).contains(&user_config.limit) {
                user_config.limit = default_upload_limit();
            }
            user_config.normalize_template_metadata();
        }
    }

    fn normalize_user_order(&mut self) {
        let mut seen = HashSet::new();
        self.user_order
            .retain(|uid| self.config.contains_key(uid) && seen.insert(*uid));

        let mut missing_uids: Vec<u64> = self
            .config
            .keys()
            .copied()
            .filter(|uid| !seen.contains(uid))
            .collect();
        missing_uids.sort_unstable();
        self.user_order.extend(missing_uids);
    }

    pub fn new_user_config(
        &mut self,
        uid: u64,
        username: String,
        cookie: LoginInfo,
        proxy: Option<String>,
    ) -> &Self {
        let user_info = UserInfo {
            uid,
            name: username,
            cookie,
        };
        let user_config = UserConfig {
            user: user_info,
            line: None,
            proxy,
            limit: default_upload_limit(),
            watermark: 0,
            auto_edit: 0,
            templates: HashMap::new(),
            template_order: Vec::new(),
            template_updated_at: HashMap::new(),
        };
        self.config.insert(uid, user_config);
        if !self.user_order.contains(&uid) {
            self.user_order.push(uid);
        }

        self
    }

    pub fn add_user_config(&mut self, config: UserConfig) -> &Self {
        let uid = config.user.uid;
        let mut config = config;
        config.normalize_template_metadata();
        self.config.insert(uid, config);
        if !self.user_order.contains(&uid) {
            self.user_order.push(uid);
        }

        self
    }

    pub fn remove_user_config(&mut self, uid: u64) -> Result<&Self> {
        if self.config.remove(&uid).is_some() {
            self.user_order.retain(|id| *id != uid);
            Ok(self)
        } else {
            Err(anyhow::anyhow!("{}", crate::tr!("config.user_missing")))
        }
    }

    pub fn save_user_order(&mut self, user_order: Vec<u64>) -> Result<&Self> {
        self.user_order = user_order;
        self.normalize_user_order();
        Ok(self)
    }

    pub fn save_user_config(
        &mut self,
        uid: u64,
        line: Option<String>,
        proxy: Option<String>,
        limit: u32,
        watermark: u8,
        auto_edit: u8,
    ) -> Result<&Self> {
        anyhow::ensure!(
            (1..=12).contains(&limit),
            crate::i18n::text("upload.invalid_concurrency")
        );
        anyhow::ensure!(
            matches!(
                line.as_deref(),
                None | Some(
                    "auto"
                        | "bda2"
                        | "bldsa"
                        | "tx"
                        | "txa"
                        | "alia"
                        | "cnbd"
                        | "anbd"
                        | "atbd"
                        | "cnbldsa"
                        | "andsa"
                        | "atdsa"
                        | "cntx"
                        | "antx"
                        | "attx"
                        | "estx"
                        | "akbd"
                )
            ),
            crate::i18n::text("upload.unsupported_route")
        );
        if let Some(user_config) = self.config.get_mut(&uid) {
            info!(
                "{}",
                crate::tr!(
                    "config.user_updated",
                    uid = uid,
                    line = format!("{:?}", line),
                    proxy_enabled = proxy.is_some(),
                    limit = limit,
                    watermark = watermark,
                    auto_edit = auto_edit
                )
            );
            user_config.line = line;
            user_config.proxy = proxy;
            user_config.limit = limit;
            user_config.watermark = watermark;
            user_config.auto_edit = auto_edit;
            Ok(self)
        } else {
            Err(anyhow::anyhow!("{}", crate::tr!("config.user_missing")))
        }
    }

    pub fn save_global_config(
        &mut self,
        max_curr: u32,
        auto_start: bool,
        auto_upload: bool,
        log_level: String,
        language: String,
    ) -> &Self {
        info!(
            "{}",
            crate::tr!(
                "config.global_updating",
                max_curr = max_curr,
                auto_start = auto_start,
                auto_upload = auto_upload,
                log_level = log_level,
                language = language
            )
        );
        self.max_curr = max_curr;
        self.auto_start = auto_start;
        self.auto_upload = auto_upload;
        self.log_level = log_level;
        self.language = normalize_language(&language);

        self
    }

    pub fn add_user_template(
        &mut self,
        uid: u64,
        template_name: &str,
        template: TemplateConfig,
    ) -> TemplateConfig {
        if let Some(user_config) = self.config.get_mut(&uid) {
            let template_name = template_name.to_owned();
            let existed = user_config.templates.contains_key(&template_name);
            let old = user_config
                .templates
                .insert(template_name.clone(), template.clone());
            #[cfg(debug_assertions)]
            if let Some(ref old_val) = old {
                // Avoid borrow conflict by not using self here
                Self::compare_templates_debug(old_val, &template);
            }

            if !existed {
                user_config.template_order.push(template_name.clone());
            }
            user_config
                .template_updated_at
                .insert(template_name, current_timestamp());
            user_config.normalize_template_metadata();
        }

        template
    }

    pub fn delete_user_template(
        &mut self,
        uid: u64,
        template_name: &str,
    ) -> Option<TemplateConfig> {
        if let Some(user_config) = self.config.get_mut(&uid) {
            let removed = user_config.templates.remove(template_name);
            if removed.is_some() {
                user_config
                    .template_order
                    .retain(|name| name != template_name);
                user_config.template_updated_at.remove(template_name);
            }
            user_config.normalize_template_metadata();
            removed
        } else {
            None
        }
    }

    pub fn get_user_template_order(&self, uid: u64) -> &[String] {
        self.config
            .get(&uid)
            .map(|user_config| user_config.template_order.as_slice())
            .unwrap_or(&[])
    }

    pub fn save_template_order(&mut self, uid: u64, template_order: Vec<String>) -> Result<&Self> {
        if let Some(user_config) = self.config.get_mut(&uid) {
            user_config.template_order = template_order;
            user_config.normalize_template_metadata();
            Ok(self)
        } else {
            Err(anyhow::anyhow!("{}", crate::tr!("config.user_missing")))
        }
    }

    pub fn rename_user_template(
        &mut self,
        uid: u64,
        old_name: &str,
        new_name: &str,
    ) -> Result<&Self> {
        if let Some(user_config) = self.config.get_mut(&uid) {
            if !user_config.templates.contains_key(old_name) {
                return Err(anyhow::anyhow!(
                    "{}",
                    crate::tr!("template.original_missing")
                ));
            }

            if user_config.templates.contains_key(new_name) {
                return Err(anyhow::anyhow!("{}", crate::tr!("template.name_exists")));
            }

            let template = user_config
                .templates
                .remove(old_name)
                .ok_or_else(|| anyhow::anyhow!("{}", crate::tr!("template.original_missing")))?;

            user_config.templates.insert(new_name.to_string(), template);

            if let Some(updated_at) = user_config.template_updated_at.remove(old_name) {
                user_config
                    .template_updated_at
                    .insert(new_name.to_string(), updated_at);
            }

            for name in &mut user_config.template_order {
                if name == old_name {
                    *name = new_name.to_string();
                }
            }

            user_config.normalize_template_metadata();
            Ok(self)
        } else {
            Err(anyhow::anyhow!("{}", crate::tr!("config.user_missing")))
        }
    }
}

impl Default for ConfigRoot {
    fn default() -> Self {
        Self {
            read_only: false,
            max_curr: 1,
            auto_start: true,
            auto_upload: true,
            log_level: default_log_level(),
            language: default_language(),
            user_order: Vec::new(),
            config: HashMap::new(),
        }
    }
}

impl ConfigRoot {
    #[cfg(debug_assertions)]
    fn compare_templates_debug(old: &TemplateConfig, new: &TemplateConfig) {
        macro_rules! compare_field {
            ($field:ident, $old:expr, $new:expr) => {
                if $old.$field != $new.$field {
                    debug!(
                        "Field '{}' updated: {:?} -> {:?}",
                        stringify!($field),
                        $old.$field,
                        $new.$field
                    );
                }
            };
        }

        fn compare_subtitle(old: &Subtitle, new: &Subtitle) {
            compare_field!(open, old, new);
            compare_field!(lan, old, new);
        }

        fn compare_video_info_vec(old: &Vec<VideoInfo>, new: &Vec<VideoInfo>) {
            // 打印新增的 VideoInfo
            for new_video in new {
                if !old.contains(new_video) {
                    debug!(
                        "{}",
                        crate::tr!("video.adding", video = format!("{:?}", new_video))
                    );
                }
            }
            // 打印被删除的 VideoInfo
            for old_video in old {
                if !new.contains(old_video) {
                    debug!(
                        "{}",
                        crate::tr!("video.removing", video = format!("{:?}", old_video))
                    );
                }
            }
        }

        fn compare_template_fields(old: &TemplateConfig, new: &TemplateConfig) {
            compare_field!(copyright, old, new);
            compare_field!(source, old, new);
            compare_field!(tid, old, new);
            compare_field!(tid_v2, old, new);
            compare_field!(cover, old, new);
            compare_field!(title, old, new);
            compare_field!(desc, old, new);
            compare_field!(desc_v2, old, new);
            compare_field!(dynamic, old, new);
            compare_subtitle(&old.subtitle, &new.subtitle);
            compare_field!(tag, old, new);
            compare_video_info_vec(&old.videos, &new.videos);
            compare_field!(dtime, old, new);
            compare_field!(open_subtitle, old, new);
            compare_field!(interactive, old, new);
            compare_field!(mission_id, old, new);
            compare_field!(dolby, old, new);
            compare_field!(lossless_music, old, new);
            compare_field!(no_reprint, old, new);
            compare_field!(open_elec, old, new);
            compare_field!(no_disturbance, old, new);
            compare_field!(auto_edit_after_submit, old, new);
            compare_field!(aid, old, new);
            compare_field!(up_selection_reply, old, new);
            compare_field!(up_close_reply, old, new);
            compare_field!(up_close_danmu, old, new);
            compare_field!(atomic_int, old, new);
        }

        compare_template_fields(old, new);
    }
}

impl UserConfig {
    pub fn normalize_template_metadata(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.template_order
            .retain(|name| self.templates.contains_key(name) && seen.insert(name.clone()));

        for template_name in self.templates.keys() {
            if !self.template_order.iter().any(|name| name == template_name) {
                self.template_order.push(template_name.clone());
            }
        }

        self.template_updated_at
            .retain(|name, _| self.templates.contains_key(name));

        for template_name in self.templates.keys() {
            self.template_updated_at
                .entry(template_name.clone())
                .or_insert(0);
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            copyright: 1, // 默认自制
            source: String::new(),
            tid: 0,
            tid_v2: 0,
            cover: String::new(),
            title: String::new(),
            desc: String::new(),
            desc_v2: None,
            dynamic: String::new(),
            subtitle: Subtitle::default(),
            tag: String::new(),
            videos: Vec::new(),
            dtime: None,
            open_subtitle: false,
            interactive: 0,
            mission_id: None,
            topic_id: None,
            topic_name: None,
            season_id: None,
            section_id: None,
            dolby: 0,
            lossless_music: 0,
            no_reprint: 0,
            open_elec: 0,
            no_disturbance: 0,
            auto_edit_after_submit: 0,
            aid: None,
            up_selection_reply: 0,
            up_close_reply: 0,
            up_close_danmu: 0,
            atomic_int: 0,
            is_only_self: 0,
            watermark: 0,
            is_360: -1,
            staff: None,
            state: None,
            state_desc: None,
        }
    }
}

fn atomic_write(path: &std::path::Path, content: &[u8]) -> Result<()> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn obsolete_video_fields_are_ignored_when_loading_and_saving_config() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "biliup-legacy-cid-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let legacy_config = serde_json::json!({
            "config": {
                "1": {
                    "user": {
                        "uid": 1,
                        "name": "测试账号",
                        "cookie": {
                            "cookie_info": {"cookies": []},
                            "sso": [],
                            "token_info": {"access_token": "", "expires_in": 0, "mid": 1, "refresh_token": ""},
                            "platform": null
                        }
                    },
                    "templates": {
                        "旧模板": {"videos": [{
                            "filename": "test-video", "cid": 12345,
                            "encoding_status": -30, "status_desc": "转码中"
                        }]}
                    }
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&legacy_config).unwrap()).unwrap();
        let mut config = ConfigRoot::from_file(&path).unwrap();
        assert_eq!(
            config.config[&1].templates["旧模板"].videos[0].filename,
            "test-video"
        );
        config.save_to_file(&path).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let video = &saved["config"]["1"]["templates"]["旧模板"]["videos"][0];
        assert_eq!(video["filename"], "test-video");
        assert!(video.get("cid").is_none());
        assert!(video.get("encoding_status").is_none());
        assert!(video.get("status_desc").is_none());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creator_video_status_is_not_imported_into_templates() {
        let template = TemplateConfig::from_bilibili_res(serde_json::json!({
            "archive": {"aid": 123, "title": "测试稿件"},
            "videos": [{"filename": "remote-file", "title": "分P标题", "status": -30,
                "status_desc": "转码中", "encoding_status": 0}]
        }))
        .unwrap();
        assert_eq!(template.aid, Some(123));
        assert_eq!(template.videos[0].filename, "remote-file");
        assert_eq!(template.videos[0].title, "分P标题");
        let saved = serde_json::to_value(&template.videos[0]).unwrap();
        for key in ["status", "encoding_status", "status_desc"] {
            assert!(saved.get(key).is_none());
        }
    }

    #[test]
    fn language_persists_and_old_configs_remain_compatible() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "biliup-language-test-{}-{nonce}",
            std::process::id()
        ));
        let path = dir.join("config.json");
        let mut config = ConfigRoot::default();
        let cookie: LoginInfo = serde_json::from_value(serde_json::json!({
            "cookie_info": {"cookies": [{"name": "SESSDATA", "value": "test-secret"}]},
            "sso": [],
            "token_info": {"access_token": "", "expires_in": 0, "mid": 1, "refresh_token": ""},
            "platform": null
        }))
        .unwrap();
        config.new_user_config(1, "中文用户名".to_string(), cookie, None);
        let mut template = TemplateConfig::default();
        template.title = "中文投稿标题".to_string();
        template.desc = "原始简介".to_string();
        config.add_user_template(1, "中文模板", template);
        // 使用真实持久化路径验证语言字段，账号及模板数据不参与翻译。
        let original_users = serde_json::to_value(&config.config).unwrap();
        for language in ["en-US", "ja-JP", "zh-TW"] {
            config.save_global_config(2, true, true, "info".to_string(), language.to_string());
            config.save_to_file(&path).unwrap();
            let restored = ConfigRoot::from_file(&path).unwrap();
            assert_eq!(restored.language, language);
            assert_eq!(
                serde_json::to_value(&restored.config).unwrap(),
                original_users
            );
        }
        let restored = ConfigRoot::from_file(&path).unwrap();

        let mut old_config = serde_json::to_value(&restored).unwrap();
        old_config.as_object_mut().unwrap().remove("language");
        fs::write(&path, serde_json::to_vec(&old_config).unwrap()).unwrap();
        assert_eq!(ConfigRoot::from_file(&path).unwrap().language, "zh-CN");

        old_config["language"] = serde_json::json!("unsupported");
        fs::write(&path, serde_json::to_vec(&old_config).unwrap()).unwrap();
        assert_eq!(ConfigRoot::from_file(&path).unwrap().language, "zh-CN");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recover_previous_config_after_corrupt_primary() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("biliup-config-test-{}-{nonce}", std::process::id()));
        let path = dir.join("config.json");
        let mut config = ConfigRoot::default();
        config.save_to_file(&path).unwrap();
        config.max_curr = 4;
        config.save_to_file(&path).unwrap();
        assert_eq!(ConfigRoot::from_file(&path).unwrap().max_curr, 4);
        fs::write(&path, "{").unwrap();
        assert_eq!(ConfigRoot::from_file(&path).unwrap().max_curr, 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn logout_backup_does_not_restore_removed_account() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("biliup-logout-test-{}-{nonce}", std::process::id()));
        let path = dir.join("config.json");
        let cookie: LoginInfo = serde_json::from_value(serde_json::json!({
            "cookie_info": {"cookies": [{"name": "SESSDATA", "value": "test-secret"}]},
            "sso": [],
            "token_info": {"access_token": "", "expires_in": 0, "mid": 1, "refresh_token": ""},
            "platform": null
        }))
        .unwrap();
        let mut config = ConfigRoot::default();
        config.new_user_config(1, "测试账号".into(), cookie.clone(), None);
        config.new_user_config(2, "保留账号".into(), cookie, None);
        config.save_to_file(&path).unwrap();
        config.remove_user_config(1).unwrap();
        config.save_logout_to_file(&path).unwrap();
        let current = ConfigRoot::from_file(&path).unwrap();
        assert!(!current.config.contains_key(&1));
        assert!(current.config.contains_key(&2));
        fs::write(&path, "{").unwrap();
        let recovered = ConfigRoot::from_file(&path).unwrap();
        assert!(!recovered.config.contains_key(&1));
        assert!(recovered.config.contains_key(&2));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recovery_mode_preserves_primary_and_backup() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "biliup-readonly-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let backup = path.with_extension("json.bak");
        fs::write(&path, "original-primary").unwrap();
        fs::write(&backup, "original-backup").unwrap();
        let mut config = ConfigRoot {
            read_only: true,
            ..ConfigRoot::default()
        };
        assert!(config.save_to_file(&path).is_err());
        assert!(config.save_logout_to_file(&path).is_err());
        let mut cloned = config.clone();
        assert!(cloned.save_to_file(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original-primary");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "original-backup");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_replace_preserves_destination_and_cleans_temporary_file() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("biliup-write-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::create_dir(&path).unwrap();
        assert!(atomic_write(&path, b"{}").is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
