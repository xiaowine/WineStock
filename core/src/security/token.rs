//! 令牌与安全随机工具。
//!
//! 本模块属于 `security` 前置层，负责 refresh token 哈希、安全随机文本和时间戳工具。
//! 它不持有数据库连接，也不承载具体业务流程。

use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

use super::AuthApiError;

/// refresh token 使用高熵随机明文，数据库只保存其 SHA-256 哈希文本。
pub(crate) fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// 返回当前 Unix 时间戳，用于 JWT `iat` 和 `exp`。
pub(crate) fn unix_timestamp() -> Result<u64, AuthApiError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AuthApiError::Internal)
}

/// 生成 URL-safe base64 随机文本，用于 JWT `kid`、HS256 对称签名密钥和 refresh token。
pub(crate) fn random_urlsafe(length: usize) -> Result<String, getrandom::Error> {
    // 使用 URL-safe base64，便于 key_id 和密钥材料进入 JSON/JWT 相关文本格式。
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
