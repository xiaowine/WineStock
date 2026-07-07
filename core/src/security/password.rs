//! 密码安全工具。
//!
//! 本模块属于 `security` 前置层，负责密码哈希与校验。
//! 它不查询数据库，也不决定登录或注册业务流程。

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use super::AuthApiError;

/// 根据明文密码和 PHC 格式 Argon2 哈希验证登录请求。
pub(crate) fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// 生成 Argon2 PHC 密码哈希，注册流程之外不得保存明文密码。
pub(crate) fn create_password_hash(password: &str) -> Result<String, AuthApiError> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::fill(&mut salt_bytes).map_err(AuthApiError::Random)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AuthApiError::Internal)?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthApiError::Internal)
}
