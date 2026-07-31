# Rust 依赖与 Feature 审计

状态：静态审计已完成；低风险 feature 裁剪与 SeaORM 2.0 兼容整改已实施。

日期：2026-07-29

## 目标

检查当前 Rust workspace 是否开启了业务未使用的依赖 feature，并区分：

- 可直接缩减的项目配置；
- 被工作区统一配置放大的 feature；
- 由上游 crate 强制开启、无法在项目侧单独关闭的 feature；
- 看似可关闭但实际被源码使用的 feature。

本文最初只检查 `Cargo.toml`、源码使用点和 Cargo 实际解析出的依赖树；后续实施结果已合并记录，详细迁移和验证见 [`rust-dependency-remediation.md`](rust-dependency-remediation.md)。

## 审计方法

主要使用以下命令核对实际解析结果：

```text
cargo tree --workspace -e features --locked --offline
cargo tree --workspace -d --locked --offline
cargo tree -e features -i tokio --workspace --locked --offline
cargo tree -e features -i rustls --workspace --locked --offline
cargo tree -e features -i rustls-platform-verifier --workspace --locked --offline
cargo tree -e features -i sqlx@0.9.0 --workspace --locked --offline
cargo info <crate>@<version> --offline
```

Cargo resolver v2 不会在部分目标依赖、构建依赖和未参与当前构建的开发依赖之间统一 feature；普通依赖在同一解析图中仍采用 feature 并集。成员包从 `[workspace.dependencies]` 继承的 feature 会进入对应构建图，成员额外声明的 feature 只能增加，不能减去工作区已开启的 feature。

## 结论摘要

按预期收益和处理优先级排序：

1. `sea-orm-migration 2.0.0` 间接开启大量 SeaORM、SQLx 和类型生态 feature，是当前最主要的依赖膨胀来源。
2. Tokio 的 `signal` 在工作区根统一开启，但只有 server 使用，可按 crate 拆分。
3. Rustls 直接依赖的默认 `logging` 和 `prefer-post-quantum` 没有对应源码需求，可评估关闭。
4. Reqwest `json` 当前确实用于请求序列化，不能关闭。
5. Axum 存在未使用的默认 feature，但 `utoipa-axum` 会重新开启，单改 Axum 声明没有收益。
6. `jsonwebtoken/rust_crypto` 包含当前 HS256 业务不使用的非对称算法，但上游没有按算法拆分的细粒度 feature。

### 落地结果（2026-07-29）

- 已将 Tokio `signal` 收窄到 `server`。
- 已关闭直接 Rustls defaults，只保留 `std`；依赖树确认 Reqwest 继续提供 `aws-lc-rs` 和 TLS 1.2。
- 已升级并验证 SeaORM / SeaORM Migration 2.0、SQLx 0.9、Base64 0.23 与 JSON Web Token 11。
- 保留 Reqwest JSON、Axum defaults、JWT `rust_crypto` 和 SeaORM Migration；这些项目受实际源码或上游 feature 结构约束，继续裁剪需要独立架构改动。
- `sqlite-use-returning-for-3_35` 已由工作区直接声明，同时仍有 SeaORM Migration defaults 的传递来源；项目已增加 SQLite `>= 3.35` 测试门槛，并分两批选择性迁移十条能够减少查询的生产路径。

## 1. SeaORM Migration 依赖膨胀

### 整改前配置

工作区已经对 SeaORM、SeaORM Migration 和 SQLx 关闭默认 feature，并只显式请求 Tokio 与 SQLite：

```toml
sea-orm = { version = "2.0.0", default-features = false, features = ["macros", "runtime-tokio", "sqlx-sqlite"] }
sea-orm-migration = { version = "2.0.0", default-features = false, features = ["runtime-tokio", "sqlx-sqlite"] }
sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "sqlite"] }
```

### 实际解析结果

尽管项目侧关闭了 `sea-orm-migration` 默认 feature，实际依赖树仍由该 crate 的内部依赖开启 SeaORM defaults，包括：

- `with-chrono`
- `with-json`
- `with-rust_decimal`
- `with-time`
- `with-uuid`
- `stream`
- `schema-sync`
- `sqlite-use-returning-for-3_35`

这些 feature 又向下开启 SQLx 的：

- `chrono`
- `json`
- `rust_decimal`
- `time`
- `uuid`
- `sqlite-deserialize`
- `sqlite-load-extension`
- `sqlite-unlock-notify`

依赖树中因此出现 Arrow、BigDecimal、UUID、Time、Rust Decimal、Sea Schema 等包。当前业务实体和持久化源码没有使用 chrono、UUID 或 decimal 类型。

### 判断

这是当前收益潜力最高的问题，但不能通过继续给项目依赖添加 `default-features = false` 解决。`sea-orm-migration 2.0.0` 的内部依赖方式已经把 SeaORM defaults 带入统一 feature 图。

### 可选方向

- 评估不在运行时依赖 `sea-orm-migration`，改用项目拥有的窄迁移执行层。
- 检查后续版本是否允许关闭其内部 SeaORM defaults。
- 对比依赖树更窄的兼容版本，并完整验证 migration 与数据库兼容行为。

任何方案都涉及数据库启动和迁移边界，不能只依据包体变化直接切换。

## 2. Tokio Feature 作用域过宽

### 当前配置

```toml
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "signal", "sync", "time"] }
```

所有通过 `workspace = true` 使用 Tokio 的成员都会继承这组 feature。

### 源码使用情况

- `server` 使用 `#[tokio::main]` 和 `tokio::signal::ctrl_c()`。
- `android/native` 使用多线程 Tokio Runtime。
- `core` 使用 TCP listener、task、semaphore 和测试宏。
- 只有 `server` 直接使用 `signal`。

### 建议

从工作区根移除 `signal`，仅在 `server/Cargo.toml` 增加：

```toml
tokio = { workspace = true, features = ["signal"] }
```

这样 server 保持现有行为，Android Release 构建不再因项目直接声明而启用 Tokio signal 支持。

`macros` 不具有同等明确的裁剪收益，因为 Axum 的 `tokio` feature 也会开启 Tokio macros。其余 `net`、runtime、sync 和 time 能力由项目源码或主要传递依赖使用，不建议在没有按目标构建验证的情况下删除。

### 实施状态

已按建议实施。工作区根不再声明 `signal`，`server/Cargo.toml` 单独追加该 feature；最终依赖树中 `tokio feature "signal"` 的项目来源只有 `winestock-server`。

## 3. Rustls 默认 Feature

### 整改前配置

```toml
rustls = "0.23"
```

Rustls 0.23.42 默认开启：

- `aws_lc_rs`
- `logging`
- `prefer-post-quantum`
- `std`
- `tls12`

项目只使用 `RootCertStore` 和 `ClientConfig` 构造基于 `webpki-roots` 的自定义 TLS 配置，没有直接使用 Rustls logging，也没有明确要求 post-quantum 优先策略。

### 建议

可评估改为：

```toml
rustls = { version = "0.23", default-features = false, features = ["std"] }
```

当前 `reqwest/rustls` 会在统一 feature 图中提供 `aws-lc-rs` 和 TLS 1.2。该调整的目标是移除直接 Rustls 依赖额外开启的 `logging` 与 `prefer-post-quantum`，不是移除 TLS provider。

实施后必须验证：

- server Release 的立创商城 HTTPS 查询；
- Android Release JNI 内的首次 TLS 握手；
- HTTP/1.1 ALPN 限制；
- 证书校验失败路径。

### 实施状态

已按建议实施。工作区、server Release、Android ARM64 native 和 Debug/Release APK 均构建通过；依赖树保留 `aws-lc-rs`、`std`、`tls12`，并移除了直接依赖带入的 `logging`、`prefer-post-quantum`。真实 server/Android HTTPS 请求和证书失败注入仍需发布前真机验证。

## 4. Reqwest Feature

当前配置关闭 Reqwest defaults，只开启：

```toml
reqwest = { version = "0.13.4", default-features = false, features = ["json", "rustls"] }
```

### `json`

不能关闭。LCSC client 调用 `.json(&request)` 序列化请求体，该方法由 `reqwest/json` 提供。响应虽然使用 `serde_json::from_slice` 手工解析，仍不影响请求侧对该 feature 的需求。

### `rustls-platform-verifier`

项目向 Reqwest 传入自定义 Rustls `ClientConfig`，运行时使用内置 `webpki-roots`，不会走默认平台根证书策略。但是 Reqwest 0.13.4 的公开 `rustls` feature 本身包含 `rustls-platform-verifier`，因此该包仍会参与编译。

`rustls-no-provider` 同样包含 platform verifier，只是不选择加密 provider，不能解决这一依赖。若必须完全移除，需要改变 Reqwest TLS 集成方式、使用上游提供更细 feature 的版本，或更换 HTTP client；不建议仅为删除一个传递依赖扩大 HTTP/TLS 改动面。

## 5. Axum 默认 Feature

Axum 默认开启：

- `form`
- `http1`
- `json`
- `matched-path`
- `original-uri`
- `query`
- `tokio`
- `tower-log`
- `tracing`

当前源码明确使用 `http1`、`json`、`query`、`tokio` 和额外的 `multipart`，没有找到 `Form`、`MatchedPath` 或 `OriginalUri` 使用点，也没有项目级 tracing/log layer。

但 `utoipa-axum 0.2.0` 会重新开启 Axum defaults。因此只把直接 Axum 依赖改为 `default-features = false`，最终解析图仍包含这些 feature，不产生实际裁剪收益。

如需处理，必须同时评估：

- Release 是否仍需要 `utoipa-axum` 的路由组装能力；
- 是否能在 Debug/OpenAPI 导出路径之外将其 feature-gate；
- 是否值得用普通 Axum Router 替代 OpenAPI Router 组装。

该方向会覆盖 HTTP 路由结构和 OpenAPI 导出，不属于低风险依赖配置调整。

## 6. JSON Web Token 算法依赖

项目业务只使用 HS256，但 `jsonwebtoken` 的 `rust_crypto` backend 同时包含 HMAC、RSA、P256、P384 和 Ed25519 等实现。当前上游 feature 按加密 backend 而非单个 JWT 算法划分，项目侧无法只保留 HS256。

可选方向包括自定义更窄的 provider 或更换只包含 HMAC JWT 的实现，但这属于鉴权实现变更。必须覆盖登录、刷新、登出、过期 token、错误 token、旧 token 兼容和 Android 本地免登录交换流程，不应作为普通 feature 清理直接实施。

## 7. 版本重复

这不是 feature 问题，但依赖树还存在以下可见重复：

- 项目直接使用 `base64 0.23`，传递依赖中存在 `base64 0.22`。
- 项目直接使用 `sha2 0.11`，JWT 与其他加密依赖中存在 `sha2 0.10`。

两者可能通过对齐直接依赖版本减少重复代码，但需要先确认 API、锁文件和安全维护要求。与 SeaORM 默认 feature 相比，这部分预期收益较小。

## 实施结果与后续顺序

1. 已完成：将 Tokio `signal` 从工作区公共 feature 移到 server。
2. 已完成配置与构建验证：关闭直接 Rustls defaults；真实 server/Android HTTPS 留作发布前验收。
3. 后续：独立评估 `sea-orm-migration 2.0.0` 的替代方案和 Release 二进制组成；该项收益最大，但风险也最高。
4. 已完成两批：显式声明 `sqlite-use-returning-for-3_35`，迁移三条 raw SQL 和七条 Entity 创建路径；其它无收益或模型不等价的路径保留原实现。
5. 只有在包体数据证明值得时，再评估 `utoipa-axum` Release feature-gate、JWT 实现替换及 Base64/SHA2 重复版本。

## 建议验收

实施任何一项后，至少执行覆盖对应边界的最窄检查：

```text
cargo tree --workspace -e features --locked --offline
cargo check -p winestock-core --release
cargo test -p winestock-core --release
cargo build -p winestock-server --release
```

涉及 Android 依赖树或 TLS 时，还应执行 Android Release native/APK 构建验收，并在真实 ARM64 设备验证自托管启动和 LCSC HTTPS 查询。涉及迁移依赖时，应使用已有数据库和新数据库分别验证启动、升级、回滚失败呈现与数据保留。
