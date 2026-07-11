# Core Persistence 代码地图

`core/src/persistence` 拥有 SQLite/SeaORM 启动、migration、entity 和业务 repository，不拥有 HTTP DTO。

## 连接与 migration

- `core/src/persistence/mod.rs`：`StorageRuntime`、存储启动错误、路径校验和 migration 入口。
- `core/src/persistence/connection.rs`：SQLite 连接池、WAL、foreign keys、busy timeout 和 checkpoint 配置。
- `core/src/persistence/migration/mod.rs`：SeaORM `Migrator`。
- `core/src/persistence/migration/m20260706_000001_initial_schema.rs`
  - 创建 auth、storage、stock 和 audit 业务表，包括物品/入库属性及两类图片绑定关系。
  - 初始 schema 直接采用分类、物品属性模板和入库模板分离设计，不保留旧统一模板表。

数据库表与字段边界见 `core/docs/database-schema.md`。

## Entity

- `entity/auth_setting.rs`：鉴权设置。
- `entity/auth_signing_key.rs`：JWT signing key。
- `entity/refresh_token.rs`：refresh token 设备记录和吊销状态。
- `entity/user.rs`：用户、状态和强制改密标记。
- `entity/file_object.rs`：文件元数据；物品主图通过 `stock_items.image_file_id` 直接引用。
- `entity/inbound_file_binding.rs`、`item_file_binding.rs`：文件对象与具体属性行的一对一绑定。
- `entity/stock_item.rs`、`stock_item_category.rs`、`item_attribute.rs`：物品基础资料、分类和实际属性。
- `entity/item_attribute_template*.rs`：可选物品属性预设。
- `entity/inbound_template*.rs`、`inbound_order_item_attribute.rs`：入库模板和实际入库属性。

## Repository

- `repository/auth_repo.rs`：鉴权默认设置、active signing key 和首次管理员判断。
- `repository/audit_repo.rs`：跨业务审计事件写入，调用方必须传入脱敏详情。
- `repository/user_repo.rs`：用户创建、查询、分页、状态、密码哈希和强制改密标记。
- `repository/rbac_repo.rs`：权限定义、用户权限、整体替换和防锁死查询。
- `repository/refresh_token_repo.rs`：refresh token 创建、查询、吊销和事务轮换。
- `repository/file_object.rs`：文件元数据、物品主图/属性/入库绑定访问查询、临时删除和孤儿清理查询，不保存文件内容。
- `repository/time.rs`：SQLite UTC 时间辅助函数。
- `repository/validation.rs`：repository 写库输入校验。

## Stock Repository

- `repository/stock_repo.rs`：`StockRepository` 入口和稳定重新导出。
- `stock_repo/types/`：按物品、库位、模板、入库、出库和分析审计子域拆分仓储输入与读取模型，不执行查询。
- `stock_repo/items.rs`：物品必选主图、任意属性、扩展文件绑定和库存快照。
- `stock_repo/categories.rs`：物品分类。
- `stock_repo/templates/`：按 `common`、`item`、`inbound` 拆分两类模板仓储。
- `stock_repo/locations.rs`：分组、库位和移库。
- `stock_repo/inbound.rs`：入库单、明细、逐行属性与图片引用事务绑定，并复用同一事务步骤处理直接入库和后续审批。
- `stock_repo/outbound.rs`：出库单、指定批次或 FIFO 扣减。
- `stock_repo/dashboard.rs`：看板聚合。
- `stock_repo/substitutes.rs`：替代料关系。
- `stock_repo/events.rs`：审计事件查询。
- `stock_repo/search.rs`：物品属性、入库属性和出库历史追溯搜索及筛选值聚合。
- `stock_repo/common.rs`：库存余额、审计写入和 JSON 编码共享逻辑。

字段约束来源见 `core/docs/validation/core-src-persistence-*.md`。
