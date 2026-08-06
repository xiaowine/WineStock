# Server 代码地图

`server` 是正式无头服务端平台 shell，负责进程生命周期和共享 Axum 服务启动，不拥有前端资源。

## 源码

- `server/src/main.rs`
  - 二进制入口；`--check-update` 检查 GitHub 最新正式 Release，其它调用进入 `winestock_server::run()`。
  - 打印启动错误及其 source 链。

- `server/src/lib.rs`
  - 编排服务端生命周期。
  - 加载配置、校验运行模式、准备存储目录，通过 core 统一运行句柄启动服务；同时等待 Ctrl+C、退出信号错误和服务 task 异常结束。
  - Debug 构建打印 OpenAPI 和 Swagger UI 地址；Release 构建不打印文档地址。
  - 绑定所有接口时只展示 loopback URL，不把 `0.0.0.0` 作为可打开地址。

- `server/src/config.rs`
  - 使用当前可执行文件目录固定定位 `data/config.json`。
  - 为 server shell 构造相对于 `data` 目录的默认存储配置，并调用 shared 加载或创建 JSON 文件。
  - 相对存储路径以配置文件所在目录为基准。
  - 确保 server shell 只运行本地服务模式，并在 core 打开 SQLite 前创建存储目录。

- `server/src/error.rs`
  - 定义 `ServerShellError`，集中配置、存储准备和统一 local-service 运行错误。

- `server/src/update.rs`
  - 通过 `--check-update` 显式检查 GitHub 最新正式 Release 并输出 Release 页面地址。
  - 不在常规服务启动时联网，也不下载或安装更新。

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
  -> winestock_core::start_local_service()
     -> bind_server（先占用端口）
     -> bootstrap_from_config
     -> serve_local_with_shutdown
  -> Ctrl+C：RunningLocalService::shutdown()
  -> 服务 task 异常结束：RunningLocalService::wait() -> 非零错误
```

固定配置位置是运行时可执行文件同目录下的 `data/config.json`。
