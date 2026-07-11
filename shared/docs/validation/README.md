# Shared 字段限制文档

本目录记录 `shared/src` 中运行配置、配置文件加载和基础文本辅助规则的约束来源。

文件按对应源码文件命名，路径分隔符用 `-` 展开。例如 `shared/src/config.rs` 对应 `shared-src-config.md`。

JSON 启动配置通过 `AppConfig::from_json_str()` 反序列化并立即执行 `garde` 校验；配置文件位置、默认值和相对路径解析仍由调用它的平台 shell 决定。
