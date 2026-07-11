# Server Shell 文档

本目录作为 `winestock-server` 无头平台 shell 的文档入口。
server shell 负责进程生命周期、固定配置位置、存储目录准备、服务启动、访问地址输出和优雅关闭，不拥有 core 业务规则或前端资源。

当前详细规则集中在以下项目级和代码地图文档中：

- [`../../docs/runtime-networking.md`](../../docs/runtime-networking.md)：server mode、绑定地址和访问 URL。
- [`../../docs/platforms.md`](../../docs/platforms.md)：server shell 平台职责。
- [`../../docs/code-map/server.md`](../../docs/code-map/server.md)：当前源码结构和启动流程。

后续新增仅属于 server shell 的部署、配置或运维说明时，应放在本目录，而不是堆入根 `docs/`。
