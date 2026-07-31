# Core Persistence 代码地图

`core/src/persistence` 拥有 SQLite/SeaORM 启动、migration、entity 和业务 repository，不拥有 HTTP DTO。
数据库表与字段边界见 `core/docs/database-schema.md`；逐文件职责以源码中文文件头注释为准。

## 连接与 migration

- `mod.rs`、`connection.rs`：`StorageRuntime`、存储启动错误、路径校验，以及 SQLite 连接池、WAL、foreign keys、busy timeout 和 checkpoint 配置。
- `migration/`：SeaORM `Migrator`，当前只有一个初始 schema migration，一次性创建 auth、storage、stock 和 audit 全部业务表；`stock_item_attribute_definitions` 统一保存模板与物品私有属性定义（含 `catalog_visible` 和数字单位规则），库位以全局唯一名称和可选备注建模，不保留开发阶段增量兼容 migration。

## Entity（`entity/`）

- 鉴权域：鉴权设置、JWT signing key、refresh token 设备记录与吊销状态、用户（状态与强制改密标记）。
- 文件域：文件元数据，以及物品属性行与文件对象的一对一绑定；物品主图由 `stock_items.image_file_id` 直接引用。
- 库存域：物品、分类、实际属性和可选物品属性预设（字段级显式单位规则）。

## Repository（`repository/`）

- 鉴权与用户：auth 默认设置与 active signing key、用户创建/查询/分页/状态/密码哈希、权限定义与整体替换（含防锁死查询）、refresh token 创建/吊销/事务轮换。
- `audit_repo.rs`：跨业务审计事件写入，调用方必须传入脱敏详情。
- `file_object.rs`：文件元数据、绑定访问查询、临时删除和孤儿清理查询，不保存文件内容。
- `time.rs`、`validation.rs`：SQLite UTC 时间辅助与 repository 写库输入校验。

## Stock Repository（`repository/stock_repo/`）

- `stock_repo.rs`：`StockRepository` 入口和稳定重新导出；`types/` 按子域拆分仓储输入与读取模型，不执行查询。
- 子域查询模块与库存业务子域一一对应：物品（命令、目录实时库存聚合、参数化筛选与状态计数、批次分页）、分类与模板（使用数聚合、删除事务内影响数）、库位（分组、整批次移库）、入库（含状态条件分页，直接入库与审批复用同一事务步骤）、出库（状态筛选、指定批次或 FIFO 扣减）、看板聚合、替代料关系、审计事件查询。
- SQLite RETURNING 只用于能够等价返回单表读取模型并减少额外查询的路径：库位分组创建/更新与移库记录使用 raw `RETURNING`，签名密钥、刷新令牌、用户、文件元数据、物品分类、物品和属性模板主记录使用 Entity `exec_with_returning`；JOIN、聚合、明细加载或只需影响行数的写入继续保留原路径，选择依据与验证记录见 `core/docs/implementation-notes/sqlite-returning-remediation.md`。
- `search.rs`：物品/入库属性和出库历史追溯搜索，以及目录上下文分面筛选值聚合。
- `common.rs`：库存余额、审计写入和 JSON 编码共享逻辑。

字段约束来源见 `core/docs/validation/core-src-persistence-*.md`。
