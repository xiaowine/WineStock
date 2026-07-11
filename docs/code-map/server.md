# Server 代码地图

`server` 是正式无头服务端平台 shell，负责进程生命周期和共享 Axum 服务启动，不拥有前端资源。

## 源码

- `server/src/main.rs`
  - 二进制入口，调用 `winestock_server::run()`。
  - 打印启动错误及其 source 链。

- `server/src/lib.rs`
  - 编排服务端生命周期。
  - 加载固定配置、校验运行模式、准备存储目录、启动 core、绑定服务并等待 Ctrl+C。
  - 打印 API、OpenAPI 和 Swagger UI 地址。
  - 绑定所有接口时只展示 loopback URL，不把 `0.0.0.0` 作为可打开地址。

- `server/src/config.rs`
  - 使用当前可执行文件目录固定定位 `data/config.json`。
  - 为 server shell 构造相对于 `data` 目录的默认存储配置，并调用 shared 加载或创建 JSON 文件。
  - 相对存储路径以配置文件所在目录为基准。
  - 确保 server shell 只运行本地服务模式，并在 core 打开 SQLite 前创建存储目录。

- `server/src/error.rs`
  - 定义 `ServerShellError`，集中配置、存储准备、core 启动和服务启动错误。

- `server/src/tests/`
  - `lib.rs` 覆盖 shell 生命周期相关行为。
  - `config.rs` 覆盖固定配置路径、server 默认配置创建、运行模式和路径解析。

## 运行流程

```text
server/src/main.rs
  -> winestock_server::run()
  -> config::fixed_config_path()
  -> config::load_config()
  -> config::ensure_server_runtime()
  -> config::prepare_storage_dirs()
  -> winestock_core::bootstrap_from_config().await
  -> winestock_core::bind_server()
  -> BoundServer::serve_local_with_shutdown()
  -> winestock_core::build_router_with_local_service()
```

固定配置位置是运行时可执行文件同目录下的 `data/config.json`。
