# `core/src/persistence/repository/validation.rs`

本文件定义 repository 写库输入的内部校验工具。

## 使用边界

- `validate_repository_input()` 在 repository 写库前执行 `garde` 校验。
- 校验失败会转换为 `DbErr::Custom`，不会继续进入 SeaORM 写库。
- 本文件只处理内部输入实体的静态字段约束，不替代数据库唯一约束、外键约束或事务校验。

## 自定义函数

| 函数                              | 用途                                      |
|---------------------------------|-----------------------------------------|
| `validate_repository_input`     | 统一执行 repository 输入实体的 `garde::Validate` |
| `validate_optional_positive_id` | 可空；存在时必须是正整数                            |
