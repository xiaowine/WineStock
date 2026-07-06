# JSON 启动配置与数据库 Auth 配置方案

## 文档状态

本文是后续实现配置系统和鉴权内部配置的方案文档，用于记录设计意图和实现方向。

除非被正式规范文档引用，本文不作为日常 agent 的强制约束。

## 目标

WineStock 使用 JSON 文件保存启动前必须知道的配置，只保留 `server` 和 `storage` 两类配置。

鉴权相关配置和 JWT 签名密钥全部保存到数据库，由系统初始化和管理，不让用户在 JSON 中手动配置。

这样可以避免：

- 用户误配 JWT secret。
- 配置文件泄漏时直接泄漏签名密钥。
- core 硬编码端口、数据库路径、文件目录。
- 桌面、Android、server shell 各自定义不同配置格式。

## JSON 配置范围

JSON 文件只保存启动服务前必须知道的信息：

```json
{
  "server": {
    "mode": "self-hosted",
    "bind_host": "127.0.0.1",
    "port": 17890,
    "auto_start_server": true,
    "remote_base_url": ""
  },
  "storage": {
    "database_path": "data/winestock.sqlite",
    "files_dir": "data/files",
    "auto_migrate": true
  }
}
```

不放入 JSON 的内容：

- JWT signing secret。
- access token TTL。
- refresh token TTL。
- 密码策略。
- 默认用户密码。
- refresh token 轮换策略。

这些内容属于服务内部状态，保存在数据库中。

## 配置职责边界

`shared`：

- 定义 `AppConfig`。
- 定义 `ServerConfig`。
- 定义 `StorageConfig`。
- 定义 `RuntimeMode`。
- 支持 JSON 序列化和反序列化。

`core`：

- 接收已经解析好的配置对象。
- 根据 `StorageConfig` 打开数据库。
- 根据数据库内容初始化 auth 设置和签名密钥。
- 根据 `ServerConfig` 启动或构建服务能力。
- 不查找配置文件路径。
- 不硬编码数据库路径、文件目录、端口或 JWT 密钥。

平台壳：

- 决定配置文件放在哪里。
- 读取 JSON 配置文件。
- 补齐平台默认路径。
- 解析相对路径。
- 将最终配置传给 `core`。

## 运行模式与 Storage 使用规则

`self-hosted`：

- 启动本地 Axum。
- 需要 `storage`。
- 打开本地数据库。
- 初始化 auth 内部配置。

`server-mode`：

- 启动本地 Axum，并允许其他设备访问。
- 需要 `storage`。
- 打开本地数据库。
- 初始化 auth 内部配置。

`client-only`：

- 不启动本地 Axum。
- 连接 `remote_base_url`。
- 不打开本地数据库。
- 不初始化 auth 内部配置。
- `storage` 可以存在于配置文件中，但本模式运行时不使用。

## 数据库 Auth 设置

鉴权配置保存在数据库中，建议使用独立内部表。

### `auth_settings`

保存普通鉴权策略：

```text
key
value
updated_at
```

默认初始化值：

```text
access_token_ttl_seconds = 900
refresh_token_ttl_seconds = 604800
refresh_token_rotation = true
```

后续可以通过管理 API 或管理 UI 修改这些策略。

### `auth_signing_keys`

保存 JWT access token 的签名密钥。

```text
id
key_id
algorithm
key_material
status
created_at
activated_at
retired_at
```

字段含义：

- `key_id`：JWT header 中的 `kid`，用于定位签名密钥。
- `algorithm`：签名算法，v1 默认为 `HS256`。
- `key_material`：系统生成的随机签名密钥，必须可读原文，不能 hash。
- `status`：`active` 或 `retired`。
- `retired_at`：密钥退役时间。

初始化规则：

```text
1. migration 创建 auth_signing_keys 表。
2. bootstrap 检查是否存在 active key。
3. 不存在则生成随机签名密钥。
4. 插入一条 active signing key。
```

签发 JWT：

```text
读取 active signing key
JWT header 写入 kid
使用 key_material 签名 access token
```

验证 JWT：

```text
读取 JWT header 中的 kid
根据 kid 查询 auth_signing_keys
使用对应 key_material 校验签名和 exp
```

## Bootstrap 流程

本地服务启动前的初始化流程：

```text
1. 平台壳读取 JSON 配置。
2. 平台壳解析 database_path 和 files_dir。
3. core 打开 SQLite。
4. core 执行 migration。
5. core 初始化 auth_settings。
6. core 初始化 auth_signing_keys。
7. core 检查是否已有用户。
8. 如果没有用户，进入首次管理员初始化流程。
9. core 根据 server 配置启动 Axum。
```

默认管理员不使用固定账号密码。

建议方式：

- server shell：首次启动时生成一次性 setup token 或随机管理员密码，并打印到控制台。
- desktop/android：通过首次设置界面创建管理员。
- setup token 或临时密码使用后立即失效。

## 安全规则

- JSON 配置文件不包含 JWT secret。
- JWT signing key 由系统生成，不让用户手动配置。
- `key_material` 不能 hash，因为签发和验证 JWT 都需要原文。
- refresh token 仍然只保存 hash。
- 数据库泄漏时 signing key 也会泄漏，这是无外部 KMS/安全存储时的工程折中。
- 后续如需要更高安全性，可增加平台安全存储或外部 KMS，但不影响当前配置边界。

## 当前假设

- v1 配置文件格式为 JSON，不新增 TOML。
- v1 JSON 只包含 `server` 和 `storage`。
- v1 auth 配置、JWT signing key、默认策略全部在数据库初始化后管理。
- v1 JWT access token 使用 HS256。
- v1 不实现 signing key rotation 的完整管理 UI，但表结构预留 `active` / `retired`。
- v1 不使用固定默认管理员密码。
