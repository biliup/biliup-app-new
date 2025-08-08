use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uid: u64,
    pub username: String,
    pub avatar: String,
}

impl Default for User {
    fn default() -> Self {
        Self {
            uid: 0,
            username: "未登录用户".to_string(),
            avatar: "default_profile_picture.png".to_string(),
        }
    }
}

impl User {
    pub fn new(uid: u64, username: String, avatar: String) -> Self {
        Self {
            uid,
            username,
            avatar,
        }
    }
}
