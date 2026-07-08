//! 库存服务输入归一化。
//!
//! 本模块属于 `stock` 业务服务层，负责跨库存用例复用的文本、URL、数值、ID 和 JSON 解析规则。
//! 它不访问数据库，也不直接构造 HTTP 响应。

use serde_json::Value;
use winestock_shared::validation::validate_not_blank;

use super::StockApiError;

/// 解析数据库中的 options_json；损坏数据统一映射为 `InvalidRequest`。
pub(super) fn parse_options_json(
    value: Option<String>,
) -> Result<Option<Vec<String>>, StockApiError> {
    value
        .map(|value| serde_json::from_str(&value).map_err(|_| StockApiError::InvalidRequest))
        .transpose()
}

/// 归一化必填文本；裁剪后为空时返回 `InvalidRequest`。
pub(super) fn normalize_required_text(value: &str) -> Result<String, StockApiError> {
    validate_not_blank(value, &()).map_err(|_| StockApiError::InvalidRequest)?;
    Ok(value.trim().to_owned())
}

/// 把 SQLite 0/非 0 布尔值恢复为 Rust bool。
pub(super) fn sqlite_bool(value: i32) -> bool {
    value != 0
}

/// 归一化可选文本；存在时仍必须是有效业务文本。
pub(super) fn normalize_optional_text(
    value: Option<String>,
) -> Result<Option<String>, StockApiError> {
    value
        .map(|value| normalize_required_text(&value))
        .transpose()
}

/// 校验模板 URL 字段必须是 HTTP 或 HTTPS 链接。
pub(super) fn validate_http_url(value: &str) -> Result<(), StockApiError> {
    let value = value.trim();
    let Some(rest) = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
    else {
        return Err(StockApiError::InvalidRequest);
    };
    if rest.is_empty() || rest.chars().any(char::is_whitespace) {
        Err(StockApiError::InvalidRequest)
    } else {
        Ok(())
    }
}

/// 校验可选数值必须有限且非负。
pub(super) fn validate_non_negative(value: Option<f64>) -> Result<Option<f64>, StockApiError> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        Err(StockApiError::InvalidRequest)
    } else {
        Ok(value)
    }
}

/// 校验数量必须是有限正数。
pub(super) fn validate_positive(value: f64) -> Result<f64, StockApiError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(StockApiError::InvalidRequest)
    }
}

/// 校验数据库实体 ID 必须为正数。
pub(super) fn positive_id(value: i64) -> Result<i64, StockApiError> {
    if value > 0 {
        Ok(value)
    } else {
        Err(StockApiError::InvalidRequest)
    }
}

/// 校验替代料优先级必须为正数。
pub(super) fn positive_i32(value: i32) -> Result<i32, StockApiError> {
    if value > 0 {
        Ok(value)
    } else {
        Err(StockApiError::InvalidRequest)
    }
}

/// 解析入库明细扩展属性；未传值等价于空对象，非对象 JSON 会被拒绝。
pub(super) fn parse_attribute_object(
    json: Option<&str>,
) -> Result<serde_json::Map<String, Value>, StockApiError> {
    let Some(json) = json else {
        return Ok(serde_json::Map::new());
    };
    let value: Value = serde_json::from_str(json).map_err(|_| StockApiError::InvalidRequest)?;
    value
        .as_object()
        .cloned()
        .ok_or(StockApiError::InvalidRequest)
}
