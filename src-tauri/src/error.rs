use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("用户未登录或不存在: uid={0}")]
    UserNotFound(u64),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),

    #[error("文件操作失败: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP操作失败: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Bilibili API 错误: {0}")]
    Biliup(String),

    #[error("内部错误: {0}")]
    Internal(anyhow::Error),

    #[error("{0}")]
    Custom(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
