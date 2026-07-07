//! 共享字段校验函数。
//!
//! 本模块属于 `shared` 层，提供 DTO 和配置实体复用的 `garde` 自定义规则。
//! 它只表达平台无关的静态字段约束，不访问数据库或平台运行时。

/// 校验字符串裁剪空白后不为空。
pub fn validate_not_blank(value: &str, _: &()) -> garde::Result {
    if value.trim().is_empty() {
        Err(garde::Error::new("must_not_be_blank"))
    } else {
        Ok(())
    }
}

/// 校验可选字符串，存在时要求裁剪空白后非空。
pub fn validate_optional_not_blank(value: &Option<String>, _: &()) -> garde::Result {
    if let Some(value) = value {
        validate_not_blank(value, &())?;
    }

    Ok(())
}

/// 校验角色和权限代码列表，列表项必须是非空短代码。
pub fn validate_code_list(values: &[String], _: &()) -> garde::Result {
    for value in values {
        validate_code(value, &())?;
    }

    Ok(())
}

/// 校验远端 URL 字段：空字符串表示未配置；非空时必须是 HTTP(S) URL。
pub fn validate_optional_http_url(value: &str, _: &()) -> garde::Result {
    let value = value.trim();
    if value.is_empty() || value.starts_with("http://") || value.starts_with("https://") {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_empty_or_http_url"))
    }
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
