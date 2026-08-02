//! 用户服务输入归一化。
//!
//! 本模块属于 `users` 业务服务层，负责用户名、搜索文本、状态码和权限代码列表的归一化。
//! 它不访问数据库，也不直接构造 HTTP 响应。

use std::collections::BTreeSet;

use crate::security::AuthApiError;

/// 规范化用户名，避免空白用户名或仅靠首尾空白区分账号。
pub(super) fn normalize_username(username: &str) -> Result<String, AuthApiError> {
    let username = username.trim();
    if username.is_empty() {
        Err(AuthApiError::InvalidRegisterRequest)
    } else {
        Ok(username.to_owned())
    }
}

/// 归一化用户管理修改用的用户名；HTTP 注册和其它已存在用户操作统一要求有效业务文本。
pub(super) fn normalize_existing_username(username: &str) -> Result<String, AuthApiError> {
    let username = username.trim();
    if username.is_empty() {
        Err(AuthApiError::InvalidRequest)
    } else {
        Ok(username.to_owned())
    }
}

/// 归一化可选搜索文本；存在时仍必须是有效业务文本。
pub(super) fn normalize_optional_text(
    value: Option<String>,
) -> Result<Option<String>, AuthApiError> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(AuthApiError::InvalidRequest)
            } else {
                Ok(trimmed.to_owned())
            }
        })
        .transpose()
}

/// 归一化可选用户状态筛选值。
pub(super) fn normalize_optional_status(
    value: Option<String>,
) -> Result<Option<String>, AuthApiError> {
    value
        .map(|value| normalize_status_code(&value).map(ToOwned::to_owned))
        .transpose()
}

/// 归一化并去重权限代码列表；空字符串视为无效请求。
pub(super) fn normalize_permission_codes(values: Vec<String>) -> Result<Vec<String>, AuthApiError> {
    let mut codes = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AuthApiError::InvalidRequest);
        }
        codes.insert(trimmed.to_owned());
    }

    Ok(codes.into_iter().collect())
}

fn normalize_status_code(value: &str) -> Result<&'static str, AuthApiError> {
    match value.trim() {
        "active" => Ok("active"),
        "disabled" => Ok("disabled"),
        _ => Err(AuthApiError::InvalidRequest),
    }
}
