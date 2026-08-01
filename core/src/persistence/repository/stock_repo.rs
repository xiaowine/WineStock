//! stock repository 模块入口。
//!
//! 本模块属于 `core` 的持久化层，按库存子域拆分仓储实现并保留统一导出面。
//! handler 和 service 不应直接拼接 `stock_*` 表结构。

use sea_orm::{ConnectionTrait, DatabaseConnection};

mod categories;
mod common;
mod dashboard;
mod events;
mod inbound;
mod items;
mod locations;
mod outbound;
mod search;
mod substitutes;
mod templates;
mod types;

pub(crate) use search::{StockFilterFieldRecord, StockFilterValueRecord};
pub(crate) use types::{
    AuditEventRecord, CatalogAttributeRecord, CatalogSort, CatalogStockFilter, CreateInboundOrder,
    CreateInboundOrderItem, CreateItemAttributeTemplate, CreateItemCategory, CreateLocation,
    CreateLocationGroup, CreateLocationTransfer, CreateOutboundOrder, CreateOutboundOrderItem,
    CreateStockItem, DailyMovementTrendRecord, DashboardOverviewRecord, InboundOrderDetail,
    InboundOrderItemRecord, InboundOrderRecord, ItemAttributeInput, ItemAttributeRecord,
    ItemAttributeTemplateDetail, ItemCatalogCountsRecord, ItemCatalogCriteria,
    ItemCatalogFieldFilter, ItemCatalogPage, ItemCatalogRecord, ItemFilterValuesCriteria,
    ItemInventoryRecord, ItemOptionCriteria, ItemOptionLookupCriteria, ItemOptionRecord,
    ListAuditEvents, ListInboundOrders, ListOutboundOrders, OutboundOrderDetail,
    OutboundOrderItemRecord, OutboundOrderRecord, Page, SlowMovingStockItemRecord,
    StockItemBatchRecord, StockItemListRecord, StockItemLocationRecord, StockLocationGroupRecord,
    StockLocationRecord, StockLocationTransferRecord, StockSubstituteInput, StockSubstituteRecord,
    TemplateFieldInput, UpdateItemAttributeTemplate, UpdateItemCategory, UpdateLocation,
    UpdateLocationGroup, UpdateStockItem,
};

/// stock 仓储层封装库存领域持久化语义。
pub(crate) struct StockRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的 stock 仓储。
    pub(crate) fn new(database: &'db C) -> Self {
        Self { database }
    }

    /// 返回当前仓储绑定的连接，供同一业务服务组合其它 repository。
    pub(crate) fn database(&self) -> &'db C {
        self.database
    }
}
