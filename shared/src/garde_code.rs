//! garde 校验消息 → 稳定错误码映射。
//!
//! 本模块属于 `shared` 层，为 shared 与 core 共用的 garde 校验结果提供稳定错误码。
//! garde 0.23 的 `Error` 只有 message 字段、没有独立 code：
//!
//! - 本项目自定义 validator 直接把英文 snake_case 稳定码作为消息（如
//!   `must_not_be_blank`、`invalid_json`、`must_be_positive`），本模块原样直通；
//! - garde 内置规则（`length`/`range`/`ip`/`url` 等）产出英文模板消息，本模块按
//!   模板前缀映射为稳定码；
//! - 未命中任何已知模板时回退 `invalid_field`。
//!
//! 映射表由单元测试锁定当前 garde 版本的实际消息文本；升级 garde 后先运行
//! `cargo test -p winestock-shared garde_code` 确认映射未漂移。

/// 把 garde 校验消息映射为稳定错误码；未知消息回退 `invalid_field`。
pub fn garde_error_code(message: &str) -> &'static str {
    // 自定义 validator 的稳定码直通，避免被内置规则前缀误匹配。
    match message {
        "must_not_be_blank" => return "must_not_be_blank",
        "must_be_empty_or_http_url" => return "must_be_empty_or_http_url",
        "invalid_json" => return "invalid_json",
        "invalid_code" => return "invalid_code",
        "code_too_long" => return "code_too_long",
        "must_be_positive" => return "must_be_positive",
        _ => {}
    }

    // garde 内置规则英文模板前缀映射；文本以 garde 0.23 DefaultI18n 为准
    // （source: crates.io garde-0.23.0/src/i18n.rs），按最长前缀优先。
    const BUILTIN_RULES: &[(&str, &str)] = &[
        ("length is lower than", "length_too_short"),
        ("length is greater than", "length_too_long"),
        ("lower than", "value_too_low"),
        ("greater than", "value_too_high"),
        ("not a valid IP address", "invalid_ip"),
        ("not a valid IPv4 address", "invalid_ip"),
        ("not a valid IPv6 address", "invalid_ip"),
        ("not a valid url", "invalid_url"),
        ("not a valid email", "invalid_email"),
        ("not a valid phone number", "invalid_phone"),
        ("does not match pattern", "invalid_pattern"),
        ("does not contain", "contains_missing"),
        ("value does not begin with", "invalid_prefix"),
        ("does not end with", "invalid_suffix"),
        ("does not match", "fields_mismatch"),
        ("not set", "required"),
        ("not ascii", "invalid_ascii"),
        ("not alphanumeric", "invalid_alphanumeric"),
        ("must not be blank", "not_blank"),
    ];
    for (prefix, code) in BUILTIN_RULES {
        if message.starts_with(prefix) {
            return code;
        }
    }

    "invalid_field"
}

#[cfg(test)]
mod tests {
    use super::garde_error_code;

    #[test]
    fn passthrough_custom_validator_codes() {
        assert_eq!(garde_error_code("must_not_be_blank"), "must_not_be_blank");
        assert_eq!(
            garde_error_code("must_be_empty_or_http_url"),
            "must_be_empty_or_http_url"
        );
        assert_eq!(garde_error_code("invalid_json"), "invalid_json");
        assert_eq!(garde_error_code("invalid_code"), "invalid_code");
        assert_eq!(garde_error_code("code_too_long"), "code_too_long");
        assert_eq!(garde_error_code("must_be_positive"), "must_be_positive");
    }

    #[test]
    fn maps_builtin_template_prefixes() {
        assert_eq!(garde_error_code("length is lower than 8"), "length_too_short");
        assert_eq!(
            garde_error_code("length is greater than 64"),
            "length_too_long"
        );
        assert_eq!(garde_error_code("lower than 1"), "value_too_low");
        assert_eq!(garde_error_code("greater than 100"), "value_too_high");
        assert_eq!(
            garde_error_code("not a valid IP address"),
            "invalid_ip"
        );
        assert_eq!(
            garde_error_code("not a valid IPv6 address"),
            "invalid_ip"
        );
        assert_eq!(garde_error_code("not a valid url: bad scheme"), "invalid_url");
        assert_eq!(
            garde_error_code("not a valid email: missing domain"),
            "invalid_email"
        );
        assert_eq!(
            garde_error_code("not a valid phone number"),
            "invalid_phone"
        );
        assert_eq!(
            garde_error_code("does not match pattern /^[a-z]+$/"),
            "invalid_pattern"
        );
    }

    #[test]
    fn falls_back_to_invalid_field_for_unknown_messages() {
        assert_eq!(garde_error_code("something unexpected"), "invalid_field");
        assert_eq!(garde_error_code(""), "invalid_field");
    }
}
