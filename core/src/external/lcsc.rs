//! 立创商城商品查询接口适配器。
//!
//! 本模块只理解固定上游请求与原始响应，不公开 WineStock HTTP DTO，也不写数据库。

use std::{collections::HashMap, error::Error, fmt, sync::Arc, time::Duration};

use reqwest::{redirect::Policy, Client};
use serde::{
    de::{DeserializeOwned, Deserializer},
    Deserialize, Serialize,
};
use serde_json::Value;
use tokio::sync::Semaphore;
use url::Url;

const LCSC_PHONE_QUERY_URL: &str = "https://so.szlcsc.com/phone/global/query";
const LCSC_IMAGE_HOST: &str = "alimg.szlcsc.com";
const LCSC_IMAGE_PATH_PREFIXES: [&str; 2] = [
    "/upload/public/product/",
    "/upload/public/brand/product/certificate/",
];
const LCSC_BREVIARY_IMAGE_PATH_PREFIX: &str = "/upload/public/product/breviary/";
const LCSC_SOURCE_IMAGE_PATH_PREFIX: &str = "/upload/public/product/source/";
const LCSC_IMAGE_LIST_SEPARATOR: &str = "<$>";
const LCSC_DATASHEET_HOST: &str = "atta.szlcsc.com";
const LCSC_DATASHEET_PATH_PREFIX: &str = "/upload/public/pdf/";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_CONCURRENT_LOOKUPS: usize = 4;

/// 构建立创查询 client 失败。
#[derive(Debug)]
pub struct ExternalCatalogBootstrapError {
    source: reqwest::Error,
}

impl fmt::Display for ExternalCatalogBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "无法初始化外部商品资料查询 client")
    }
}

impl Error for ExternalCatalogBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// 立创上游查询错误；WineStock 稳定 HTTP 错误由 stock service 继续映射。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LcscLookupError {
    Busy,
    Timeout,
    Failed,
    InvalidResponse,
}

/// 单个立创商品的原始业务记录。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LcscProductRecord {
    /// 商城搜索结果声明的立创客编。
    pub(crate) product_code: Option<String>,
    /// 商城搜索结果中的商品描述。
    pub(crate) description: Option<String>,
    /// 已通过 HTTPS、主机和路径白名单校验的商品图片地址。
    pub(crate) image_url: Option<String>,
    /// 上游器件属性；业务层只投影允许公开的字段。
    pub(crate) attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct LcscLookupClient {
    client: Client,
    search_endpoint: String,
    permits: Arc<Semaphore>,
}

impl LcscLookupClient {
    pub(crate) fn build() -> Result<Self, ExternalCatalogBootstrapError> {
        Self::build_inner(LCSC_PHONE_QUERY_URL.to_owned(), true)
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(
        search_endpoint: String,
    ) -> Result<Self, ExternalCatalogBootstrapError> {
        Self::build_inner(search_endpoint, false)
    }

    fn build_inner(
        search_endpoint: String,
        https_only: bool,
    ) -> Result<Self, ExternalCatalogBootstrapError> {
        let client = Client::builder()
            .use_preconfigured_tls(webpki_tls_config())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .https_only(https_only)
            .redirect(Policy::none())
            .pool_max_idle_per_host(2)
            .build()
            .map_err(|source| ExternalCatalogBootstrapError { source })?;

        Ok(Self {
            client,
            search_endpoint,
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_LOOKUPS)),
        })
    }

    /// 查询单个规范化客编；请求槽只覆盖完整上游请求和响应读取。
    pub(crate) async fn lookup(
        &self,
        product_code: &str,
    ) -> Result<(Vec<LcscProductRecord>, Option<f64>), LcscLookupError> {
        let _permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| LcscLookupError::Busy)?;
        let records = self.query_with_permit(product_code, 10).await?;
        let default_price = records
            .iter()
            .filter(|record| exact_product_code(record, product_code))
            .filter_map(reference_price)
            .next();
        Ok((
            records.into_iter().map(normalize_product_record).collect(),
            default_price,
        ))
    }

    /// 查询单个规范化客编并等待共享请求槽；批量查询使用此入口避免把排队请求报为忙。
    pub(crate) async fn lookup_wait(
        &self,
        product_code: &str,
    ) -> Result<(Vec<LcscProductRecord>, Option<f64>), LcscLookupError> {
        let _permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| LcscLookupError::Busy)?;
        let records = self.query_with_permit(product_code, 10).await?;
        let default_price = records
            .iter()
            .filter(|record| exact_product_code(record, product_code))
            .filter_map(reference_price)
            .next();
        Ok((
            records.into_iter().map(normalize_product_record).collect(),
            default_price,
        ))
    }

    /// 一次查询多个规范化客编；调用方必须自行按客编精确筛选返回记录。
    pub(crate) async fn lookup_batch_wait(
        &self,
        product_codes: &[String],
    ) -> Result<(Vec<LcscProductRecord>, HashMap<String, f64>), LcscLookupError> {
        let _permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| LcscLookupError::Busy)?;
        let keyword = product_codes.join(" ");
        let records = self
            .query_with_permit(&keyword, product_codes.len() as u8)
            .await?;
        let prices = records
            .iter()
            .filter_map(|record| {
                let code = record.product.product_code.as_ref()?.trim();
                let price = reference_price(record)?;
                Some((code.to_ascii_uppercase(), price))
            })
            .collect();
        Ok((
            records.into_iter().map(normalize_product_record).collect(),
            prices,
        ))
    }

    async fn query_with_permit(
        &self,
        keyword: &str,
        page_size: u8,
    ) -> Result<Vec<LcscPhoneProductRecord>, LcscLookupError> {
        let request = LcscPhoneQueryRequest {
            keyword,
            page_size,
            current_page: 1,
            search_source: "main_so",
            async_request: false,
        };
        let response = self
            .client
            .post(&self.search_endpoint)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&request)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        read_search_records(response).await
    }
}

/// 使用内置 webpki 根证书的 rustls 配置。
/// reqwest 默认的 rustls-platform-verifier 在 Android 上依赖未接入的 JNI 初始化，
/// 首次 TLS 握手会 panic 并使该连接以空响应中断；上游只有固定公网域名，内置根证书足够且三端一致。
fn webpki_tls_config() -> rustls::ClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    // 只声明 http/1.1：工作区 reqwest 未启用 http2 feature，若 ALPN 协商出 h2，hyper 会直接 panic。
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config
}

async fn read_search_records(
    mut response: reqwest::Response,
) -> Result<Vec<LcscPhoneProductRecord>, LcscLookupError> {
    if !response.status().is_success() {
        return Err(LcscLookupError::Failed);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
        bytes.extend_from_slice(&chunk);
    }

    let body: LcscPhoneQueryResponse =
        serde_json::from_slice(&bytes).map_err(|_| LcscLookupError::InvalidResponse)?;
    if body.code != 200 || !body.ok {
        return Err(LcscLookupError::InvalidResponse);
    }
    let search_result = body
        .result
        .and_then(|result| result.search_result)
        .ok_or(LcscLookupError::InvalidResponse)?;

    Ok(search_result.product_record_list)
}

fn exact_product_code(record: &LcscPhoneProductRecord, product_code: &str) -> bool {
    record
        .product
        .product_code
        .as_deref()
        .is_some_and(|candidate| candidate.trim().eq_ignore_ascii_case(product_code))
}

fn normalize_product_record(record: LcscPhoneProductRecord) -> LcscProductRecord {
    let product = record.product;
    let mut attributes = record.parameters;
    insert_attribute(
        &mut attributes,
        "LCSC Part Name",
        first_text(record.light_product_name, product.product_name.clone()),
    );
    insert_attribute(
        &mut attributes,
        "Supplier Part",
        product.product_code.clone(),
    );
    insert_attribute(
        &mut attributes,
        "Manufacturer",
        first_text(
            record.light_brand_name,
            product.product_grade_plate_name.clone(),
        ),
    );
    insert_attribute(
        &mut attributes,
        "Manufacturer Part",
        first_text(record.light_product_model, product.product_model.clone()),
    );
    insert_attribute(
        &mut attributes,
        "Supplier Footprint",
        first_text(record.light_standard, product.encapsulation_model.clone()),
    );
    insert_attribute(
        &mut attributes,
        "Datasheet",
        datasheet_url(&product.file_groups),
    );

    let image_url = preferred_image_url(&product);
    LcscProductRecord {
        product_code: product.product_code,
        description: first_text(record.light_product_intro, product.product_name),
        image_url,
        attributes,
    }
}

fn insert_attribute(attributes: &mut HashMap<String, Value>, name: &str, value: Option<String>) {
    if let Some(value) = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    {
        attributes.insert(name.to_owned(), Value::String(value));
    }
}

fn first_text(primary: Option<String>, fallback: Option<String>) -> Option<String> {
    primary
        .filter(|value| !value.trim().is_empty())
        .or_else(|| fallback.filter(|value| !value.trim().is_empty()))
}

/// 只依据查询响应中的首图字段选择地址，不在资料查询阶段下载图片。
///
/// 立创未提供独立 source 字段；首张 breviary 的稳定路径可归一化为 source。
/// 无法生成受控 source 地址时依次退回首张 breviary 和 bigImageUrl。
fn preferred_image_url(product: &LcscPhoneProduct) -> Option<String> {
    let lucene_first = product
        .lucene_breviary_image_urls
        .as_deref()
        .and_then(|urls| urls.split(LCSC_IMAGE_LIST_SEPARATOR).next())
        .map(str::trim)
        .filter(|url| !url.is_empty());
    let first_breviary = lucene_first.and_then(controlled_image_url).or_else(|| {
        product
            .breviary_image_url
            .as_deref()
            .and_then(controlled_image_url)
    });

    first_breviary
        .as_deref()
        .and_then(source_image_url)
        .or(first_breviary)
        .or_else(|| {
            product
                .big_image_url
                .as_deref()
                .and_then(controlled_image_url)
        })
}

fn source_image_url(breviary_url: &str) -> Option<String> {
    let mut url = Url::parse(breviary_url).ok()?;
    let suffix = url.path().strip_prefix(LCSC_BREVIARY_IMAGE_PATH_PREFIX)?;
    url.set_path(&format!("{LCSC_SOURCE_IMAGE_PATH_PREFIX}{suffix}"));
    controlled_image_url(url.as_str())
}

fn reference_price(record: &LcscPhoneProductRecord) -> Option<f64> {
    let product = &record.product;
    let available_stock = product
        .valid_stock_number
        .or(product.stock_number)
        .unwrap_or_default();
    if available_stock <= 0 {
        return None;
    }
    product
        .price_list
        .iter()
        .filter(|tier| {
            tier.start_purchased_number >= 1
                && tier.product_price.is_finite()
                && tier.product_price > 0.0
        })
        .min_by_key(|tier| tier.start_purchased_number)
        .map(|tier| tier.product_price)
}

fn controlled_image_url(source: &str) -> Option<String> {
    let url = Url::parse(source).ok()?;
    (url.scheme() == "https"
        && url.host_str() == Some(LCSC_IMAGE_HOST)
        && url.username().is_empty()
        && url.password().is_none()
        && LCSC_IMAGE_PATH_PREFIXES
            .iter()
            .any(|prefix| url.path().starts_with(prefix))
        && url.query().is_none()
        && url.fragment().is_none())
    .then(|| url.into())
}

fn datasheet_url(groups: &[LcscPhoneFileGroup]) -> Option<String> {
    let path = groups
        .iter()
        .find(|group| group.file_type == "pdf_property")?
        .details
        .iter()
        .filter_map(|detail| detail.file_url.as_deref())
        .find(|path| path.starts_with(LCSC_DATASHEET_PATH_PREFIX))?;
    let mut url = Url::parse(&format!("https://{LCSC_DATASHEET_HOST}")).ok()?;
    url.set_path(path);
    Some(url.into())
}

fn map_reqwest_error(error: reqwest::Error) -> LcscLookupError {
    if error.is_timeout() {
        LcscLookupError::Timeout
    } else {
        LcscLookupError::Failed
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LcscPhoneQueryRequest<'a> {
    keyword: &'a str,
    page_size: u8,
    current_page: u8,
    search_source: &'static str,
    async_request: bool,
}

#[derive(Debug, Deserialize)]
struct LcscPhoneQueryResponse {
    code: i64,
    #[serde(default = "default_query_ok")]
    ok: bool,
    result: Option<LcscPhoneQueryResult>,
}

fn default_query_ok() -> bool {
    // 立创当前真实响应省略 ok 字段；只有明确返回 false 时才视为失败。
    true
}

#[derive(Debug, Deserialize)]
struct LcscPhoneQueryResult {
    #[serde(rename = "searchResult")]
    search_result: Option<LcscPhoneSearchResult>,
}

#[derive(Debug, Deserialize)]
struct LcscPhoneSearchResult {
    #[serde(default, rename = "productRecordList")]
    product_record_list: Vec<LcscPhoneProductRecord>,
}

#[derive(Debug, Deserialize)]
struct LcscPhoneProductRecord {
    #[serde(rename = "productVO")]
    product: LcscPhoneProduct,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "lightBrandName"
    )]
    light_brand_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "lightProductIntro"
    )]
    light_product_intro: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "lightProductName"
    )]
    light_product_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "lightProductModel"
    )]
    light_product_model: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "lightStandard"
    )]
    light_standard: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_map_or_empty",
        rename = "paramLinkedMap"
    )]
    parameters: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct LcscPhoneProduct {
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "productCode"
    )]
    product_code: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "productName"
    )]
    product_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "productGradePlateName"
    )]
    product_grade_plate_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "encapsulationModel"
    )]
    encapsulation_model: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "productModel"
    )]
    product_model: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "breviaryImageUrl"
    )]
    breviary_image_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "bigImageUrl"
    )]
    big_image_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "luceneBreviaryImageUrls"
    )]
    lucene_breviary_image_urls: Option<String>,
    #[serde(default, deserialize_with = "deserialize_i64", rename = "stockNumber")]
    stock_number: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_i64",
        rename = "validStockNumber"
    )]
    valid_stock_number: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_vec_or_empty",
        rename = "productPriceList"
    )]
    price_list: Vec<LcscPhonePriceTier>,
    #[serde(
        default,
        deserialize_with = "deserialize_vec_or_empty",
        rename = "fileTypeVOList"
    )]
    file_groups: Vec<LcscPhoneFileGroup>,
}

#[derive(Debug, Deserialize)]
struct LcscPhonePriceTier {
    #[serde(
        deserialize_with = "deserialize_number_i64",
        rename = "startPurchasedNumber"
    )]
    start_purchased_number: i64,
    #[serde(deserialize_with = "deserialize_number_f64", rename = "productPrice")]
    product_price: f64,
}

#[derive(Debug, Deserialize)]
struct LcscPhoneFileGroup {
    #[serde(default, deserialize_with = "deserialize_string", rename = "fileType")]
    file_type: String,
    #[serde(default, rename = "detailVOList")]
    details: Vec<LcscPhoneFileDetail>,
}

#[derive(Debug, Deserialize)]
struct LcscPhoneFileDetail {
    #[serde(
        default,
        deserialize_with = "deserialize_string_option",
        rename = "fileUrl"
    )]
    file_url: Option<String>,
}

fn deserialize_vec_or_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;
    let Value::Array(values) = value else {
        return Ok(Vec::new());
    };
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

fn deserialize_map_or_empty<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: DeserializeOwned + Eq + std::hash::Hash,
    V: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;
    let Value::Object(values) = value else {
        return Ok(HashMap::new());
    };
    Ok(values
        .into_iter()
        .filter_map(|(key, value)| {
            Some((
                serde_json::from_value(Value::String(key)).ok()?,
                serde_json::from_value(value).ok()?,
            ))
        })
        .collect())
}

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Value::deserialize(deserializer)? {
        Value::String(value) => value,
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => String::new(),
    })
}

fn deserialize_string_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Value::deserialize(deserializer)? {
        Value::Null => None,
        Value::String(value) => Some(value),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    })
}

fn deserialize_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(match value {
        Value::Null => None,
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    })
}

fn deserialize_number_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(deserialize_i64(deserializer)?.unwrap_or_default())
}

fn deserialize_number_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(match value {
        Value::Number(value) => value.as_f64().unwrap_or_default(),
        Value::String(value) => value.trim().parse().unwrap_or_default(),
        _ => 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_current_lcsc_response_that_omits_ok_field() {
        let response: LcscPhoneQueryResponse =
            serde_json::from_value(serde_json::json!({ "code": 200, "result": {} }))
                .expect("response without ok should deserialize");
        assert!(response.ok);
    }

    #[test]
    fn accepts_nullable_and_string_numeric_upstream_fields() {
        let record: LcscPhoneProductRecord = serde_json::from_value(serde_json::json!({
            "productVO": {
                "productCode": "C1",
                "stockNumber": "10",
                "validStockNumber": null,
                "productPriceList": [
                    { "startPurchasedNumber": "1", "productPrice": "0.25" }
                ],
                "fileTypeVOList": null
            },
            "paramLinkedMap": null
        }))
        .expect("nullable upstream fields should deserialize");
        assert_eq!(record.product.stock_number, Some(10));
        assert_eq!(record.product.price_list[0].start_purchased_number, 1);
        assert_eq!(record.product.price_list[0].product_price, 0.25);
        assert!(record.product.file_groups.is_empty());
        assert!(record.parameters.is_empty());

        let malformed_shapes: LcscPhoneProductRecord = serde_json::from_value(serde_json::json!({
            "productVO": {
                "productCode": "C2",
                "productPriceList": {},
                "fileTypeVOList": {}
            },
            "paramLinkedMap": []
        }))
        .expect("unexpected collection shapes should not reject the whole response");
        assert!(malformed_shapes.product.price_list.is_empty());
        assert!(malformed_shapes.product.file_groups.is_empty());
        assert!(malformed_shapes.parameters.is_empty());
    }

    #[test]
    fn selects_first_quantity_tier_only_for_available_products() {
        let record = LcscPhoneProductRecord {
            product: LcscPhoneProduct {
                product_code: Some("C1".to_owned()),
                product_name: None,
                product_grade_plate_name: None,
                encapsulation_model: None,
                product_model: None,
                breviary_image_url: None,
                big_image_url: None,
                lucene_breviary_image_urls: None,
                stock_number: Some(10),
                valid_stock_number: None,
                price_list: vec![
                    LcscPhonePriceTier {
                        start_purchased_number: 10,
                        product_price: 8.2,
                    },
                    LcscPhonePriceTier {
                        start_purchased_number: 1,
                        product_price: 9.91,
                    },
                ],
                file_groups: Vec::new(),
            },
            light_brand_name: None,
            light_product_intro: None,
            light_product_name: None,
            light_product_model: None,
            light_standard: None,
            parameters: HashMap::new(),
        };
        assert_eq!(reference_price(&record), Some(9.91));
    }

    #[test]
    fn accepts_only_controlled_image_urls() {
        let source = "https://alimg.szlcsc.com/upload/public/product/middle/20241118/example.jpg";
        assert_eq!(controlled_image_url(source).as_deref(), Some(source));
        let certificate =
            "https://alimg.szlcsc.com/upload/public/brand/product/certificate/20240701/example.jpg";
        assert_eq!(
            controlled_image_url(certificate).as_deref(),
            Some(certificate)
        );
        assert!(
            controlled_image_url("http://alimg.szlcsc.com/upload/public/product/a.jpg").is_none()
        );
        assert!(controlled_image_url("https://example.com/upload/public/product/a.jpg").is_none());
        assert!(controlled_image_url(
            "https://user@alimg.szlcsc.com/upload/public/product/middle/example.jpg"
        )
        .is_none());
        assert!(controlled_image_url("https://alimg.szlcsc.com/private/a.jpg").is_none());
    }

    #[test]
    fn selects_source_from_first_breviary_then_falls_back_without_downloading() {
        let mut product = image_test_product();
        product.lucene_breviary_image_urls = Some(
            "https://alimg.szlcsc.com/upload/public/product/breviary/20230105/first.jpg<$>https://alimg.szlcsc.com/upload/public/product/breviary/20230105/second.jpg"
                .to_owned(),
        );
        assert_eq!(
            preferred_image_url(&product).as_deref(),
            Some("https://alimg.szlcsc.com/upload/public/product/source/20230105/first.jpg")
        );

        product.lucene_breviary_image_urls = Some(
            "https://alimg.szlcsc.com/upload/public/brand/product/certificate/20240701/first.png"
                .to_owned(),
        );
        assert_eq!(
            preferred_image_url(&product).as_deref(),
            Some(
                "https://alimg.szlcsc.com/upload/public/brand/product/certificate/20240701/first.png"
            )
        );

        product.lucene_breviary_image_urls = Some("https://example.com/untrusted.jpg".to_owned());
        product.breviary_image_url = None;
        assert_eq!(
            preferred_image_url(&product).as_deref(),
            Some("https://alimg.szlcsc.com/upload/public/product/middle/20230105/fallback.jpg")
        );

        product.big_image_url = None;
        assert_eq!(preferred_image_url(&product), None);
    }

    fn image_test_product() -> LcscPhoneProduct {
        LcscPhoneProduct {
            product_code: Some("C1".to_owned()),
            product_name: None,
            product_grade_plate_name: None,
            encapsulation_model: None,
            product_model: None,
            breviary_image_url: Some(
                "https://alimg.szlcsc.com/upload/public/product/breviary/20230105/fallback.jpg"
                    .to_owned(),
            ),
            big_image_url: Some(
                "https://alimg.szlcsc.com/upload/public/product/middle/20230105/fallback.jpg"
                    .to_owned(),
            ),
            lucene_breviary_image_urls: None,
            stock_number: None,
            valid_stock_number: None,
            price_list: Vec::new(),
            file_groups: Vec::new(),
        }
    }

    #[test]
    fn accepts_only_controlled_datasheet_paths() {
        let groups = vec![LcscPhoneFileGroup {
            file_type: "pdf_property".to_owned(),
            details: vec![LcscPhoneFileDetail {
                file_url: Some("/upload/public/pdf/source/20241012/example.pdf".to_owned()),
            }],
        }];
        assert_eq!(
            datasheet_url(&groups).as_deref(),
            Some("https://atta.szlcsc.com/upload/public/pdf/source/20241012/example.pdf")
        );
    }
}
