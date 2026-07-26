//! 立创 EDA 商品资料接口适配器。
//!
//! 本模块只理解固定上游请求与原始响应，不公开 WineStock HTTP DTO，也不写数据库。

use std::{collections::HashMap, error::Error, fmt, sync::Arc, time::Duration};

use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Semaphore;
use url::Url;

const LCEDA_SEARCH_URL: &str = "https://pro.lceda.cn/api/devices/search";
const LCEDA_PRICE_URL: &str = "https://pro.lceda.cn/api/components/getSmtPartInfo";
const LCEDA_SEARCH_PATH: &str = "0819f05c4eef4c71ace90d822a990e87";
const LCEDA_IMAGE_HOST: &str = "alimg.szlcsc.com";
const LCEDA_IMAGE_MIDDLE_PREFIX: &str = "/upload/public/product/middle/";
const LCEDA_IMAGE_SOURCE_PREFIX: &str = "/upload/public/product/source/";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;
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
    /// 搜索结果声明的立创客编；模糊结果可能不包含该字段。
    pub(crate) product_code: Option<String>,
    /// 搜索结果顶层的商品参数描述。
    pub(crate) description: Option<String>,
    /// 搜索结果中的首张商品图片地址；只允许由受控下载方法读取。
    pub(crate) image_url: Option<String>,
    /// 上游器件属性；业务层只投影允许公开的字段。
    pub(crate) attributes: HashMap<String, Value>,
}

/// 已复核 MIME 和文件签名的立创商品图片。
pub(crate) struct LcscProductImage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mime_type: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct LcscLookupClient {
    client: Client,
    search_endpoint: String,
    price_endpoint: String,
    permits: Arc<Semaphore>,
}

impl LcscLookupClient {
    pub(crate) fn build() -> Result<Self, ExternalCatalogBootstrapError> {
        Self::build_inner(
            LCEDA_SEARCH_URL.to_owned(),
            LCEDA_PRICE_URL.to_owned(),
            true,
        )
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(
        search_endpoint: String,
        price_endpoint: String,
    ) -> Result<Self, ExternalCatalogBootstrapError> {
        Self::build_inner(search_endpoint, price_endpoint, false)
    }

    fn build_inner(
        search_endpoint: String,
        price_endpoint: String,
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
            price_endpoint,
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_LOOKUPS)),
        })
    }

    pub(crate) async fn lookup(
        &self,
        product_code: &str,
    ) -> Result<(Vec<LcscProductRecord>, Option<f64>), LcscLookupError> {
        let _permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| LcscLookupError::Busy)?;
        let search_request = LcedaSearchRequest {
            attributes: HashMap::new(),
            path: LCEDA_SEARCH_PATH,
            uid: LCEDA_SEARCH_PATH,
            page: 1,
            page_size: 50,
            tag: [],
            wd: product_code,
        };
        let price_request = LcedaPriceRequest {
            numbers: [product_code],
            path: LCEDA_SEARCH_PATH,
        };
        let search = self
            .client
            .post(&self.search_endpoint)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&search_request)
            .send();
        let price = self
            .client
            .post(&self.price_endpoint)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&price_request)
            .send();
        let (search_response, price_response) = tokio::join!(search, price);
        let records = read_search_response(search_response.map_err(map_reqwest_error)?).await?;
        let price = match price_response {
            Ok(response) => read_price_response(response, product_code)
                .await
                .ok()
                .flatten(),
            Err(_) => None,
        };
        Ok((records, price))
    }

    pub(crate) async fn download_image(
        &self,
        source: &str,
    ) -> Result<LcscProductImage, LcscLookupError> {
        let candidates = image_candidates(source)?;
        let mut last_error = LcscLookupError::InvalidResponse;
        for url in candidates {
            match self.download_image_url(url).await {
                Ok(image) => return Ok(image),
                Err(error) => last_error = error,
            }
        }

        Err(last_error)
    }

    /// 下载单个已通过地址白名单校验的图片候选，并复核响应内容。
    async fn download_image_url(&self, url: Url) -> Result<LcscProductImage, LcscLookupError> {
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
        {
            return Err(LcscLookupError::Failed);
        }
        let declared_mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim)
            .map(str::to_owned);
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_IMAGE_BYTES {
                return Err(LcscLookupError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        let mime_type = detected_image_mime(&bytes).ok_or(LcscLookupError::InvalidResponse)?;
        if declared_mime.as_deref() != Some(mime_type) {
            return Err(LcscLookupError::InvalidResponse);
        }
        Ok(LcscProductImage { bytes, mime_type })
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
    // 只声明 http/1.1：工作区 reqwest 未启用 http2 feature，若 ALPN 协商出 h2（如立创图片主机）
    // hyper 会直接 panic。
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config
}

fn image_candidates(source: &str) -> Result<Vec<Url>, LcscLookupError> {
    let original = Url::parse(source).map_err(|_| LcscLookupError::InvalidResponse)?;
    if original.scheme() != "https" || original.host_str() != Some(LCEDA_IMAGE_HOST) {
        return Err(LcscLookupError::InvalidResponse);
    }

    let Some(suffix) = original.path().strip_prefix(LCEDA_IMAGE_MIDDLE_PREFIX) else {
        return Ok(vec![original]);
    };
    if suffix.is_empty() {
        return Ok(vec![original]);
    }

    // 搜索接口返回中等尺寸图；同一受控路径下优先尝试 source 原图，失败后仍保留原地址兜底。
    let mut source_variant = original.clone();
    source_variant.set_path(&format!("{LCEDA_IMAGE_SOURCE_PREFIX}{suffix}"));
    Ok(vec![source_variant, original])
}

fn detected_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

async fn read_search_response(
    mut response: reqwest::Response,
) -> Result<Vec<LcscProductRecord>, LcscLookupError> {
    if !response.status().is_success() {
        return Err(LcscLookupError::Failed);
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(LcscLookupError::InvalidResponse);
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(LcscLookupError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }

    let body: LcedaSearchResponse =
        serde_json::from_slice(&bytes).map_err(|_| LcscLookupError::InvalidResponse)?;
    if !body.success || body.code != 0 {
        return Err(LcscLookupError::InvalidResponse);
    }

    Ok(body
        .result
        .lists
        .into_records()
        .map(|record| LcscProductRecord {
            product_code: record.product_code,
            description: record.description,
            image_url: record.images.into_iter().next(),
            attributes: record.attributes,
        })
        .collect())
}

async fn read_price_response(
    mut response: reqwest::Response,
    product_code: &str,
) -> Result<Option<f64>, LcscLookupError> {
    if !response.status().is_success() {
        return Err(LcscLookupError::Failed);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(LcscLookupError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    let body: LcedaPriceResponse =
        serde_json::from_slice(&bytes).map_err(|_| LcscLookupError::InvalidResponse)?;
    if !body.success || body.code != 0 {
        return Err(LcscLookupError::InvalidResponse);
    }
    Ok(reference_price(body.result, product_code))
}

fn reference_price(records: Vec<LcedaPriceRecord>, product_code: &str) -> Option<f64> {
    let mut matches = records.into_iter().filter(|record| {
        record
            .component_code
            .trim()
            .eq_ignore_ascii_case(product_code)
    });
    let Some(record) = matches.next() else {
        return None;
    };
    if matches.next().is_some() || record.on_sale != 1 || record.stock_num <= 0 {
        return None;
    }
    record
        .price_list
        .into_iter()
        .filter(|tier| {
            tier.start_number >= 1 && tier.product_price.is_finite() && tier.product_price > 0.0
        })
        .min_by_key(|tier| tier.start_number)
        .map(|tier| tier.product_price)
}

fn map_reqwest_error(error: reqwest::Error) -> LcscLookupError {
    if error.is_timeout() {
        LcscLookupError::Timeout
    } else {
        LcscLookupError::Failed
    }
}

#[derive(Debug, Serialize)]
struct LcedaSearchRequest<'a> {
    attributes: HashMap<&'static str, &'static str>,
    path: &'static str,
    uid: &'static str,
    page: u8,
    #[serde(rename = "pageSize")]
    page_size: u8,
    tag: [&'static str; 0],
    wd: &'a str,
}

#[derive(Debug, Serialize)]
struct LcedaPriceRequest<'a> {
    numbers: [&'a str; 1],
    path: &'static str,
}

#[derive(Debug, Deserialize)]
struct LcedaPriceResponse {
    success: bool,
    code: i64,
    #[serde(default)]
    result: Vec<LcedaPriceRecord>,
}

#[derive(Debug, Deserialize)]
struct LcedaPriceRecord {
    component_code: String,
    #[serde(rename = "onSale")]
    on_sale: i64,
    stock_num: i64,
    #[serde(default, rename = "priceList")]
    price_list: Vec<LcedaPriceTier>,
}

#[derive(Debug, Deserialize)]
struct LcedaPriceTier {
    #[serde(rename = "startNumber")]
    start_number: i64,
    #[serde(rename = "productPrice")]
    product_price: f64,
}

#[derive(Debug, Deserialize)]
struct LcedaSearchResponse {
    success: bool,
    code: i64,
    #[serde(default)]
    result: LcedaSearchResult,
}

#[derive(Debug, Default, Deserialize)]
struct LcedaSearchResult {
    #[serde(default)]
    lists: LcedaSearchLists,
}

#[derive(Debug, Default, Deserialize)]
struct LcedaSearchLists {
    #[serde(default)]
    lcsc: Vec<LcedaProductRecord>,
    #[serde(flatten)]
    other: HashMap<String, Vec<LcedaProductRecord>>,
}

impl LcedaSearchLists {
    fn into_records(self) -> impl Iterator<Item = LcedaProductRecord> {
        let mut other = self.other.into_iter().collect::<Vec<_>>();
        other.sort_by(|left, right| left.0.cmp(&right.0));
        self.lcsc
            .into_iter()
            .chain(other.into_iter().flat_map(|(_, records)| records))
    }
}

#[derive(Debug, Deserialize)]
struct LcedaProductRecord {
    #[serde(default)]
    product_code: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    images: Vec<String>,
    #[serde(default)]
    attributes: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price_record(code: &str, stock_num: i64, prices: &[(i64, f64)]) -> LcedaPriceRecord {
        LcedaPriceRecord {
            component_code: code.to_owned(),
            on_sale: 1,
            stock_num,
            price_list: prices
                .iter()
                .map(|(start_number, product_price)| LcedaPriceTier {
                    start_number: *start_number,
                    product_price: *product_price,
                })
                .collect(),
        }
    }

    #[test]
    fn selects_first_quantity_tier_only_for_available_products() {
        assert_eq!(
            reference_price(
                vec![price_record("C2983288", 10, &[(10, 8.2), (1, 9.91)])],
                "C2983288"
            ),
            Some(9.91)
        );
        assert_eq!(
            reference_price(vec![price_record("C2982", 0, &[(1, 7.17)])], "C2982"),
            None
        );
        assert_eq!(reference_price(Vec::new(), "C9900201662"), None);
    }

    #[test]
    fn recognizes_only_supported_image_signatures() {
        assert_eq!(
            detected_image_mime(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
            Some("image/png")
        );
        assert_eq!(detected_image_mime(&[0xff, 0xd8, 0xff]), Some("image/jpeg"));
        assert_eq!(detected_image_mime(b"not-an-image"), None);
    }

    #[test]
    fn prefers_source_image_and_keeps_middle_as_fallback() {
        let candidates = image_candidates(
            "https://alimg.szlcsc.com/upload/public/product/middle/20230105/C6EA1F0A304B456033FFA9E209D5B049.jpg",
        )
        .expect("controlled LCSC image should be accepted");

        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].as_str(),
            "https://alimg.szlcsc.com/upload/public/product/source/20230105/C6EA1F0A304B456033FFA9E209D5B049.jpg"
        );
        assert_eq!(
            candidates[1].as_str(),
            "https://alimg.szlcsc.com/upload/public/product/middle/20230105/C6EA1F0A304B456033FFA9E209D5B049.jpg"
        );
    }

    #[test]
    fn does_not_rewrite_unrecognized_image_paths() {
        let source = "https://alimg.szlcsc.com/upload/public/product/thumb/20230105/example.jpg";
        let candidates = image_candidates(source).expect("allowed host should remain downloadable");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].as_str(), source);

        assert!(matches!(
            image_candidates("https://example.com/upload/public/product/middle/example.jpg"),
            Err(LcscLookupError::InvalidResponse)
        ));
    }
}
