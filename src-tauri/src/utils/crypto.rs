use base64::{Engine as _, engine::general_purpose};

/// Base64 编码
pub fn encode_base64(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}
