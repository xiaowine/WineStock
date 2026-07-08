# `shared/src/validation.rs`

本文件定义共享 `garde` 自定义校验函数。

## 自定义规则边界

优先使用 `garde` 内置规则，例如 `length`、`range`、`ip`、`dive`、`inner` 和 `skip`。

仅在内置规则不能直接表达项目语义时使用本文件的自定义规则：

- 字符串 trim 后不能是空白。
- 可选字符串存在时不能是空白；长度限制优先交给 `garde length`。
- 权限代码只能使用项目允许的字符集。
- `remote_base_url` 允许空字符串，但非空时必须是 HTTP(S) URL。

## 函数

| 函数                            | 用途                                         |
|-------------------------------|--------------------------------------------|
| `validate_not_blank`          | 字符串 trim 后非空                               |
| `validate_optional_not_blank` | 可空；存在时 trim 后非空                            |
| `validate_code_list`          | 权限代码列表，每项必须满足项目代码格式                           |
| `validate_optional_http_url`  | 空字符串表示未配置；非空时必须以 `http://` 或 `https://` 开头 |
