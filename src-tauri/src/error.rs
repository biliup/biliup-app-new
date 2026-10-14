use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{prefix}: uid={0}", prefix = crate::i18n::text("error.user_not_found"))]
    UserNotFound(u64),

    #[error("{prefix}: {0}", prefix = crate::i18n::text("error.config"))]
    Config(String),

    #[error("{prefix}: {0}", prefix = crate::i18n::text("error.network"))]
    Network(#[from] reqwest::Error),

    #[error("{prefix}: {0}", prefix = crate::i18n::text("error.io"))]
    Io(#[from] std::io::Error),

    #[error("{prefix}: {0}", prefix = crate::i18n::text("error.zip"))]
    Zip(#[from] zip::result::ZipError),

    #[error("{prefix}: {0}", prefix = crate::i18n::text("error.bilibili"))]
    Biliup(String),

    #[error("{prefix}: {0}", prefix = crate::i18n::text("error.internal"))]
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
