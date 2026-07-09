# `shared/src/config_validation.rs`

本文件定义共享运行配置使用的 `garde` 自定义校验函数。

## 自定义规则边界

优先使用 `garde` 内置规则，例如 `length`、`range`、`ip`、`dive` 和 `skip`。

仅在内置规则不能直接表达共享运行配置语义时使用本文件的自定义规则：

- `remote_base_url` 允许空字符串，但非空时必须是 HTTP(S) URL。

## 函数

| 函数                           | 用途                                         |
|------------------------------|--------------------------------------------|
| `validate_optional_http_url` | 空字符串表示未配置；非空时必须以 `http://` 或 `https://` 开头 |
