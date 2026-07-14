//! 仓储层对处理函数暴露业务语义，避免业务代码直接散写 SeaORM 查询。
//!
//! 当前仓储命名直接对齐 `auth/users/rbac` 模块，避免继续保留 `identity` 中间层目录。

mod audit_repo;
mod auth_repo;
mod file_object;
mod rbac_repo;
mod refresh_token_repo;
mod stock_repo;
mod time;
mod user_repo;
mod validation;

#[allow(unused_imports)]
pub(crate) use audit_repo::{AuditRepository, RecordAuditEvent};
#[allow(unused_imports)]
pub(crate) use auth_repo::AuthRepository;
pub(crate) use file_object::{CreateFileObject, FileObjectRepository};
#[allow(unused_imports)]
pub(crate) use rbac_repo::{PermissionRecord, RbacRepository};
#[allow(unused_imports)]
pub(crate) use refresh_token_repo::{CreateRefreshToken, RefreshTokenRepository};
#[allow(unused_imports)]
pub(crate) use stock_repo::{
    AuditEventRecord, CatalogSort, CatalogStockFilter, CreateInboundOrder, CreateInboundOrderItem,
    CreateInboundTemplate, CreateItemAttributeTemplate, CreateItemCategory, CreateLocation,
    CreateLocationGroup, CreateLocationTransfer, CreateOutboundOrder, CreateOutboundOrderItem,
    CreateStockItem, DailyMovementTrendRecord, DashboardOverviewRecord, InboundAttributeInput,
    InboundOrderDetail, InboundOrderItemRecord, InboundOrderRecord, InboundTemplateDetail,
    ItemAttributeInput, ItemAttributeRecord, ItemAttributeTemplateDetail, ItemCatalogCriteria,
    ItemCatalogFieldFilter, ItemCatalogPage, ItemFilterValuesCriteria, ItemInventoryRecord,
    ItemOptionCriteria, ItemOptionRecord, ListAuditEvents, ListInboundOrders, ListOutboundOrders,
    OutboundOrderDetail, OutboundOrderItemRecord, OutboundOrderRecord, Page,
    SlowMovingStockItemRecord, StockFilterFieldRecord, StockFilterValueRecord,
    StockItemBatchRecord, StockItemListRecord, StockItemLocationRecord, StockLocationGroupRecord,
    StockLocationRecord, StockLocationTransferRecord, StockRepository, StockSubstituteInput,
    StockSubstituteRecord, TemplateFieldInput, UpdateInboundTemplate, UpdateItemAttributeTemplate,
    UpdateItemCategory, UpdateLocation, UpdateLocationGroup, UpdateStockItem,
};
pub(crate) use time::{sqlite_now, sqlite_time_after_seconds};
#[allow(unused_imports)]
pub(crate) use user_repo::{CreateUser, ListUsers, UserPage, UserRepository};

#[cfg(test)]
#[path = "../../tests/persistence_repository.rs"]
mod tests;
