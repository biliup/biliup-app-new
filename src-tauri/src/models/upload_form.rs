use std::collections::HashMap;

use anyhow::Result;
use biliup::bilibili;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadForm {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub copyright: u8,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub aid: Option<u64>,
    #[serde(default)]
    pub tid: u32,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub dynamic: String,
    #[serde(default)]
    pub videos: Vec<Value>,
    #[serde(default)]
    pub dtime: Option<u32>,
    #[serde(default)]
    pub open_subtitle: bool,
    #[serde(default)]
    pub interactive: u8,
    #[serde(default)]
    pub mission_id: Option<u32>,
    #[serde(default)]
    pub topic_id: Option<u32>,
    #[serde(default)]
    pub season_id: Option<u32>,
    #[serde(default)]
    pub section_id: Option<u32>,
    #[serde(default)]
    pub dolby: u8,
    #[serde(default)]
    pub lossless_music: u8,
    #[serde(default)]
    pub no_reprint: u8,
    #[serde(default)]
    pub open_elec: u8,
    #[serde(default)]
    pub up_selection_reply: u8,
    #[serde(default)]
    pub up_close_reply: u8,
    #[serde(default)]
    pub up_close_danmu: u8,
    #[serde(default)]
    pub is_only_self: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicDetail {
    pub from_topic_id: Option<u32>,
    pub from_source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Watermark {
    pub state: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subtitle {
    pub open: u8,
    pub lan: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BilibiliForm {
    /// 是否转载, 1-自制 2-转载
    pub copyright: u8,
    /// 转载来源
    pub source: String,
    /// 投稿分区
    pub tid: u16,
    /// 视频封面
    pub cover: String,
    /// 视频标题
    pub title: String,
    pub desc_format_id: u32,
    /// 视频简介
    pub desc: String,
    /// 视频简介v2
    pub desc_v2: Option<()>,
    /// 空间动态
    pub dynamic: String,
    pub subtitle: Subtitle,
    /// 视频标签，逗号分隔多个tag
    pub tag: String,
    pub videos: Vec<Value>,
    /// 延时发布时间，距离提交大于4小时，格式为10位时间戳
    pub dtime: Option<u32>,
    pub open_subtitle: bool,
    pub interactive: u8,
    pub mission_id: Option<u32>,
    /// 是否开启杜比音效, 0-关闭 1-开启
    pub dolby: u8,
    /// 是否开启 Hi-Res, 0-关闭 1-开启
    pub lossless_music: u8,
    /// 0-允许转载，1-禁止转载
    pub no_reprint: u8,
    /// 是否开启充电, 0-关闭 1-开启
    pub open_elec: u8,
    /// aid 要追加视频的 avid
    pub aid: Option<u64>,
    /// 是否开启精选评论，仅提交接口为app时可用
    pub up_selection_reply: bool,
    /// 是否关闭评论，仅提交接口为app时可用
    pub up_close_reply: bool,
    /// 是否关闭弹幕，仅提交接口为app时可用
    pub up_close_danmu: bool,
    #[serde(flatten)]
    pub extra_fields: Option<HashMap<String, Value>>,
}

impl UploadForm {
    pub fn from_bilibili_res(value: Value) -> Result<Self> {
        let archive = value["archive"].clone();
        info!("{}", serde_json::to_string(&archive)?);
        let mut upload_form: Self = serde_json::from_value(archive)?;

        let videos = value["videos"].clone();
        info!("{}", serde_json::to_string(&videos)?);
        upload_form.videos = videos
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("videos should be an array"))?
            .to_vec();
        Ok(upload_form)
    }

    pub fn into_bilibili_form(self) -> BilibiliForm {
        let extra_fields = {
            let mut map = std::collections::HashMap::new();

            // 构建topic_detail
            let topic_detail = self.topic_id.map(|id| TopicDetail {
                from_topic_id: Some(id),
                from_source: None,
            });

            if let Some(topic) = self.topic_id {
                map.insert("topic_id".to_string(), json!(topic));
                map.insert("topic_detail".to_string(), json!(topic_detail));
            }

            map.insert("is_only_self".to_string(), json!(self.is_only_self));

            // todo!
            {
                map.insert("is_360".to_string(), json!(-1)); // 默认不是360度视频

                let watermark = Watermark { state: 1 }; // 默认开启水印
                map.insert("watermark".to_string(), json!(watermark));
            }

            Some(map)
        };

        BilibiliForm {
            copyright: self.copyright,
            source: self.source,
            tid: self.tid as u16,
            cover: self.cover.replace("https:", ""),
            title: self.title,
            desc_format_id: 0, // 默认值
            desc: self.desc,
            desc_v2: None,
            dynamic: self.dynamic,
            subtitle: Subtitle {
                open: if self.open_subtitle { 1 } else { 0 },
                lan: String::new(), // 默认语言
            },
            tag: self.tag,
            videos: self.videos,
            dtime: self.dtime,
            open_subtitle: self.open_subtitle,
            interactive: self.interactive,
            mission_id: self.mission_id,
            dolby: self.dolby,
            lossless_music: self.lossless_music,
            no_reprint: self.no_reprint,
            open_elec: self.open_elec,
            aid: self.aid,
            up_selection_reply: self.up_selection_reply > 0,
            up_close_reply: self.up_close_reply > 0,
            up_close_danmu: self.up_close_danmu > 0,
            extra_fields,
        }
    }
}

impl BilibiliForm {
    pub fn try_into_studio(self) -> Result<bilibili::Studio> {
        let self_str = serde_json::to_string(&self)?;
        let studio: biliup::bilibili::Studio = serde_json::from_str(&self_str)?;
        Ok(studio)
    }
}
