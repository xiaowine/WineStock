# 实体限制文档

本目录记录当前实体的字段限制、校验入口和约束来源。

文件按实体所在源码文件命名，路径分隔符用 `-` 展开。例如 `core/src/auth/contract.rs` 对应 `core-src-auth-contract.md`。

## 约束来源

- HTTP 请求 DTO：优先使用 `garde` 内置规则，在 `ValidatedJson<T>` extractor 中校验；校验失败不会进入业务 handler。
- JSON 启动配置：`AppConfig::from_json_str()` 反序列化后立即执行 `garde` 校验。
- repository 写库输入：写库前执行 `garde` 校验，校验失败不会继续进入 SeaORM insert/update。
- 数据库实体：由 SeaORM entity、migration 中的 SQL 约束、索引和 repository 输入共同保护；不作为 HTTP 请求体直接接收。
