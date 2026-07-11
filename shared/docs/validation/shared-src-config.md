# `shared/src/config.rs`

本文件定义平台无关启动配置实体。

## `AppConfig`

校验入口：`AppConfig::from_json_str()`。

| 字段        | 限制                                |
|-----------|-----------------------------------|
| `server`  | `garde dive` 递归校验 `ServerConfig`  |
| `storage` | `garde dive` 递归校验 `StorageConfig` |

## `RuntimeMode`

该实体是 enum，允许值由 Serde 反序列化限制。

JSON 允许值：

- `client-only`
- `self-hosted`
- `server-mode`
- `connect-to-remote`

## `ServerConfig`

校验入口：`AppConfig::from_json_str()`。

| 字段                  | 限制                                                               |
|---------------------|------------------------------------------------------------------|
| `mode`              | enum 值由 Serde 限制                                                 |
| `bind_host`         | `garde length(min = 1, max = 255)`；`garde ip`，必须是 IPv4 或 IPv6 地址 |
| `port`              | `garde range(min = 1)`                                           |
| `auto_start_server` | 布尔值，无额外字段校验                                                      |
| `remote_base_url`   | `garde length(max = 2048)`；可空；非空时必须以 `http://` 或 `https://` 开头   |

运行模式相关限制仍由平台壳和 core 启动逻辑校验，例如 server shell 不支持远端-only 模式。

## `StorageConfig`

校验入口：`AppConfig::from_json_str()`。

| 字段              | 限制                                           |
|-----------------|----------------------------------------------|
| `database_path` | `garde length(min = 1, max = 4096)`；trim 后非空 |
| `files_dir`     | `garde length(min = 1, max = 4096)`；trim 后非空 |
| `auto_migrate`  | 布尔值，无额外字段校验                                  |

路径是否存在、相对路径如何解析、目录创建失败等属于平台壳和 core 存储启动阶段校验。
