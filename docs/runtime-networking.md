# 运行模式与网络

本文定义正式项目的运行模式、地址选择和网络边界。

## 运行模式

### `client-only`

应用不启动本地 Axum 服务，连接 `remote_base_url`。

适用于当前设备只作为其它服务端客户端的场景。

### `connect-to-remote`

应用不启动本地 Axum 服务，连接 `remote_base_url`。
该模式为可作为客户端的平台保留显式远端连接配置。UI 可以把它与 `client-only` 放在同一设置区域，但必须保留已存模式，或明确执行规范化，不能静默改写。

### `self-hosted`

应用为自身 UI 启动本地 Axum 服务，UI 连接 `http://127.0.0.1:<port>`。

这是本地应用的默认模式，不要求局域网暴露。

### `server-mode`

应用启动 Axum，允许其它客户端连接，并绑定明确的 `bind_host` 与 `port`。

适用于当前设备需要被其它设备访问的场景，也是纯 Server Shell 的自然运行模式。

## 配置字段

共享配置模型包含以下字段；纯 Server Shell 在运行时可以只使用服务端子集。

```toml
[server]
mode = "self-hosted"
bind_host = "127.0.0.1"
port = 17890
auto_start_server = true
remote_base_url = ""
```

### `mode`

决定当前配置启动本地 Axum，还是连接远端服务。
不得增加独立 `server.enabled` 开关，否则会与 `mode` 重复并产生矛盾配置。

### `bind_host`

决定 Axum 监听地址。

- 本机自用使用 `127.0.0.1`；
- 只有明确允许外部连接时才使用具体局域网 IP 或 `0.0.0.0`。

### `port`

决定服务端口。`server-mode` 使用用户配置的固定端口；UI 平台的 `self-hosted` 可以在首次 apply 或固定端口冲突时使用临时值 `0` 请求操作系统分配端口，绑定成功后必须回写并持久化实际端口。

平台代码不得硬编码端口，并必须显式处理端口冲突。`0` 不得写入运行中快照、持久化配置或可访问 URL。

### `remote_base_url`

决定客户端模式连接的远端服务地址。纯服务端进程仅暴露自身 API 时不要求该字段。

地址必须按需要包含 scheme、host 和 port，例如：

```text
http://192.168.1.23:17890
```

### `auto_start_server`

决定平台 Shell 是否自动启动本地 Axum 服务。

该共享字段继续用于无头 Server 配置和兼容。带 UI 的平台不把它作为普通设置项：本地模式在配置已 initialized
后持久化或规范化为 `true`，首次缺少配置时等待前端 apply；远端模式不启动本地服务。

## UI 平台地址选择

桌面端和 Android 独立加载平台打包的前端资源，因此 WebView 页面地址不是 API 地址。

平台 Shell 通过 Shell Bridge 返回实际 API 根地址：

- 本地模式使用实际绑定端口派生的 loopback URL；
- 远端模式使用校验后的 `remote_base_url`；
- `0.0.0.0` 和 `::` 永远不能作为浏览器或 WebView 访问主机返回。

前端拥有配置和恢复 UI，Shell 负责持久化、校验和应用配置。
API 未配置、正在启动、已停止或失败时，运行设置仍必须可用。完整契约见 `docs/shell-bridge.md`。

### Android 当前策略

- Android `self-hosted` 只接受 `127.0.0.1`，native/shared 校验拒绝局域网绑定；
- Application 级 manager 只自动激活已有有效持久配置；首次缺失配置时发布 `initialized=false` 和默认草稿，
  不写配置、不启动本地 Axum HTTP 服务；
- 前端始终从 Android 打包资源启动，首次选择模式并成功调用 `applyRuntimeConfig` 后，Shell 才启动本地
  core 或切换到远端地址，并持久化正式配置；
- WebView 使用 core 返回的实际 `http://127.0.0.1:<bound-port>`，不使用监听通配地址；
- 切换远端模式时，先停止 manager 拥有的本地服务，再提交远端配置；
- 候选本地配置只有在 bind/bootstrap 成功后才持久化；激活或持久化失败时尽力恢复旧服务；
- Foreground Service 要求实现前，Android `server-mode` 保持不可用。

## 地址规则

`0.0.0.0` 只能作为监听地址，不能作为 WebView URL，也不能显示为用户应打开的地址。

当 `bind_host = "0.0.0.0"` 时：

- Server Shell 可以显示本机 loopback URL；
- 局域网访问必须显示主机真实 IP，例如：

```text
http://192.168.1.23:17890
http://10.0.0.8:17890
```

本机自用始终优先：

```text
http://127.0.0.1:<port>
```

只有存在明确平台原因时才使用 `localhost`；默认使用 `127.0.0.1`，避免名称解析差异。

## 访问模式

### 应用访问自身

使用 `self-hosted`，默认绑定 `127.0.0.1`。平台打包 UI 访问 `http://127.0.0.1:<port>`。
本机模式的端口由 UI Shell 自动选择并持久化实际值，前端不展示端口输入框；地址只使用 Shell 返回的实际 API 根地址。

### 其它设备访问当前应用

使用 `server-mode`，绑定明确局域网 IP 或 `0.0.0.0`。
如果绑定全部接口，应提示其它设备使用当前主机真实局域网 IP，并由平台 Shell 处理防火墙与权限。
这是纯 Server Shell 的主要访问模式。

### 应用访问其它设备

使用 `client-only` 或 `connect-to-remote`，设置 `remote_base_url`，不启动本地服务。
该场景主要适用于带 UI 或具备客户端能力的平台。

## 端口冲突

端口冲突行为必须明确：

- `self-hosted` 可以自动重试一次端口 `0`，成功后发布并持久化新的实际地址；
- `server-mode` 返回清晰的端口占用错误，要求用户选择其它固定端口；
- 任一模式都不得静默改变端口而不更新 WebView 实际 URL 与用户可见状态。

带 UI 的平台通过 Shell Bridge 状态在前端呈现冲突和重试，不弹原生 Shell 对话框。

## Server Shell

`server/` 是运行共享 Axum core 的正式无头 Shell。

- 固定读取可执行文件旁的 `data/config.json`；
- 不接受配置路径参数；
- 配置不存在时先创建默认 JSON，再继续启动；
- 相对存储路径以 `data` 目录为基准，因此默认数据库和文件仓与配置文件同目录；
- 初始化 core，绑定 `server.bind_host` 与 `server.port`，并在绑定后输出实际访问地址。

服务暴露：

```text
/api-docs/openapi.json     # 仅 Debug 构建
/swagger-ui                # 仅 Debug 构建且启用 Swagger UI feature
```

Release 构建不注册 OpenAPI JSON 或 Swagger UI，并返回统一 JSON 404；最终制品不编译或链接 Swagger UI。

当 `bind_host = "0.0.0.0"` 时，Server Shell 输出本机 loopback URL，不把 `0.0.0.0` 输出为可打开地址。

## 安全说明

局域网暴露与本机自用是两个独立选择。

- 默认不得绑定 `0.0.0.0`；
- 未经明确配置或用户操作不得启用 server mode；
- 不得假设 Android、桌面端和 Server 主机的防火墙行为一致。
