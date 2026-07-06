//! 鉴权安全工具。
//!
//! 本模块属于 core 鉴权层，集中处理密码哈希、refresh token 哈希、安全随机数
//! 和 JWT 时间戳。它不查询数据库，也不构造 HTTP 响应体。

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

use super::error::AuthApiError;

/// 根据明文密码和 PHC 格式 Argon2 哈希验证登录请求。
pub(super) fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// 生成 Argon2 PHC 密码哈希，注册流程之外不得保存明文密码。
pub(super) fn create_password_hash(password: &str) -> Result<String, AuthApiError> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::fill(&mut salt_bytes).map_err(AuthApiError::Random)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AuthApiError::Internal)?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthApiError::Internal)
}

/// refresh token 使用高熵随机明文，数据库只保存其 SHA-256 哈希文本。
pub(super) fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// 返回当前 Unix 时间戳，用于 JWT `iat` 和 `exp`。
pub(super) fn unix_timestamp() -> Result<u64, AuthApiError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AuthApiError::Internal)
}

/// 生成 URL-safe base64 随机文本，用于 JWT `kid`、HS256 对称签名密钥和 refresh token。
pub(super) fn random_urlsafe(length: usize) -> Result<String, getrandom::Error> {
    // 使用 URL-safe base64，便于 key_id 和密钥材料进入 JSON/JWT 相关文本格式。
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
