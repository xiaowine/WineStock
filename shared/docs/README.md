# Shared 文档

本目录记录 `winestock-shared` 拥有的平台无关运行配置、JSON 配置文件加载和基础文本校验规则。
它不描述任何平台 shell 的配置保存位置、存储路径策略、生命周期或 core HTTP 业务契约。

- [`validation/`](validation/)：共享配置实体、配置文件加载、错误类型和基础文本校验说明。
- [`../../docs/runtime-networking.md`](../../docs/runtime-networking.md)：跨平台运行模式和网络字段语义。
- [`../../docs/project-structure.md`](../../docs/project-structure.md)：shared 与 core、平台 shell 的依赖边界。
