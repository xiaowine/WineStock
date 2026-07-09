//! 共享配置字段校验函数。
//!
//! 本模块属于 `shared` 层，只为运行配置实体提供 `garde` 自定义规则。
//! 它不承载 core 的 HTTP DTO 或业务字段约束。

/// 校验远端 URL 字段：空字符串表示未配置；非空时必须是 HTTP(S) URL。
pub(crate) fn validate_optional_http_url(value: &str, _: &()) -> garde::Result {
    let value = value.trim();
    if value.is_empty() || value.starts_with("http://") || value.starts_with("https://") {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_empty_or_http_url"))
    }
}
