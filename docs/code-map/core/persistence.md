# Core Persistence 代码地图

`core/src/persistence` 拥有 SQLite/SeaORM 启动、migration、entity 和业务 repository，不拥有 HTTP DTO。

## 连接与 migration

- `core/src/persistence/mod.rs`：`StorageRuntime`、存储启动错误、路径校验和 migration 入口。
- `core/src/persistence/connection.rs`：SQLite 连接池、WAL、foreign keys、busy timeout 和 checkpoint 配置。
- `core/src/persistence/migration/mod.rs`：SeaORM `Migrator`。
- `core/src/persistence/migration/m20260706_000001_initial_schema.rs`
  - 创建 auth、storage、stock 和 audit 业务表，包括物品/入库属性及两类图片绑定关系。
  - 初始 schema 使用 `stock_item_attribute_definitions` 统一保存模板与物品私有属性定义，值表只引用 `definition_id`；数字定义使用 `none`、`fixed`、`select` 单位规则，不保留开发阶段兼容迁移。
- `core/src/persistence/migration/m20260713_000002_item_catalog_visibility.rs`
  - 为已执行初始 schema 的数据库补充 `catalog_visible`，新数据库检测到字段已存在时跳过。
- `core/src/persistence/migration/m20260715_000003_location_name_notes.rs`
  - 将库位自然标识切换为全局唯一名称，增加可选备注并移除旧库位编码；已有重复名称会阻止迁移，避免静默合并业务位置。

数据库表与字段边界见 `core/docs/database-schema.md`。

## Entity

- `entity/auth_setting.rs`：鉴权设置。
- `entity/auth_signing_key.rs`：JWT signing key。
- `entity/refresh_token.rs`：refresh token 设备记录和吊销状态。
- `entity/user.rs`：用户、状态和强制改密标记。
- `entity/file_object.rs`：文件元数据；物品主图通过 `stock_items.image_file_id` 直接引用。
- `entity/inbound_file_binding.rs`、`item_file_binding.rs`：文件对象与具体属性行的一对一绑定。
- `entity/stock_item.rs`、`stock_item_category.rs`、`item_attribute.rs`：物品基础资料、分类和实际属性。
- `entity/item_attribute_template*.rs`：可选物品属性预设及字段级显式单位规则。
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
- `stock_repo/items.rs`：物品命令、编辑资料、轻量选择、目录实时库存聚合、单位/库位/模板属性参数化筛选、状态筛选计数、库位摘要和批次分页。
- `stock_repo/categories.rs`：物品分类、当前有效物品分类使用数聚合和分类删除影响数。
- `stock_repo/templates/`：按 `common`、`item`、`inbound` 拆分两类模板仓储；物品属性模板子模块聚合当前有效物品使用数并在删除事务内记录影响数。
- `stock_repo/locations.rs`：分组、名称唯一且可带备注的库位，以及整批次移库。
- `stock_repo/inbound.rs`：入库单、含状态条件的服务端分页、明细、逐行属性与图片引用事务绑定，并复用同一事务步骤处理直接入库和后续审批。
- `stock_repo/outbound.rs`：出库单、状态筛选、物品身份投影，以及指定批次或 FIFO 扣减。
- `stock_repo/dashboard.rs`：看板聚合。
- `stock_repo/substitutes.rs`：替代料关系。
- `stock_repo/events.rs`：审计事件查询。
- `stock_repo/search.rs`：物品属性、入库属性和出库历史追溯搜索，以及物品目录上下文分面筛选值和历史筛选值聚合。
- `stock_repo/common.rs`：库存余额、审计写入和 JSON 编码共享逻辑。

字段约束来源见 `core/docs/validation/core-src-persistence-*.md`。
