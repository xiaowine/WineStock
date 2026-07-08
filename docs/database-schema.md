# 数据库结构

本文档记录当前 SQLite schema 的业务表命名、职责和系统表边界。
业务实现以 `core/src/persistence/migration/` 中的 SeaORM migration 为准；本文档用于帮助阅读数据库文件时快速区分表的所有权。
用户直接权限和授权规则见 [`rbac-permission-model.md`](rbac-permission-model.md)。

## 命名规则

业务表使用领域前缀，避免和 SQLite、SeaORM 或平台权限概念混淆：

- `auth_`：账号、权限、令牌和鉴权内部状态。
- `storage_`：服务端可查询的存储元数据。
- `stock_`：库存模板、物品、出入库单据、批次、流水和替代料关系。
- `audit_`：跨业务操作审计事件。

不要把 `seaql_migrations`、`sqlite_master` 或 `sqlite_sequence` 当成 WineStock 业务表。

## 业务表

### `auth_users`

账号基础表。保存登录用户名、密码哈希、展示名、账号状态、强制改密标记和创建/更新时间。

重要字段：

- `username`：登录用户名，数据库内唯一。
- `password_hash`：密码哈希，不保存明文密码。
- `status`：账号状态，当前允许 `active` 或 `disabled`。
- `password_change_required`：是否必须先修改临时密码，SQLite 中使用 0/1 布尔值。

### `auth_permissions`

权限定义表。保存系统可识别的权限代码和说明。

重要字段：

- `code`：稳定权限代码，例如 `stock.read` 或 `user.read`。
- `description`：权限说明。

启动时会补齐内置权限定义，但不会覆盖已存在权限的说明：

- `user.register`：注册新用户。
- `user.read`：查看用户列表和用户详情。
- `user.status.update`：启用或停用用户账号。
- `user.permissions.update`：整体替换用户权限。
- `user.permission.read`：查看权限定义。
- `user.password.reset`：直接重置用户密码。
- `stock.read`：查看库存数据。
- `stock.write`：创建或修改库存数据。
- `stock.item.manage`：创建、修改和软删除库存物品。
- `stock.template.manage`：管理库存模板和模板字段。
- `stock.inbound.create`：创建入库单。
- `stock.inbound.approve`：审批或拒绝入库单。
- `stock.outbound.create`：创建出库单。
- `stock.outbound.approve`：审批或拒绝出库单。
- `stock.substitute.manage`：绑定或解绑替代料关系。
- `audit.read`：查询审计事件日志。

### `auth_user_permission_assignments`

用户与权限的直接分配表。它不是权限定义，而是记录“哪个用户拥有哪些权限”。

主键：

- `(user_id, permission_id)`：同一用户不能重复分配同一权限。

首个注册用户会在注册事务中获得全部内置权限。启动初始化只补齐 `auth_permissions`，不会写入本表。

### `auth_settings`

数据库托管的鉴权策略表。JSON 启动配置不保存 token TTL 等安全相关运行时策略。
refresh token 轮换是固定安全策略，不作为配置项关闭。

当前默认设置：

- `access_token_ttl_seconds`
- `refresh_token_ttl_seconds`

### `auth_signing_keys`

JWT access token 签名密钥表。保存系统生成的签名密钥材料和生命周期状态。

重要字段：

- `key_id`：JWT header 中的 `kid`。
- `algorithm`：签名算法，当前默认 `HS256`。
- `key_material`：签名密钥材料，不能写入日志或普通 API 响应。
- `status`：当前允许 `active` 或 `retired`。

约束：

- SQLite 局部唯一索引保证同一时间最多一条 `active` 密钥。

### `auth_refresh_tokens`

刷新令牌表。只保存 refresh token 哈希和设备元数据，不保存明文令牌。
同一用户可以有多条未吊销 refresh token，用于支持多设备同时登录；轮换只影响提交的那一条 token。

重要字段：

- `user_id`：所属账号。
- `token_hash`：刷新令牌哈希，数据库内唯一。
- `device_name`：登录请求提供的设备名称，不能为空。
- `client_kind`：登录客户端类型，只允许 `desktop` 或 `android`。
- `app_version`：登录客户端 App 版本号，不能为空。
- `refresh_token_version`：refresh token 格式版本，由服务端生成规则决定。
- `expires_at`：过期时间。
- `revoked_at`：吊销时间，为空表示当前未吊销。
- `replaced_by_token_id`：轮换后替代该令牌的新令牌 ID；普通登出吊销时为空。

### `storage_file_objects`

文件元数据表。SQLite 只保存可查询、可校验的文件元数据；文件二进制内容由 `files/` 目录保存。

重要字段：

- `sha256`：文件内容摘要。
- `storage_path`：文件在 `files/` 目录下的相对路径。
- `owner_user_id`：文件所有者账号；用户删除后允许置空。

### `stock_templates`

库存模板表。模板定义物品分类所需的扩展字段，入库审批阶段会按物品关联模板校验扩展属性。
本地服务启动补齐 `元器件`、`3D打印耗材` 和 `通用` 三个内置模板；同名记录存在时跳过，因此不会覆盖用户修改或恢复用户软删除的模板。

重要字段：

- `name`：模板名称；未软删除记录内唯一。
- `deleted_at`：软删除时间；为空表示当前有效。

### `stock_template_fields`

库存模板字段表。每条记录描述一个模板字段的名称、类型、必填性、可搜索性、候选值和默认值。

重要字段：

- `field_type`：只允许 `text`、`number`、`select`、`date`、`file`、`url` 或 `boolean`。
- `options_json`：`select` 等字段类型使用的候选值 JSON。
- `sort_order`：模板详情展示和校验时的稳定排序依据。

### `stock_items`

库存物品基础资料表。物品是出入库、批次和替代料关系的最小业务对象。

重要字段：

- `sku`：物品编号；未软删除记录内唯一。
- `category_id`：关联模板 ID；模板删除后允许置空。
- `default_price`：参考单价，不允许为负。
- `reorder_point`：再订货点，不允许为负。
- `deleted_at`：软删除时间；为空表示当前有效。

### `stock_inbound_orders`

入库单主表。创建单据只写入 `pending` 状态，不增加库存；审批通过后才生成批次和库存流水。

重要字段：

- `status`：只允许 `pending`、`approved` 或 `rejected`。
- `approved_at`：已审批单据必须有审批时间。
- `rejected_at`：已拒绝单据必须有拒绝时间。

### `stock_inbound_order_items`

入库单明细表。记录入库物品、数量、单价、库位、外部批次号、有效期和模板扩展属性。

重要字段：

- `quantity`：入库数量，必须大于 0。
- `unit_price`：入库单价，不允许为负。
- `ext_attributes_json`：按物品模板校验后的扩展属性 JSON。

### `stock_batches`

库存批次表。入库审批通过后生成或增加批次库存；出库审批通过后扣减 `remaining_quantity`。

重要字段：

- `initial_quantity`：批次初始数量，必须大于 0。
- `remaining_quantity`：批次剩余数量，不允许为负且不能超过初始数量。
- `unit_cost`：批次成本，不允许为负。
- `expires_at`、`received_at` 和 `id`：未指定批次出库时的 FIFO 排序依据。

### `stock_outbound_orders`

出库单主表。创建单据只写入 `pending` 状态，不扣减库存；审批通过后才按指定批次或 FIFO 扣减库存。

重要字段：

- `status`：只允许 `pending`、`approved` 或 `rejected`。
- `approved_at`：已审批单据必须有审批时间。
- `rejected_at`：已拒绝单据必须有拒绝时间。

### `stock_outbound_order_items`

出库单明细表。记录出库物品、数量、可选指定批次和可选库位。

重要字段：

- `quantity`：出库数量，必须大于 0。
- `batch_id`：指定扣减批次；为空时后续审批逻辑按 FIFO 扣减。

### `stock_movements`

库存流水表。审批、调整等改变库存余额的动作写入流水，用于看板统计和追溯。

重要字段：

- `movement_type`：只允许 `inbound`、`outbound` 或 `adjustment`。
- `quantity_delta`：库存变化量，不能为 0。
- `balance_after`：变动后库存余额，不允许为负。

### `stock_substitutes`

替代料关系表。记录某个物品可由哪些物品替代。

重要字段：

- `(item_id, substitute_item_id)`：同一替代关系不能重复。
- `priority`：替代优先级，必须大于 0。
- 数据库约束禁止自引用；循环关系由后续业务服务校验。

### `audit_events`

审计事件表。记录创建、更新、删除、审批、拒绝、绑定和解绑等业务操作。
用户管理接口会把账号启停、权限变更、当前用户修改自己密码和管理员设置临时密码写入该表；密码明文、token 和密码哈希不得进入 `details_json`。

重要字段：

- `entity_type`：被操作实体类型，例如 `item`、`inbound`、`outbound`、`template`、`user` 或 `substitute`。
- `entity_id`：被操作实体 ID。
- `action`：只允许 `created`、`updated`、`deleted`、`approved`、`rejected`、`linked` 或 `unlinked`。
- `details_json`：变更摘要 JSON，不得写入 JWT、密码、refresh token、签名密钥等敏感值。

## 系统表

### `seaql_migrations`

SeaORM migration 版本记录表。用于记录哪些 migration 已经执行过，防止重复迁移。

### `sqlite_master`

SQLite 系统结构表。记录数据库中的表、索引、视图和触发器等对象。

### `sqlite_sequence`

SQLite 自增序列表。存在 `AUTOINCREMENT` 主键时，SQLite 用它记录自增值。

## 权限链路

用户权限通过直接分配表计算。业务授权应判断权限代码：

```text
auth_users
  -> auth_user_permission_assignments
  -> auth_permissions
```

repository 对外只暴露业务语义，例如按用户查询权限代码；handler 不直接依赖这些表名。
完整权限初始化和新增权限流程见 [`rbac-permission-model.md`](rbac-permission-model.md)。
