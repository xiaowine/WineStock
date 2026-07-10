# Shared 代码地图

`shared` 是平台无关运行配置、配置解析错误和基础文本校验 crate。
它不能依赖 `core`、Axum、平台 shell、WebView 或前端构建产物，也不承载 HTTP DTO。

## 源码

- `shared/src/lib.rs`
  - 作为 crate 薄入口，公开 `config`、`error` 和 `text_validation`，私有声明 `config_validation`。
  - 重新导出启动配置公共类型，不直接承载配置实体或校验实现。

- `shared/src/config.rs`
  - 定义 `AppConfig`、`ServerConfig`、`StorageConfig` 和 `RuntimeMode`。
  - 使用 `garde` 定义 JSON 启动配置约束，并提供解析和序列化辅助函数。
  - `AppConfig::from_json_str()` 在反序列化后执行校验。
  - `ServerConfig` 不包含独立 `enabled` 开关；是否使用本地服务由运行模式决定。

- `shared/src/error.rs`
  - 定义 `ConfigParseError`，区分 JSON 结构错误和字段约束错误。

- `shared/src/config_validation.rs`
  - 定义配置实体内部使用的 `garde` 自定义校验函数。
  - 只保存内置规则无法直接表达的项目语义。

- `shared/src/text_validation.rs`
  - 定义无业务语义的基础文本校验函数。
  - 提供 trim 后非空和可选字符串非空规则，供 shared、core DTO 和 repository 输入复用。

## 测试与约束文档

- `shared/src/tests/lib.rs`：shared 配置和基础校验测试入口。
- `docs/validation/shared-src-config.md`：启动配置和运行模式限制。
- `docs/validation/shared-src-error.md`：配置解析错误。
- `docs/validation/shared-src-config-validation.md`：配置自定义校验边界。
- `docs/validation/shared-src-text-validation.md`：基础文本校验边界。
