# Server Shell 发布前审查

审查日期：2026-08-06

审查范围：`winestock-server` 的发布构建、启动配置、网络暴露、进程生命周期、错误退出和现有验证。

## 结论

代码层面的发布阻塞项已处理，可以进入实际发布目录和服务账户验收。

## 已修复问题

### Release doctest 条件编译边界

涉及：`server/src/lib.rs`、`core/src/http/docs.rs`、`core/src/http/mod.rs`、`core/src/lib.rs`

原问题是 server 的 doctest 在 Release 测试过程中仍满足 `debug_assertions`，但 Release core 不导出 `OPENAPI_JSON_PATH`，导致：

```text
error[E0432]: unresolved import `winestock_core::OPENAPI_JSON_PATH`
```

现已将路径常量与 Debug 文档路由解耦：路径常量在所有构建导出，OpenAPI/Swagger 路由仍只在 Debug 注册。通过以下命令验证：

```text
cargo +stable test -p winestock-server --release --locked
```

### 默认配置改为 loopback

位置：`server/src/config.rs:63-65`

缺少配置文件时，server shell 现已默认生成：

```text
mode = server-mode
bind_host = 127.0.0.1
```

这与项目级网络规则一致：首次启动只允许本机访问；需要局域网服务时，部署者必须显式修改绑定地址并自行完成防火墙和访问控制验收。

### Axum 服务异常退出监控

位置：`server/src/lib.rs:38-43`

server shell 现在同时等待 Ctrl+C 和 core 服务 task 状态。core 已提供的 `RunningLocalService::wait()` 和 `is_finished()` 已接入 server 主循环。

如果 Axum serve task 因运行错误或 panic 结束，server 会等待并取得底层错误，返回非零结果，不再保持假运行状态。

验证范围：

- Ctrl+C，执行 graceful shutdown；
- 本地服务 task 结束，输出错误并以失败结果退出。

## 发布前剩余项

### P2：单文件部署目录需要可写

位置：`server/src/config.rs:15-17`、`server/src/config.rs:28-34`

配置、默认数据库和文件目录位于单文件可执行文件旁的 `data/` 目录。发布到只读目录或受限服务账户时，首次启动可能无法创建配置或数据库。
这是单文件分发模型的部署约束：发布包必须放在服务账户可写目录，或由部署流程预先创建并授权 `data/` 目录。

## 已验证项

- `cargo +stable fmt --all -- --check` 通过。
- `cargo +stable check -p winestock-server --locked` 通过。
- `cargo +stable check -p winestock-server --features swagger-ui --locked` 通过，并编译 Swagger UI 依赖。
- `cargo +stable build -p winestock-server --release --locked` 通过。
- `cargo +stable test -p winestock-server --release --locked` 通过，包含 doctest。
- Debug + Swagger OpenAPI 测试 6 项通过，包含 Swagger UI 200 响应验证。
- server Debug 单元测试 7 项通过。
- Release 无 feature 的依赖树不包含 `utoipa-swagger-ui`。

## 发布前最小验收顺序

1. 在实际发布目录以目标服务账户启动，验证 `data/config.json`、SQLite、文件目录、端口占用和 Ctrl+C 关闭。
2. 确认发布目录和服务账户对 `data/` 具有写权限；单文件发布到只读目录时必须改用可写发布路径。
3. 用干净发布目录验证 Release 制品不包含 Swagger UI、OpenAPI 路径和 Debug 启动输出。
