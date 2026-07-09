# `core/src/validation.rs`

本文件定义 core 内部复用的 `garde` 自定义校验入口。

## 自定义规则边界

优先使用 `garde` 内置规则，例如 `length`、`range`、`ip`、`dive`、`inner` 和 `skip`。

基础文本规则从 `shared/src/text_validation.rs` 复用。
仅在内置规则不能直接表达 core 业务语义时在本文件新增自定义规则：

- 权限代码只能使用项目允许的字符集。

## 函数

| 函数                            | 用途                       |
|-------------------------------|--------------------------|
| `validate_not_blank`          | 从 `shared/src/text_validation.rs` 复用 |
| `validate_optional_not_blank` | 从 `shared/src/text_validation.rs` 复用 |
| `validate_code_list`          | 权限代码列表，每项必须满足项目代码格式     |
