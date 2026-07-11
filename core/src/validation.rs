//! core 业务字段校验函数。
//!
//! 本模块属于 `core axum library` 层，为 HTTP DTO 和 repository 写库输入提供静态字段约束。
//! 它只表达业务字段格式，不访问数据库、平台 shell 或 Axum 路由状态。

pub(crate) use winestock_shared::text_validation::{
    validate_not_blank, validate_optional_not_blank,
};

/// 校验必填字符串包含合法 JSON；只检查语法，不限定顶层值类型。
pub(crate) fn validate_json_text(value: &str, _: &()) -> garde::Result {
    serde_json::from_str::<serde_json::Value>(value)
        .map(|_| ())
        .map_err(|_| garde::Error::new("invalid_json"))
}

/// 校验可选字符串在存在时包含合法 JSON。
pub(crate) fn validate_optional_json_text(value: &Option<String>, _: &()) -> garde::Result {
    match value {
        Some(value) => validate_json_text(value, &()),
        None => Ok(()),
    }
}

/// 校验权限代码列表，列表项必须是非空短代码。
pub(crate) fn validate_code_list(values: &[String], _: &()) -> garde::Result {
    for value in values {
        validate_code(value, &())?;
    }

    Ok(())
}

fn validate_code(value: &str, _: &()) -> garde::Result {
    validate_not_blank(value, &())?;
    if value.len() > 128 {
        return Err(garde::Error::new("code_too_long"));
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
    {
        Ok(())
    } else {
        Err(garde::Error::new("invalid_code"))
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_json_text, validate_optional_json_text};

    #[test]
    fn garde_json_validators_accept_valid_json_and_reject_invalid_text() {
        assert!(validate_json_text(r#"{"file_id":1}"#, &()).is_ok());
        assert!(validate_json_text("not-json", &()).is_err());
        assert!(validate_optional_json_text(&None, &()).is_ok());
        assert!(validate_optional_json_text(&Some("[1,2,3]".to_owned()), &()).is_ok());
        assert!(validate_optional_json_text(&Some("{".to_owned()), &()).is_err());
    }
}
