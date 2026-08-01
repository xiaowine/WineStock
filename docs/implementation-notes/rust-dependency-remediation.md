# Rust 依赖升级与 Feature 整改方案

> 文档状态：兼容整改及 SQLite RETURNING 两批选择性采用已实施<br>
> 编制日期：2026-07-29<br>
> 所属范围：Rust 工作区、`core` 持久化、`server` 生命周期与 `android/native` 平台适配<br>
> 审计输入：[`rust-dependency-feature-audit.md`](rust-dependency-feature-audit.md)

## 1. 目标

在不改变数据库 schema、HTTP 契约、业务权限语义和平台生命周期的前提下，同时完成：

- SeaORM / SeaORM Migration `1.1.20 -> 2.0.0` 与 SQLx `0.8.6 -> 0.9.0` 的破坏性 API 迁移；
- Base64 `0.22 -> 0.23`、JSON Web Token `10.4 -> 11.0` 和相应锁文件更新；
- Tokio `signal` feature 从工作区公共配置收窄到 `server`；
- 直接 Rustls 依赖关闭默认 feature，仅保留项目需要且由 Reqwest 依赖图补齐的能力；
- 对 SeaORM Migration、Reqwest、Axum、JWT 算法和重复版本的 feature 审计结论落地。

整改后必须恢复以下运行路径：

- `server -> core` 的无头服务启动与 SQLite 持久化；
- `android/native -> core` 的 Application 级本地服务启动与 SQLite 持久化；
- 现有 migration、bootstrap、repository、事务和测试夹具；
- 现有数据库文件的原地打开、迁移和业务读写。

本次升级是依赖兼容与 feature 收敛，不是数据库模型、鉴权协议或 HTTP 路由重构。不得借机改变表结构、API DTO、权限代码或业务行为。

## 2. 当前状态与风险

整改前工作区 manifest 已包含以下目标版本和 Release profile 变更，但源码尚未完成兼容迁移：

```toml
sea-orm = { version = "2.0.0", default-features = false, features = ["macros", "runtime-tokio", "sqlx-sqlite"] }
sea-orm-migration = { version = "2.0.0", default-features = false, features = ["runtime-tokio", "sqlx-sqlite"] }
sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "sqlite"] }
base64 = "0.23.0"
jsonwebtoken = { version = "11.0.0", default-features = false, features = ["rust_crypto"] }
```

截至本文编制时，`core/src` 中有 35 个 Rust 文件、150 处 `Statement::from_*` 调用。项目以手写 SQL 为主，升级的主要风险不是 entity derive，而是 SeaORM 2.0 对 `ConnectionTrait` 原始 SQL 接口的拆分。

主要风险如下：

| 风险 | 当前证据 | 影响 |
|---|---|---|
| 原始 SQL API 变更 | repository、bootstrap 和测试大量向 `execute/query_one/query_all` 传入 `Statement` | 编译失败；错误替换可能改变查询路径 |
| SQLx 主版本不一致 | `SqlxSqliteConnector::from_sqlx_sqlite_pool` 直接接收项目创建的 pool | SeaORM 与项目 SQLx 类型不兼容 |
| 返回值与验证边界变化 | `delete_by_id`、`UpdateOne`、`DeleteOne` 的验证和 returning 类型改变 | 编译通过后仍可能出现行为偏差 |
| SQLite RETURNING 被传递启用 | SeaORM Migration 重新开启 `sqlite-use-returning-for-3_35` | 改变写入/返回能力并增加 SQLite 3.35 运行时门槛 |
| 表级 RBAC 与现有模型冲突 | `RestrictedConnection` 禁止 raw SQL；WineStock 使用业务权限代码 | 无法直接包装现有 repository，也不能替代业务授权 |
| 工具链下限提高 | SeaORM 2.0 async crate 要求 Rust 1.94 | 主机、CI 与 Android Rust 工具链可能不一致 |
| feature 并集扩大 | `sea-orm-migration 2.0.0` 内部依赖重新开启 SeaORM defaults | Arrow、时间、UUID、Decimal、schema sync 和 SQLite RETURNING 进入构建图 |
| Tokio feature 作用域过宽 | 只有 `server` 使用 `tokio::signal::ctrl_c()` | Android/native 被工作区公共 feature 被动开启 signal |
| Rustls defaults 过宽 | 直接依赖开启 logging 与 post-quantum 偏好 | 增加无明确业务需求的 TLS 编译能力 |
| 鉴权与编码依赖升级 | Base64 0.23、JWT 11 同时变化 | token 编解码和错误映射可能回归 |

### 2.1 实施结果（2026-07-29）

- 35 个文件中的 150 处原始 `Statement` 调用已迁移到 `execute_raw`、`query_one_raw` 或 `query_all_raw`；项目没有需要迁移的流式调用。查询存在性和事务提交所需的 SeaORM 2.0 trait 已同步调整。
- 工作区只解析 SeaORM / SeaORM Migration `2.0.0` 与 SQLx `0.9.0`，不存在 SeaORM 1.1 或 SQLx 0.8 残留路径。
- Tokio `signal` 已从工作区公共 feature 移到 `server`；直接 Rustls 依赖已关闭 defaults，仅声明 `std`。最终依赖图仍由 Reqwest 提供 `aws-lc-rs` provider 和 TLS 1.2，且不再包含 Rustls `logging`、`prefer-post-quantum`。
- SeaORM Migration 仍会传递开启 SeaORM defaults，包括 `sqlite-use-returning-for-3_35`、schema sync 和多种类型 feature。工作区现已直接声明 SQLite RETURNING feature，使其不再只依赖传递来源；仍不启用 schema sync 或表级 RBAC。
- 连接测试执行 `SELECT sqlite_version()` 并强制实际链接的 bundled SQLite 不低于 3.35；主机测试与 Android ARM64 交叉构建均已通过。
- Base64 0.23、JSON Web Token 11、SQLx 0.9 的现有代码无需额外兼容层；鉴权、刷新、登出、文件和库存等回归测试均通过。

阶段 F 已分两批选择性迁移十条能够消除额外查询的创建/更新路径，具体边界和验证见
[`../../core/docs/implementation-notes/sqlite-returning-remediation.md`](../../core/docs/implementation-notes/sqlite-returning-remediation.md)。

## 3. 整改决策

### 3.1 保留现有持久化架构

- 保留 `core/src/persistence/entity` 中现有 `DeriveEntityModel` 格式。
- 保留手写 migration 和现有数据库 schema。
- 保留 repository 对 `ConnectionTrait` 的泛型边界和当前事务所有权。
- 不切换到 SeaORM 2.0 的 entity-first、schema sync 或嵌套 ActiveModel。
- 不为了使用新 API 把业务 SQL 机械重写为 Entity 查询。

这些新能力可在独立业务需求中评估，不能与依赖兼容整改绑定。

### 3.2 原始 SQL 使用 `*_raw`

SeaORM 2.0 将 SeaQuery statement 与原始 `Statement` 分成两套入口：

| 现有调用 | SeaORM 2.0 调用 |
|---|---|
| `connection.execute(statement)` | `connection.execute_raw(statement)` |
| `connection.query_one(statement)` | `connection.query_one_raw(statement)` |
| `connection.query_all(statement)` | `connection.query_all_raw(statement)` |
| `connection.stream(statement)` | `connection.stream_raw(statement)` |

整改约束：

- 只有参数类型为 SeaORM `Statement` 的调用改为 `*_raw`。
- SeaQuery `StatementBuilder` 继续使用 `execute/query_one/query_all/stream`，并以引用传入。
- 不执行无上下文的全仓库字符串替换；SQLx executor、测试 helper 和其它同名方法不得误改。
- 每个 repository 改造后核对参数顺序、占位符、`Value` 类型、事务连接和结果提取逻辑。
- 生产代码与测试夹具同步整改，不能只让 library target 编译通过。

示例：

```rust
let row = connection
    .query_one_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT id FROM auth_users WHERE username = ?",
        [username.into()],
    ))
    .await?;
```

### 3.3 对齐 SQLx 0.9

- 工作区 `sqlx`、SeaORM 和 SeaORM Migration 必须处于同一兼容依赖线。
- 保留 `SqlitePoolOptions`、WAL/PRAGMA、busy timeout 和连接池策略的现有所有权。
- 核对 `SqlxSqliteConnector::from_sqlx_sqlite_pool` 的输入 pool 与 SeaORM 2.0 使用同一个 SQLx 0.9 类型。
- 审核 SQLx 0.9 对 SQLite connect options、错误类型和 executor trait 的编译影响。
- 不在本任务中改用 SeaORM `Database::connect()`；当前显式构建 pool 的路径承载 SQLite 配置约束。

### 3.4 接受、显式拥有并选择性采用 SQLite RETURNING

直接 `sea-orm` 声明虽然使用 `default-features = false`，但实际依赖树显示
`sea-orm-migration 2.0.0` 会重新开启 SeaORM defaults，其中包含
`sqlite-use-returning-for-3_35`。Cargo resolver v2 对同一构建图中的普通依赖采用 feature 并集，因此不能把
“直接声明关闭 defaults”误写成“最终 feature 已关闭”。

SeaORM 2.0 兼容整改的初始决策（后续已由阶段 F 的选择性采用继续推进）：

- 保留 SeaORM Migration，接受 `sqlite-use-returning-for-3_35` 被传递启用。
- 在主机、server 和 Android APK 中记录 `SELECT sqlite_version()`，确认实际 SQLite 不低于 3.35。
- 覆盖 insert/update/delete、数据库默认值、事务回滚和已有数据库启动。
- 初始兼容阶段不主动改写现有 raw SQL，也不新增 `exec_with_returning*` 调用；先完成依赖升级与兼容验证。
- 如果任一交付环境低于 SQLite 3.35，必须在发布前回退 SeaORM 版本或收窄 migration 依赖，不能绕过连接检查。

中期再评估是否移除运行时 `sea-orm-migration`：只有依赖树和 Release 产物测量证明收益明确，且项目拥有的窄迁移执行层能够保持 migration 历史、事务和错误语义时，才允许单独实施。不得在 SeaORM 2.0 API 迁移中同时重写迁移架构。

后续阶段 F 的采用原则：

1. 在直接 `sea-orm` feature 中显式加入 `sqlite-use-returning-for-3_35`，把它从上游传递结果变成项目拥有的支持承诺；即使以后移除 SeaORM Migration，该能力仍保持稳定。
2. 先选择确实需要“写入并立即取回数据库结果”的路径，不为了统一风格重写全部 raw SQL。
3. 优先评估依赖数据库生成主键、默认值或更新后模型的 insert/update；delete 只有调用方确实需要被删除记录时才使用 RETURNING。
4. 分别覆盖空集合、单行、多行、冲突、事务回滚、数据库默认值和触发器；不得假定 RETURNING 一定反映后续触发器再次修改的值。
5. 对比原有“写入 + 再查询”与 RETURNING 的结果、错误、锁持有时间和调用次数，只有行为一致且收益可量化时才迁移生产路径。
6. 主机、server、Android bundled SQLite 和已有数据库升级路径全部通过后，再把 RETURNING 记录为正式持久化能力。

阶段 F 两批整改现已完成第 1、2、3、4、5 项：工作区显式声明 feature，三条手写 SQL 使用明确的 raw `RETURNING`，七条 Entity 创建使用 `exec_with_returning`；无收益或返回模型依赖 JOIN/聚合的路径保持原实现。Android 构建与 APK 包级校验已通过，真实设备 smoke 仍是发布前验收项。

### 3.5 不接入 SeaORM 内置 RBAC

本轮不使用 `RestrictedConnection` 或 SeaORM 表级 RBAC。

原因：

- `RestrictedConnection` 明确拒绝 `execute_raw/query_one_raw/query_all_raw/stream_raw` 和 unprepared SQL，与现有 repository 不兼容。
- SeaORM RBAC 判断的是 schema/table 上的 CRUD 操作；WineStock 权限是 `stock.item.read`、`user.permissions.update` 等业务动作。
- “最后一个权限管理员”“禁止修改自己的受保护权限”等业务不变量无法由表级 CRUD 推导。
- 同时维护两套权限来源会产生授权漂移和排障成本。

现有 `core` 业务授权仍是唯一权威。未来若出现不可信插件、用户自定义查询或多租户数据库访问边界，应另立安全设计任务；即使届时引入表级 RBAC，也只能作为纵深防御，不能替代 service 层权限检查。

### 3.6 合并 Feature 审计结论

本次整改按以下分组处理审计项：

| 项目 | 本轮决定 | 理由 |
|---|---|---|
| Tokio `signal` | 实施 | 从工作区根移除，仅在 `server` 成员增加 |
| Rustls defaults | 实施并做 TLS 真机验证 | 直接依赖只保留 `std`，provider/TLS 1.2 由实际统一依赖图确认 |
| Reqwest `json` | 保留 | LCSC 请求使用 `.json(&request)` |
| Reqwest platform verifier | 保留传递依赖 | 当前公开 feature 无法单独剔除，不为裁剪更换 HTTP client |
| Axum defaults | 保留 | `utoipa-axum` 会重新开启，单改直接声明没有实际收益 |
| JWT `rust_crypto` | 保留 | 上游不按 HS256 单独拆 feature，不在依赖整改中更换鉴权实现 |
| SeaORM Migration defaults | 短期接受、测量后再决策 | 直接移除会改变数据库启动和 migration 所有权 |
| Base64 0.22/0.23 重复 | 升级直接依赖并核对依赖树 | 只在 API 与 token/文件编码测试通过后保留 0.23 |
| SHA2 0.10/0.11 重复 | 暂不强制统一 | 由 JWT/加密传递依赖决定，收益低于兼容风险 |

Tokio 目标配置：

```toml
# workspace
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "sync", "time"] }

# server/Cargo.toml
tokio = { workspace = true, features = ["signal"] }
```

Rustls 目标配置：

```toml
rustls = { version = "0.23", default-features = false, features = ["std"] }
```

修改后必须从实际 `cargo tree -e features` 确认加密 provider、TLS 1.2 和项目需要的能力仍存在，不能只根据直接依赖声明推断最终 feature。

## 4. 破坏性 API 审核清单

除 raw SQL 入口外，还需逐项检查：

### 4.1 `DatabaseConnection`

SeaORM 2.0 将 `DatabaseConnection` 从枚举改为结构体，底层枚举为 `DatabaseConnectionType`。

- 搜索并移除对 `DatabaseConnection::Disconnected` 等旧变体的匹配。
- 普通借用、克隆和作为 `ConnectionTrait` 传递的路径保持不变。
- 不访问 `inner` 实现细节，除非现有功能确实需要区分 driver。

### 4.2 update/delete 验证

- 审核直接构建并调用 `.build()` 的 `UpdateOne`、`DeleteOne`；必要时先 `.validate()?`。
- 审核 `delete_by_id().exec_with_returning()`，其返回语义已从多行集合收紧为单行 `Option<Model>`。
- 确认主键未设置时按 `DbErr::PrimaryKeyNotSet` 处理，不依赖旧 panic。
- 确认不支持 RETURNING 时按 `DbErr::BackendNotSupported` 处理，不依赖旧 panic。

### 4.3 derive 与 trait 冲突

- `DerivePartialModel` 已默认实现 `FromQueryResult`，不得重复 derive。
- `DeriveValueType`、`DeriveActiveEnum` 会自动实现更多转换 trait，删除冲突的手写实现。
- 自定义 `EntityName`/`IdenStatic` 返回值必须满足 `&'static str`。
- 已移除的 `DeriveCustomColumn/default_as_str` 改为 `DeriveColumn` 与 `#[sea_orm(column_name = "...")]`。
- SeaQuery 表达式方法缺失时先核对是否需要导入 `ExprTrait`，不通过自定义扩展方法绕过。

### 4.4 migration

- 核对 `MigratorTrait` 在 2.0 中的 self/shim 签名兼容性。
- 保持 migration 顺序、名称和已发布 migration 内容不变。
- 只修复 trait/API 编译问题，不重写既有 migration SQL。
- 使用已有数据库副本验证重复启动不会重跑或篡改已应用 migration。

## 5. 实施顺序

### 阶段 A：依赖与工具链基线

1. 固定 SeaORM、SeaORM Migration、SQLx、Base64、JSON Web Token 版本及 feature。
2. 使用 `cargo metadata --locked` 确认锁文件与 manifest 一致。
3. 确认主机、CI、Android `cargo-ndk` 使用 Rust 1.94 或更高版本。
4. 保存升级前后的 `cargo tree --workspace -e features` 与重复版本对照。
5. 记录 SeaORM Migration 实际传递启用的 feature，不把上游强制能力误判为项目可直接关闭。

### 阶段 B：低风险 Feature 收窄

1. 从工作区 Tokio feature 中移除 `signal`，仅在 `server` 增加。
2. 关闭直接 Rustls defaults，仅声明 `std`。
3. 核对最终依赖图仍包含唯一可用的 Rustls provider 和 TLS 1.2。
4. 单独验证 server Ctrl+C、server HTTPS 和 Android 首次 HTTPS 握手。

### 阶段 C：SeaORM raw API 机械整改

1. 先改 `core/src/persistence/connection.rs` 和 repository 生产代码。
2. 再改 bootstrap、service 中的直接 SQL。
3. 最后改 `core/src/tests` 夹具和断言辅助代码。
4. 每批修改后运行格式检查和目标 crate check，缩小错误定位范围。

### 阶段 D：非机械依赖变化

1. 处理 `DatabaseConnection`、update/delete 和 derive 编译错误。
2. 处理 SQLx 0.9 pool、options、error 和 executor 差异。
3. 处理 migration trait 差异。
4. 处理 Base64 0.23 和 JSON Web Token 11 的 API、错误类型与 token 编解码差异。
5. 审核所有为了“先编译”添加的 clone、unwrap、类型转换或宽泛 error mapping，禁止掩盖行为变化。

### 阶段 E：行为与产物回归

1. 新建空数据库并执行完整 migration/bootstrap。
2. 使用 1.1.20 生成的已有数据库启动 2.0.0 代码。
3. 覆盖鉴权、用户权限、文件、库存、入库、出库、库位、替代品和审计事件。
4. 覆盖事务成功、业务拒绝、SQL 错误和 rollback 后的数据不变量。
5. 覆盖 server 与 Android 本地 core 的启动、停止和再次启动。
6. 覆盖登录、刷新、登出、过期/错误 token 和 Android 本地免登录交换。
7. 覆盖 LCSC HTTPS 成功、证书失败和连接错误路径。
8. 对比依赖树、server Release 与 Android Release native/APK 组成，记录 feature 收窄的实际收益。

### 阶段 F：显式并选择性采用 SQLite RETURNING（两批已完成）

该阶段在兼容整改之后独立实施：

1. 显式声明 `sqlite-use-returning-for-3_35`。
2. 建立符合条件的写入路径清单和旧行为基线。
3. 逐条迁移到 SeaORM returning API 或明确的 raw `RETURNING` SQL。
4. 为返回模型、默认值、触发器和事务失败补充测试。
5. 完成 server 与 Android 真机数据库回归并记录实测收益。

两批已完成 feature 显式声明、候选清单、三条 raw `RETURNING` SQL、七条 Entity `exec_with_returning`、返回/冲突/回滚测试以及 server/Android 构建验证。没有实际收益的写入继续使用原有接口；migration 执行层未改动。

## 6. 验证门槛

### 静态与依赖检查

```text
cargo +stable fmt --all -- --check
cargo +stable metadata --locked --no-deps
cargo +stable tree --workspace -e features --locked --offline
cargo +stable tree --workspace -d --locked --offline
cargo +stable tree -p winestock-core -d
```

必须确认：

- SeaORM 只解析到 2.0.x；
- SQLx 只保留必要的 0.9.x 路径，不因直接/传递依赖并存产生 pool 类型分裂；
- `sqlite-use-returning-for-3_35` 的来源明确记录为 SeaORM Migration 传递 feature；
- 未使用 SeaORM RBAC/`RestrictedConnection`；
- Tokio `signal` 只由 server 项目直接请求；
- Rustls provider、TLS 1.2 和 Reqwest JSON 仍在实际构建图中；
- 未意外启用 MySQL、PostgreSQL 等无业务用途的 database backend。

### Core 与工作区

```text
cargo +stable check -p winestock-core --all-targets --locked
cargo +stable test -p winestock-core --locked
cargo +stable check --workspace --all-targets --locked
cargo +stable test --workspace --locked
```

该任务修改公共持久化依赖并影响 server、Android native 与测试代码，因此完成前必须扩大到 workspace，而不能只验证 core library target。

### 平台交付

```text
cargo ndk -t arm64-v8a -P 28 check -p winestock-android --locked
cd android
gradlew.bat :app:testDebugUnitTest :app:assembleDebug :app:assembleRelease --no-daemon
```

Android 验证至少覆盖：

- APK 内只有预期 ARM64 native library；
- 已有数据库可打开且 migration/bootstrap 幂等；
- 本地 core 能启动、响应 `/api/health` 并正常关闭；
- 冷启动、force-stop 后重启和本地/远端模式切换不破坏数据库。
- LCSC HTTPS 请求与证书失败路径正常；
- `SELECT sqlite_version()` 不低于 3.35。

## 7. 验收标准

- 所有 `Statement` 均通过正确的 `*_raw` 入口执行，SeaQuery statement 未被误改。
- 工作区不存在 SeaORM 1.1 或 SQLx 0.8 的残留依赖路径。
- 新数据库和升级前数据库均能启动并通过完整 core 测试。
- 数据库 schema、migration 历史、HTTP/OpenAPI 契约和业务权限代码没有变化。
- 主机、server 与 Android 交付环境的 SQLite 均不低于 3.35。
- SeaORM Migration 传递 feature 已被准确记录，没有虚报不可实现的裁剪收益。
- 未创建或使用 `RestrictedConnection`。
- Tokio `signal` 已收窄到 server；Android native 不再由项目直接声明该 feature。
- Rustls 收窄后 HTTPS、证书校验、TLS 1.2 和 provider 初始化均正常。
- Base64/JWT 升级后现有 token 与编码行为保持兼容。
- server 与 Android native 均通过构建和生命周期 smoke。
- 变更代码中的中文注释、持久化边界说明和相关文档已同步。

### 7.1 本轮验证记录（2026-07-29）

已通过：

```text
cargo +stable fmt --all -- --check
cargo +stable metadata --locked --offline --no-deps
cargo +stable check -p winestock-core --all-targets --locked
cargo +stable test -p winestock-core --locked                    # 121 passed
cargo +stable check --workspace --all-targets --locked
cargo +stable test --workspace --locked                          # 145 passed
cargo +stable tree --workspace -d --locked --offline
cargo +stable tree --workspace -e features --locked --offline
cargo +stable build -p winestock-server --release --locked
cargo ndk -t arm64-v8a -P 28 check -p winestock-android --locked
gradlew.bat :app:testDebugUnitTest :app:assembleDebug :app:assembleRelease --no-daemon
```

Gradle 构建同时通过 Debug/Release native library 的 ELF、JNI、ABI 校验，前端资源校验以及最终 APK 包级校验。使用的主机 Rust 为 `1.96.1`，Android NDK 为 `30.0.14904198`。

尚未在本轮自动验证：SeaORM 1.1.20 时代的真实生产数据库副本、server Ctrl+C 交互 smoke、真实 LCSC HTTPS 成功/证书失败，以及 ARM64 真机上的数据库与 HTTPS 路径。这些属于发布前外部环境验收，不影响已完成的编译、测试和产物兼容整改，但不得在发布记录中标为已覆盖。

## 8. 回退条件

出现以下任一情况时，不以兼容层或跳过测试方式继续推进，应整体回退到 SeaORM 1.1.20 / SQLx 0.8.6：

- 无法在当前 Android Rust 工具链满足 Rust 1.94；
- SQLx 0.9 与 Android NDK/bundled SQLite 构建链不兼容；
- 任一交付环境实际 SQLite 低于 3.35，导致 SeaORM 连接拒绝；
- 已有数据库出现无法解释的 migration、WAL、事务或返回值差异；
- raw API 改造需要改变业务 SQL 或 repository 公共语义；
- Rustls feature 收窄导致 provider、TLS 1.2、证书校验或 Android HTTPS 回归；
- Base64/JWT 升级导致已签发 token 或 Android 本地交换不兼容；
- Release APK 的 native 依赖、体积或启动行为出现未能定位的回归。

回退按独立整改批次执行：Tokio/Rustls feature 收窄可以单独回退；SeaORM/SQLx/Base64/JWT 版本批次必须同时恢复 `Cargo.toml`、`Cargo.lock` 和对应调用点，不保留双版本适配层。

## 9. 非目标

- 不启用 SeaORM entity-first/schema sync。
- 不把现有手写 SQL 全面重写为 Entity/SeaQuery。
- 不把全部手写写入机械改成 RETURNING；只迁移能保持模型语义并减少查询的路径。
- 不用 SeaORM entity returning API 重写现有手写 SQL repository。
- 不接入 SeaORM RBAC/`RestrictedConnection`。
- 不修改数据库表、索引、migration 历史或数据格式。
- 不修改 HTTP API、OpenAPI、前端生成类型或平台 Shell Bridge。
- 不更换 HTTP client、JWT 实现或 TLS provider。
- 不为消除小规模重复版本强制 patch 上游 crate。
- 不在本批次用自研迁移执行层替换 SeaORM Migration。

## 10. 官方依据

- [SeaORM 2.0.0 Changelog](https://github.com/SeaQL/sea-orm/blob/2.0.0/CHANGELOG.md)
- [SeaORM 2.0 API](https://docs.rs/sea-orm/2.0.0/sea_orm/)
- [SeaORM `ConnectionTrait`](https://docs.rs/sea-orm/2.0.0/sea_orm/trait.ConnectionTrait.html)
- [SeaORM `RestrictedConnection`](https://docs.rs/sea-orm/2.0.0/sea_orm/struct.RestrictedConnection.html)
- [SQLite RETURNING](https://www.sqlite.org/lang_returning.html)

Context7 当前可检索的 SeaORM 2.0 文档索引仍以 release candidate API 为主；最终破坏性变化以 SeaQL 官方 `2.0.0` changelog 和 docs.rs 稳定版 API 为准。
