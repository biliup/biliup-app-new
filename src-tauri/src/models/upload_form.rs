use std::collections::HashMap;

use crate::models::TemplateConfig;
use anyhow::Result;
use biliup::bilibili;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, trace};

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

#[derive(PartialEq, Deserialize, Serialize, Debug, Clone)]
pub struct CompatibleCredit {
    pub type_id: u8,
    pub raw_text: String,
    pub biz_id: Option<String>,
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
    pub desc_v2: Option<Vec<CompatibleCredit>>,
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
    pub charging_pay: u8,
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

impl TemplateConfig {
    pub fn from_bilibili_res(value: Value) -> Result<Self> {
        debug!("{}", serde_json::to_string(&value)?);

        let archive = value["archive"].clone();
        trace!("{}", serde_json::to_string(&value)?);
        let mut template_config: Self = serde_json::from_value(archive)?;

        let videos = value["videos"].clone();
        trace!("{}", serde_json::to_string(&videos)?);

        template_config.videos = videos
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("videos should be an array"))?
            .iter()
            .map(|v| {
                let mut v = v.clone();
                let status = v.get("status").cloned();
                if let Some(status) = status {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("encoding_status".to_string(), status);
                        obj.remove("status");
                    }
                }
                Ok(serde_json::from_value(v)?)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(template_config)
    }

    pub fn into_bilibili_form(self) -> BilibiliForm {
        let desc_v2 = self.desc_v2.and_then(|credits| {
            let new_credits = credits
                .into_iter()
                .filter(|item| (item.r#type == 1 || item.r#type == 2) && !item.raw_text.is_empty())
                .map(|c| CompatibleCredit {
                    type_id: c.r#type,
                    biz_id: if c.r#type == 2 && !c.biz_id.is_empty() {
                        Some(c.biz_id)
                    } else {
                        None
                    },
                    raw_text: c.raw_text,
                })
                .collect::<Vec<_>>();
            (!new_credits.is_empty()).then_some(new_credits)
        });

        let desc_format_id = if desc_v2.is_some() { 9999 } else { 0 };

        let extra_fields = {
            let mut map = std::collections::HashMap::new();

            let topic_detail = self.topic_id.map(|id| TopicDetail {
                from_topic_id: Some(id),
                from_source: None,
            });

            if let Some(topic) = self.topic_id {
                map.insert("topic_id".to_string(), json!(topic));
                map.insert("topic_detail".to_string(), json!(topic_detail));
            }

            if self.aid.is_none() && self.season_id.is_some() && self.no_disturbance == 1 {
                map.insert("no_disturbance".to_string(), json!(1));
            }

            map.insert("is_only_self".to_string(), json!(self.is_only_self));

            let watermark = Watermark {
                state: self.watermark,
            };
            map.insert("watermark".to_string(), json!(watermark));

            map.insert("is_360".to_string(), json!(self.is_360));

            map.insert(
                "staffs".to_string(),
                json!(self.staff.map(|staffs| {
                    staffs
                        .into_iter()
                        .filter(|staff| {
                            staff.is_del == 0 && staff.mid != 0 && !staff.title.is_empty()
                        })
                        .collect::<Vec<_>>()
                })),
            );

            Some(map)
        };

        BilibiliForm {
            copyright: self.copyright,
            source: self.source,
            tid: self.tid as u16,
            cover: self.cover.replace("https:", ""),
            title: self.title,
            desc_format_id,
            desc: self.desc,
            desc_v2,
            dynamic: self.dynamic,
            subtitle: Subtitle {
                open: if self.open_subtitle { 1 } else { 0 },
                lan: String::new(), // 默认语言
            },
            tag: self.tag,
            videos: self.videos.into_iter().map(|v| json!(v)).collect(),
            dtime: self.dtime,
            open_subtitle: self.open_subtitle,
            interactive: self.interactive,
            mission_id: self.mission_id,
            dolby: self.dolby,
            lossless_music: self.lossless_music,
            no_reprint: self.no_reprint,
            charging_pay: self.open_elec,
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

        trace!(
            "转换为B站提交表单: \n{}",
            serde_json::to_string_pretty(&studio)?
        );
        Ok(studio)
    }
}
