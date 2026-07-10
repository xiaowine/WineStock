# Core Persistence 代码地图

`core/src/persistence` 拥有 SQLite/SeaORM 启动、migration、entity 和业务 repository，不拥有 HTTP DTO。

## 连接与 migration

- `core/src/persistence/mod.rs`：`StorageRuntime`、存储启动错误、路径校验和 migration 入口。
- `core/src/persistence/connection.rs`：SQLite 连接池、WAL、foreign keys、busy timeout 和 checkpoint 配置。
- `core/src/persistence/migration/mod.rs`：SeaORM `Migrator`。
- `core/src/persistence/migration/m20260706_000001_initial_schema.rs`
  - 创建 auth、storage、stock 和 audit 业务表。
  - 建立 refresh token、文件、签名密钥、SKU、模板、FIFO 批次和审计查询索引或约束。

数据库表与字段边界见 `docs/database-schema.md`。

## Entity

- `entity/auth_setting.rs`：鉴权设置。
- `entity/auth_signing_key.rs`：JWT signing key。
- `entity/refresh_token.rs`：refresh token 设备记录和吊销状态。
- `entity/user.rs`：用户、状态和强制改密标记。
- `entity/file_object.rs`：文件元数据。
- `entity/stock_item.rs`：库存物品基础资料。
- `entity/stock_template.rs`、`stock_template_field.rs`：库存模板和字段定义。

## Repository

- `repository/auth_repo.rs`：鉴权默认设置、active signing key 和首次管理员判断。
- `repository/audit_repo.rs`：跨业务审计事件写入，调用方必须传入脱敏详情。
- `repository/user_repo.rs`：用户创建、查询、分页、状态、密码哈希和强制改密标记。
- `repository/rbac_repo.rs`：权限定义、用户权限、整体替换和防锁死查询。
- `repository/refresh_token_repo.rs`：refresh token 创建、查询、吊销和事务轮换。
- `repository/file_object.rs`：文件元数据，不保存文件内容。
- `repository/time.rs`：SQLite UTC 时间辅助函数。
- `repository/validation.rs`：repository 写库输入校验。

## Stock Repository

- `repository/stock_repo.rs`：`StockRepository` 入口和稳定重新导出。
- `stock_repo/types.rs`：库存仓储输入和读取模型，不执行查询。
- `stock_repo/items.rs`：物品和库存快照。
- `stock_repo/templates.rs`：模板与字段。
- `stock_repo/locations.rs`：分组、库位和移库。
- `stock_repo/inbound.rs`：入库单与审批批次生成。
- `stock_repo/outbound.rs`：出库单、指定批次或 FIFO 扣减。
- `stock_repo/dashboard.rs`：看板聚合。
- `stock_repo/substitutes.rs`：替代料关系。
- `stock_repo/events.rs`：审计事件查询。
- `stock_repo/search.rs`：物品、入库、出库 JSON 标量搜索和筛选值聚合。
- `stock_repo/common.rs`：库存余额、审计写入和 JSON 编码共享逻辑。

字段约束来源见 `docs/validation/core-src-persistence-*.md`。
