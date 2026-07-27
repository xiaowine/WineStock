//! 外部物品资料查询用例与候选资料归一化。
//!
//! 本模块不写数据库；第三方原始协议由 `external` 适配器拥有。

use std::collections::HashSet;

use serde_json::Value;
use url::Url;

use crate::{
    external::{LcscLookupError, LcscProductRecord},
    state::CoreState,
    stock::controller::{ItemLookupSource, LcscItemLookupResponse, LcscLookupParameterResponse},
};

use super::StockApiError;

const MAX_PRODUCT_CODE_LENGTH: usize = 32;
const MAX_TEXT_LENGTH: usize = 1024;
const MAX_PARAMETER_NAME_LENGTH: usize = 128;
const MAX_PARAMETERS: usize = 64;

const KNOWN_ATTRIBUTE_NAMES: &[&str] = &[
    "LCSC Part Name",
    "Supplier Part",
    "Manufacturer",
    "Manufacturer Part",
    "Supplier Footprint",
    "Datasheet",
];
const INTERNAL_ATTRIBUTE_NAMES: &[&str] = &[
    "Supplier",
    "Add into BOM",
    "Convert to PCB",
    "Symbol",
    "Designator",
    "Footprint",
    "3D Model",
    "3D Model Title",
    "3D Model Transform",
    "Name",
    "JLCPCB Part Class",
];

pub(crate) async fn lookup_lcsc_item(
    state: &CoreState,
    product_code: &str,
) -> Result<LcscItemLookupResponse, StockApiError> {
    let product_code = normalize_product_code(product_code)?;
    let (records, default_price) = state
        .external_catalog()
        .lcsc()
        .lookup(&product_code)
        .await
        .map_err(map_lookup_error)?;
    normalize_lookup_result(&product_code, records, default_price)
}

fn normalize_product_code(product_code: &str) -> Result<String, StockApiError> {
    let normalized = product_code.trim().to_ascii_uppercase();
    let digits = normalized
        .strip_prefix('C')
        .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()));
    if digits.is_none() || normalized.len() > MAX_PRODUCT_CODE_LENGTH {
        return Err(StockApiError::InvalidLcscProductCode);
    }
    Ok(normalized)
}

fn normalize_lookup_result(
    product_code: &str,
    records: Vec<LcscProductRecord>,
    default_price: Option<f64>,
) -> Result<LcscItemLookupResponse, StockApiError> {
    let record = exact_record(product_code, records)?;

    let part_name = attribute_text(&record, "LCSC Part Name");
    let manufacturer_part = attribute_text(&record, "Manufacturer Part");
    let parameters = additional_parameters(&record);
    let name = manufacturer_part
        .clone()
        .or_else(|| part_name.clone())
        .unwrap_or_else(|| product_code.to_owned());
    let datasheet_url = attribute_text(&record, "Datasheet").filter(|value| valid_http_url(value));

    Ok(LcscItemLookupResponse {
        source: ItemLookupSource::Lcsc,
        product_code: product_code.to_owned(),
        name,
        description: record
            .description
            .as_deref()
            .map(|value| truncate_chars(value.trim(), MAX_TEXT_LENGTH))
            .filter(|value| !value.is_empty())
            .or(part_name),
        manufacturer: attribute_text(&record, "Manufacturer"),
        manufacturer_part,
        footprint: attribute_text(&record, "Supplier Footprint"),
        datasheet_url,
        image_url: record.image_url,
        default_price,
        parameters,
    })
}

fn exact_record(
    product_code: &str,
    records: Vec<LcscProductRecord>,
) -> Result<LcscProductRecord, StockApiError> {
    let mut matches = records.into_iter().filter(|record| {
        record_product_code(record)
            .is_some_and(|candidate| candidate.trim().eq_ignore_ascii_case(product_code))
    });
    let record = matches.next().ok_or(StockApiError::LcscProductNotFound)?;
    if matches.next().is_some() {
        return Err(StockApiError::LcscInvalidResponse);
    }

    Ok(record)
}

fn record_product_code(record: &LcscProductRecord) -> Option<&str> {
    record.product_code.as_deref().or_else(|| {
        record
            .attributes
            .get("Supplier Part")
            .and_then(Value::as_str)
    })
}

fn attribute_text(record: &LcscProductRecord, name: &str) -> Option<String> {
    record
        .attributes
        .get(name)
        .and_then(scalar_text)
        .map(|value| truncate_chars(value.trim(), MAX_TEXT_LENGTH))
        .filter(|value| !value.is_empty())
}

fn additional_parameters(record: &LcscProductRecord) -> Vec<LcscLookupParameterResponse> {
    let excluded = KNOWN_ATTRIBUTE_NAMES
        .iter()
        .chain(INTERNAL_ATTRIBUTE_NAMES)
        .copied()
        .collect::<HashSet<_>>();
    let mut parameters = record
        .attributes
        .iter()
        .filter(|(name, _)| !excluded.contains(name.as_str()))
        .filter_map(|(name, value)| {
            let name = truncate_chars(name.trim(), MAX_PARAMETER_NAME_LENGTH);
            let value = scalar_text(value)
                .map(|value| truncate_chars(value.trim(), MAX_TEXT_LENGTH))
                .filter(|value| !value.is_empty())?;
            (!name.is_empty()).then_some(LcscLookupParameterResponse { name, value })
        })
        .collect::<Vec<_>>();
    parameters.sort_by(|left, right| left.name.cmp(&right.name));
    parameters.truncate(MAX_PARAMETERS);
    parameters
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn valid_http_url(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
}

/// 按 UTF-16 code unit 上限安全裁剪，与 HTTP DTO 的 `length(utf16, ...)` 契约一致。
///
/// 遍历 Unicode scalar value 并累加 `char::len_utf16()`，加入下一个字符会超过上限时停止，
/// 因此不会在代理对中间切断，也不会生成非法字符串。
fn truncate_chars(value: &str, max_utf16: usize) -> String {
    let mut used = 0usize;
    let mut end = value.len();
    for (index, ch) in value.char_indices() {
        let width = ch.len_utf16();
        if used + width > max_utf16 {
            end = index;
            break;
        }
        used += width;
    }
    value[..end].to_owned()
}

fn map_lookup_error(error: LcscLookupError) -> StockApiError {
    match error {
        LcscLookupError::Busy => StockApiError::LcscLookupBusy,
        LcscLookupError::Timeout => StockApiError::LcscLookupTimeout,
        LcscLookupError::Failed => StockApiError::LcscLookupFailed,
        LcscLookupError::InvalidResponse => StockApiError::LcscInvalidResponse,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;

    #[test]
    fn normalizes_single_matching_record_and_filters_internal_fields() {
        let response = normalize_lookup_result(
            "C2983288",
            vec![LcscProductRecord {
                product_code: Some("C2983288".to_owned()),
                description: Some("顶层描述".to_owned()),
                image_url: None,
                attributes: HashMap::from([
                    ("LCSC Part Name".to_owned(), json!("旋转编码开关")),
                    ("Supplier Part".to_owned(), json!("C2983288")),
                    ("Manufacturer".to_owned(), json!("SM Switch")),
                    ("Manufacturer Part".to_owned(), json!("BER-04")),
                    ("Supplier Footprint".to_owned(), json!("插件")),
                    ("Datasheet".to_owned(), json!("https://example.com/a.pdf")),
                    ("Symbol".to_owned(), json!("private-id")),
                    ("Operating Temperature".to_owned(), json!("-40℃~+85℃")),
                ]),
            }],
            Some(9.91),
        )
        .expect("record should normalize");

        assert_eq!(response.name, "BER-04");
        assert_eq!(response.description.as_deref(), Some("顶层描述"));
        assert_eq!(response.manufacturer.as_deref(), Some("SM Switch"));
        assert_eq!(response.default_price, Some(9.91));
        assert_eq!(response.parameters.len(), 1);
        assert_eq!(response.parameters[0].name, "Operating Temperature");
    }

    #[test]
    fn validates_product_code_and_requires_unique_match() {
        assert_eq!(normalize_product_code(" c2983288 ").unwrap(), "C2983288");
        assert!(matches!(
            normalize_product_code("LC2983288"),
            Err(StockApiError::InvalidLcscProductCode)
        ));
        assert!(matches!(
            normalize_lookup_result(
                "C1",
                vec![LcscProductRecord {
                    product_code: Some("C10".to_owned()),
                    description: None,
                    image_url: None,
                    attributes: HashMap::new(),
                }],
                None,
            ),
            Err(StockApiError::LcscProductNotFound)
        ));

        let supplier_part_match = normalize_lookup_result(
            "C1",
            vec![LcscProductRecord {
                product_code: None,
                description: None,
                image_url: None,
                attributes: HashMap::from([("Supplier Part".to_owned(), json!("C1"))]),
            }],
            None,
        )
        .expect("Supplier Part should provide the exact product code fallback");
        assert_eq!(supplier_part_match.product_code, "C1");
    }

    #[test]
    fn maps_every_upstream_failure_to_a_stable_stock_error() {
        assert!(matches!(
            map_lookup_error(LcscLookupError::Busy),
            StockApiError::LcscLookupBusy
        ));
        assert!(matches!(
            map_lookup_error(LcscLookupError::Timeout),
            StockApiError::LcscLookupTimeout
        ));
        assert!(matches!(
            map_lookup_error(LcscLookupError::Failed),
            StockApiError::LcscLookupFailed
        ));
        assert!(matches!(
            map_lookup_error(LcscLookupError::InvalidResponse),
            StockApiError::LcscInvalidResponse
        ));
    }

    #[test]
    fn truncate_chars_counts_utf16_code_units_without_splitting_characters() {
        // ASCII 恰好达到上限。
        assert_eq!(truncate_chars("abcdef", 3), "abc");

        // 中文按 1 个 UTF-16 code unit 计数，字节数远超上限也不会被裁剪。
        let chinese = "中".repeat(4);
        assert_eq!(truncate_chars(&chinese, 4), chinese);
        assert_eq!(truncate_chars(&chinese, 3), "中".repeat(3));

        // 增补平面字符占 2 个 UTF-16 code unit；上限为奇数时不切断代理对。
        assert_eq!(truncate_chars("😀😀", 4), "😀😀");
        assert_eq!(truncate_chars("😀😀", 3), "😀");
        assert_eq!(truncate_chars("😀😀", 1), "");
    }
}
