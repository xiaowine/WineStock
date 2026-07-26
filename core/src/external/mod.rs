//! 外部资料服务适配入口。
//!
//! 本模块属于 core 基础设施边界，负责第三方 HTTP client 的构建与协议隔离。
//! 它不拥有 WineStock 物品创建、持久化或前端交互。

mod lcsc;

pub use lcsc::ExternalCatalogBootstrapError;
pub(crate) use lcsc::{LcscLookupClient, LcscLookupError, LcscProductImage, LcscProductRecord};

/// core 共享的外部资料查询运行时。
#[derive(Debug, Clone)]
pub(crate) struct ExternalCatalogRuntime {
    lcsc: LcscLookupClient,
}

impl ExternalCatalogRuntime {
    pub(crate) fn build() -> Result<Self, ExternalCatalogBootstrapError> {
        Ok(Self {
            lcsc: LcscLookupClient::build()?,
        })
    }

    pub(crate) fn lcsc(&self) -> &LcscLookupClient {
        &self.lcsc
    }

    #[cfg(test)]
    pub(crate) fn with_lcsc_endpoints(
        search_endpoint: String,
        price_endpoint: String,
    ) -> Result<Self, ExternalCatalogBootstrapError> {
        Ok(Self {
            lcsc: LcscLookupClient::build_for_test(search_endpoint, price_endpoint)?,
        })
    }
}
