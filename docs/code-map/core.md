# Core 代码地图

`core` 是各平台 shell 共用的 Rust/Axum 服务库，拥有 HTTP API、服务生命周期、业务逻辑、共享状态和持久化集成。
它不能依赖 server、desktop、Android 或 frontend 资源。

## 子地图

- [`core/http-auth-users.md`](core/http-auth-users.md)：全局 HTTP 外壳、security、auth、users 和 RBAC。
- [`core/stock.md`](core/stock.md)：库存业务 controller、service、权限和启动补齐。
- [`core/persistence.md`](core/persistence.md)：SQLite/SeaORM 连接、migration、entity 和 repository。
- [`core/http-api.md`](core/http-api.md)：当前公共 HTTP 路径清单。

## 顶层入口

- `core/src/lib.rs`
  - 声明 `auth`、`bootstrap`、`http`、`local_service`、`persistence`、`rbac`、`security`、`server`、`state`、`stock` 和 `users`。
  - 重新导出公共启动入口、HTTP 构建入口、鉴权公开类型和运行时错误。
  - 重新导出 `winestock_shared`，但不直接承担 Router 细节。

- `core/src/state.rs`
  - 定义统一 `CoreState`，组合 `StorageRuntime` 和 `SecurityRuntime`。
  - 避免具体领域 runtime 充当整个服务的全局状态。

- `core/src/validation.rs`
  - 定义 HTTP DTO 和 repository 输入共享的业务字段校验函数。
  - 复用 shared 基础文本规则，不访问数据库或平台 shell。

- `core/src/files/`
  - `controller.rs`：图片 multipart 上传、受控读取/删除 DTO 和 handler。
  - `service.rs`：签名/MIME/大小校验、SHA-256 内容寻址、动态授权和 24 小时孤儿清理。
  - `error.rs`：文件 API 稳定错误码和启动清理错误。
  - 文件模块处理物品属性和入库属性共用的受控图片，不拥有前端文件选择器或客户端路径。

## 启动与服务生命周期

- `core/src/bootstrap.rs`
  - 定义 `CoreBootstrap`、`LocalServiceBootstrap` 和 `bootstrap_from_config()`。
  - 本地服务模式下打开存储、执行 migration、补齐 RBAC、库存默认数据、清理超期临时图片并初始化鉴权设置。
  - 远端-only 模式跳过本地存储初始化。

- `core/src/server.rs`
  - 定义 `BoundServer`、`ServerStartError` 和 `bind_server()`。
  - 负责按共享配置绑定 socket、报告端口冲突和优雅运行 Axum。
  - 不拥有平台进程生命周期或用户展示文本。

- `core/src/local_service.rs`
  - 定义 `start_local_service()`、`RunningLocalService`、`LocalServiceInfo` 和
    `LocalServiceRuntimeError`，统一平台 shell 的 bootstrap/bind/serve/shutdown 编排。
  - 先绑定端口再执行有副作用的 bootstrap，因此端口占用不会提前打开数据库或执行 migration。
  - 运行句柄报告实际绑定地址、管理员初始化状态、任务意外结束，并支持显式 graceful shutdown。

## 测试

- `core/src/tests/support.rs`：共享测试搭建。
- `core/src/tests/bootstrap.rs`、`server.rs`、`http_openapi.rs`：启动、服务和 HTTP 外壳。
- `core/src/tests/local_service.rs`：统一运行句柄的远端模式拒绝、端口冲突、实际地址和关闭后端口释放。
- `core/src/tests/security_authorization.rs`：授权中间件。
- `core/src/tests/auth_*.rs`、`users_*.rs`：鉴权和用户域。
- `core/src/tests/stock_*.rs`：库存业务。
- `core/src/tests/persistence_*.rs`：连接和 repository。
