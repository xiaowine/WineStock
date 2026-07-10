# Core 库存代码地图

`core/src/stock` 是库存业务域，拥有物品、模板、库位、出入库、审批、看板、替代料和事件日志 HTTP/服务逻辑。

## 路由和权限

- `core/src/stock/mod.rs`
  - 以 `/api` 为 base path 注册各库存子域路由。
  - 在路由装配阶段声明物品、库位、模板、入库、出库、看板、替代料和审计权限。
- `core/src/stock/permissions.rs`
  - 定义库存和审计稳定权限代码，以及历史兼容 `stock.read`、`stock.write`。

## Controller

- `controller.rs`：库存 HTTP 控制器入口和子模块重新导出。
- `controller/common.rs`：单据状态、筛选值 DTO 和共享正数校验。
- `controller/items.rs`：物品 DTO、分页、搜索、筛选值、详情和 CRUD handler。
- `controller/templates.rs`：模板与模板字段 DTO、CRUD/copy handler。
- `controller/locations.rs`：库位分组树、库位、整批次移库 DTO 和 handler。
- `controller/inbound.rs`：入库单、分页、筛选值、详情和 handler。
- `controller/outbound.rs`：出库单、搜索、筛选值、详情和 handler。
- `controller/dashboard.rs`：库存总览和趋势。
- `controller/substitutes.rs`：替代料整体替换、查询和删除。
- `controller/events.rs`：审计事件分页查询。

## Service

- `service.rs`：库存服务入口和子模块重新导出。
- `service/items.rs`：物品 CRUD、分页、搜索、筛选值、库存快照、SKU 冲突和审计。
- `service/templates.rs`：模板 CRUD/copy、字段组合校验和审计。
- `service/locations.rs`：分组树、库位 CRUD、移库、循环和占用校验。
- `service/inbound.rs`：入库创建、查询、审批、拒绝和模板扩展属性校验。
- `service/outbound.rs`：出库创建、搜索、审批、拒绝和库存不足映射。
- `service/dashboard.rs`：库存总览、趋势和呆滞料查询。
- `service/substitutes.rs`：替代料关系、自引用/重复/循环校验和审计。
- `service/events.rs`：审计事件筛选与分页。
- `service/error.rs`：`StockApiError` 和 repository 错误收敛。
- `service/pagination.rs`：库存通用分页结构。
- `service/response.rs`：repository 记录到 HTTP DTO 的投影。
- `service/validation.rs`：库存文本、数值、ID 和 JSON 归一化。

## 启动补齐

- `core/src/stock/bootstrap.rs`
  - 补齐 `元器件`、`3D打印耗材`、`通用` 三个内置模板。
  - 只在没有任何有效库位时创建 `默认库区`/`DEFAULT`。
  - 不覆盖用户修改，不恢复被软删除的模板。
