use anyhow::Result;
use biliup::credential::LoginInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};
use tracing::{debug, info};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub cid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub finished_at: u64,
    #[serde(default)]
    pub encoding_status: i64,
    #[serde(default)]
    pub status_desc: String,
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
    pub cover: String, // 封面URL
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub desc_v2: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub user: UserInfo,
    #[serde(default)]
    pub line: Option<String>,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
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
    #[serde(default)]
    pub max_curr: u32,
    #[serde(default)]
    pub auto_upload: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub user_order: Vec<u64>,
    #[serde(default)]
    pub config: HashMap<u64, UserConfig>,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl ConfigRoot {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let json_content = fs::read_to_string(path)?;
        let mut config: Self = serde_json::from_str(&json_content)?;
        config.normalize_template_metadata();
        Ok(config)
    }

    pub fn save_to_file(&mut self, path: &PathBuf) -> Result<()> {
        self.normalize_template_metadata();

        // 确保父目录存在
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        let json_content = serde_json::to_string_pretty(self)?;
        fs::write(path, json_content)?;
        Ok(())
    }

    pub fn normalize_template_metadata(&mut self) {
        self.normalize_user_order();
        for user_config in self.config.values_mut() {
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
            limit: 0,
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
            Err(anyhow::anyhow!("用户配置不存在"))
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
        if let Some(user_config) = self.config.get_mut(&uid) {
            info!(
                "UID {} 用户配置更新: line={:?}, proxy={:?}, limit={}, watermark={}, auto_edit={}",
                uid, line, proxy, limit, watermark, auto_edit
            );
            user_config.line = line;
            user_config.proxy = proxy;
            user_config.limit = limit;
            user_config.watermark = watermark;
            user_config.auto_edit = auto_edit;
            Ok(self)
        } else {
            Err(anyhow::anyhow!("用户配置不存在"))
        }
    }

    pub fn save_global_config(
        &mut self,
        max_curr: u32,
        auto_start: bool,
        auto_upload: bool,
        log_level: String,
    ) -> &Self {
        info!(
            "更新全局配置: max_curr={}, auto_start={}, auto_upload={}, log_level={}",
            max_curr, auto_start, auto_upload, log_level
        );
        self.max_curr = max_curr;
        self.auto_start = auto_start;
        self.auto_upload = auto_upload;
        self.log_level = log_level;

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

    pub fn save_template_order(&mut self, uid: u64, template_order: Vec<String>) -> Result<&Self> {
        if let Some(user_config) = self.config.get_mut(&uid) {
            user_config.template_order = template_order;
            user_config.normalize_template_metadata();
            Ok(self)
        } else {
            Err(anyhow::anyhow!("用户配置不存在"))
        }
    }

    pub fn rename_user_template(&mut self, uid: u64, old_name: &str, new_name: &str) -> Result<&Self> {
        if let Some(user_config) = self.config.get_mut(&uid) {
            if !user_config.templates.contains_key(old_name) {
                return Err(anyhow::anyhow!("原模板不存在"));
            }

            if user_config.templates.contains_key(new_name) {
                return Err(anyhow::anyhow!("该名称的模板已存在"));
            }

            let template = user_config
                .templates
                .remove(old_name)
                .ok_or_else(|| anyhow::anyhow!("原模板不存在"))?;

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
            Err(anyhow::anyhow!("用户配置不存在"))
        }
    }

    pub fn default() -> Self {
        Self {
            max_curr: 1,
            auto_start: true,
            auto_upload: true,
            log_level: default_log_level(),
            user_order: Vec::new(),
            config: HashMap::new(),
        }
    }

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
                    debug!("添加视频信息: {:?}", new_video);
                }
            }
            // 打印被删除的 VideoInfo
            for old_video in old {
                if !new.contains(old_video) {
                    debug!("删除视频信息: {:?}", old_video);
                }
            }
        }

        fn compare_template_fields(old: &TemplateConfig, new: &TemplateConfig) {
            compare_field!(copyright, old, new);
            compare_field!(source, old, new);
            compare_field!(tid, old, new);
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
            season_id: None,
            section_id: None,
            dolby: 0,
            lossless_music: 0,
            no_reprint: 0,
            open_elec: 0,
            aid: None,
            up_selection_reply: 0,
            up_close_reply: 0,
            up_close_danmu: 0,
            atomic_int: 0,
            is_only_self: 0,
            watermark: 0,
        }
    }
}
