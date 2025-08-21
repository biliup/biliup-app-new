use crate::{
    models::{ConfigRoot, Subtitle, TemplateConfig, UserConfig, UserInfo, VideoInfo},
    services::validate_cookie_in_old_config,
    utils::{get_config_json_path, get_config_yaml_path, get_old_cookie_file_path},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use tracing::{info, warn};

/// 旧版本config.yaml的结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyUserAccount {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyUser {
    pub account: LegacyUserAccount,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyVideoInfo {
    pub title: String,
    pub filename: String,
    pub desc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyStreamerConfig {
    pub copyright: u8,
    pub source: String,
    pub tid: u32,
    pub cover: String,
    pub title: String,
    pub desc_format_id: u32,
    pub desc: String,
    pub desc_v2: Option<String>,
    pub dynamic: String,
    pub subtitle: Subtitle,
    pub tag: String,
    pub videos: Vec<LegacyVideoInfo>,
    pub dtime: Option<u32>,
    pub open_subtitle: bool,
    pub interactive: u8,
    pub mission_id: Option<u32>,
    pub dolby: u8,
    pub lossless_music: u8,
    pub no_reprint: u8,
    pub open_elec: u8,
    pub aid: Option<u64>,
    pub up_selection_reply: bool,
    pub up_close_reply: bool,
    pub up_close_danmu: bool,
    #[serde(rename = "atomicInt")]
    pub atomic_int: u32,
    pub changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfig {
    pub user: LegacyUser,
    pub line: Option<String>,
    pub limit: u32,
    pub streamers: HashMap<String, LegacyStreamerConfig>,
}

/// 兼容性转换工具
pub struct CompatibilityConverter;

impl CompatibilityConverter {
    /// 检查是否需要进入兼容模式
    /// 如果config.json不存在但config.yaml存在，则需要兼容模式
    pub fn should_convert_old_config() -> Result<bool> {
        let json_path = get_config_json_path()?;
        let yaml_path = get_config_yaml_path()?;

        let json_exists = json_path.exists();
        let yaml_exists = yaml_path.exists();

        // 只有当JSON不存在但YAML存在时，才进入兼容模式
        Ok(!json_exists && yaml_exists)
    }

    /// 程序启动时的兼容性检查和转换
    /// 如果需要兼容模式，自动转换YAML到JSON
    pub async fn startup_with_compatibility() -> Result<bool> {
        if Self::should_convert_old_config()? {
            info!("检测到旧版配置文件，开始兼容性转换...");

            let yaml_path = get_config_yaml_path()?;
            let json_path = get_config_json_path()?;

            // 读取YAML文件
            let yaml_content = fs::read_to_string(&yaml_path)?;

            // 转换为JSON格式
            let json_content = Self::convert_yaml_to_json(&yaml_content).await?;

            // 写入JSON文件
            fs::write(&json_path, &json_content)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 从旧版config.yaml转换到新版config.json格式
    pub async fn convert_yaml_to_json(yaml_content: &str) -> Result<String> {
        // 解析旧版YAML
        let legacy_config: LegacyConfig = serde_yaml::from_str(yaml_content)?;

        // 转换为新版格式
        let user_config = Self::convert_legacy_to_user_config(legacy_config).await?;

        // 序列化为JSON
        let json_content = serde_json::to_string_pretty(&user_config)?;
        Ok(json_content)
    }

    /// 将旧版配置转换为UserConfig格式
    async fn convert_legacy_to_user_config(legacy: LegacyConfig) -> Result<ConfigRoot> {
        let mut template = HashMap::new();

        // 转换每个streamer为template
        for (streamer_name, streamer_config) in legacy.streamers {
            let template_config = TemplateConfig {
                copyright: streamer_config.copyright,
                source: streamer_config.source,
                tid: streamer_config.tid,
                cover: streamer_config.cover,
                title: streamer_config.title,
                desc: streamer_config.desc,
                desc_v2: streamer_config.desc_v2,
                dynamic: streamer_config.dynamic,
                subtitle: streamer_config.subtitle,
                tag: streamer_config.tag,
                videos: {
                    let mut vids = Vec::new();
                    for v in streamer_config.videos {
                        vids.push(VideoInfo {
                            title: v.title,
                            id: v.filename.clone(), // 旧配置的id和filename相同
                            filename: v.filename,
                            desc: v.desc,
                            path: String::new(), // 旧版配置没有path字段，留空
                            finished_at: 0,      // 旧版配置没有finished_at字段，默认0
                            encoding_status: 0,  // 旧版配置没有encoding_status字段，默认0
                            status_desc: String::new(), // 旧版配置没有status_desc字段，留空
                        });
                    }
                    vids
                },
                dtime: streamer_config.dtime,
                open_subtitle: streamer_config.open_subtitle,
                interactive: streamer_config.interactive,
                mission_id: streamer_config.mission_id,
                topic_id: None,   // 旧版配置没有topic_id
                season_id: None,  // 旧版配置没有season_id
                section_id: None, // 旧版配置没有section_id
                is_only_self: 0,  // 旧版配置没有is_only_self
                dolby: streamer_config.dolby,
                lossless_music: streamer_config.lossless_music,
                no_reprint: streamer_config.no_reprint,
                open_elec: streamer_config.open_elec,
                aid: streamer_config.aid,
                up_selection_reply: if streamer_config.up_selection_reply {
                    1
                } else {
                    0
                },
                up_close_reply: if streamer_config.up_close_reply { 1 } else { 0 },
                up_close_danmu: if streamer_config.up_close_danmu { 1 } else { 0 },
                atomic_int: streamer_config.atomic_int,
                watermark: 0, // 默认关闭
            };

            template.insert(streamer_name, template_config);
        }

        // 读取cookie
        let cookie_path = get_old_cookie_file_path()?;
        let (bilibili, user) = validate_cookie_in_old_config(&cookie_path)
            .await
            .map_err(|e| {
                warn!("Cookie验证失败: {}", e);
                e
            })?;

        let user_config = UserConfig {
            user: UserInfo {
                uid: user.uid,
                name: user.username,
                cookie: bilibili.login_info,
            },
            proxy: None, // 旧版配置没有代理设置
            line: legacy.line,
            limit: legacy.limit,
            watermark: 0,
            auto_edit: 0,
            templates: template,
        };

        let mut config_root = ConfigRoot::default();
        config_root.add_user_config(user_config);

        Ok(config_root)
    }
}
