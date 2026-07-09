//! 共享文本字段校验函数。
//!
//! 本模块属于 `shared` 层，只放平台无关、无业务语义的基础文本规则。
//! 它不承载 HTTP DTO、权限代码格式或具体业务字段约束。

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
