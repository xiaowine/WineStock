# Core 字段限制文档

本目录记录 `core/src` 中 HTTP DTO、数据库实体和 repository 输入的字段限制、校验入口与约束来源。

文件按实体所在源码文件命名，路径分隔符用 `-` 展开。例如 `core/src/auth/contract.rs` 对应 `core-src-auth-contract.md`。

## 约束来源

- HTTP 请求 DTO：优先使用 `garde` 内置规则，在 `ValidatedJson<T>` extractor 中校验；校验失败不会进入业务 handler。
- repository 写库输入：写库前执行 `garde` 校验，校验失败不会继续进入 SeaORM insert/update。
- 数据库实体：由 SeaORM entity、migration 中的 SQL 约束、索引和 repository 输入共同保护；不作为 HTTP 请求体直接接收。
