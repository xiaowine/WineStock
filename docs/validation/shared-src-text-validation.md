# `shared/src/text_validation.rs`

本文件定义平台无关、无业务语义的基础文本校验函数。

## 自定义规则边界

本文件只放可被配置、HTTP DTO 和写库输入共同复用的基础文本规则。
具体业务规则，例如权限代码字符集，仍归业务所属 crate 或模块。

## 函数

| 函数                            | 用途              |
|-------------------------------|-----------------|
| `validate_not_blank`          | 字符串 trim 后非空    |
| `validate_optional_not_blank` | 可空；存在时 trim 后非空 |
