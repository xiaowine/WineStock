# Core 库存代码地图

`core/src/stock` 是库存业务域，拥有物品、模板、库位、出入库、审批、看板、替代料和事件日志 HTTP/服务逻辑。

## 路由和权限

- `core/src/stock/mod.rs`
  - 以 `/api` 为 base path 注册各库存子域路由。
  - 在路由装配阶段声明物品、库位、模板、入库、出库、看板、替代料和审计权限。
  - 入库模板两个读取接口按 stock.inbound.create 或 stock.template.read 任一权限放行，写接口仍要求 stock.template.manage。
- `core/src/stock/permissions.rs`
  - 定义库存和审计稳定权限代码，以及历史兼容 `stock.read`、`stock.write`。

## Controller

- `controller.rs`：库存 HTTP 控制器入口和子模块重新导出。
- `controller/common.rs`：单据状态、筛选值 DTO 和共享正数校验。
- `controller/items.rs`、`item_attributes.rs`：物品命令、目录查询、结构化筛选参数、轻量选择、编辑资料、库存详情、批次分页和任意类型化属性。
  - 轻量选择响应额外返回推荐入库模板 ID 和其当前是否可用，供入库工作台直接判断推荐状态。
- `controller/templates/`：分类、物品属性模板、入库模板和共享字段 DTO/handler；分类和物品模板响应公开当前有效物品使用数，删除响应公开事务内影响数；物品模板字段额外公开显式单位规则 DTO。
- `controller/locations.rs`：库位分组树、名称唯一且可带备注的库位、整批次移库 DTO 和 handler。
- `controller/inbound.rs`：入库提交模式、单据响应、分页、筛选值、详情和 handler。
- `controller/outbound.rs`：出库单、搜索、筛选值、详情和 handler。
- `controller/dashboard.rs`：库存总览和趋势。
- `controller/substitutes.rs`：替代料整体替换、查询和删除；指定物品查询响应包含替代物品第一层资料与库存摘要。
- `controller/events.rs`：审计事件分页查询。

## Service

- `service.rs`：库存服务入口和子模块重新导出。
- `service/items.rs`、`item_attributes.rs`：物品必选主图、命令与查询分离、库存状态规则、结构化筛选解析与定义校验、任意属性校验、模板单位派生/候选校验和文件所有权。
- `service/templates/`：分类与两类模板的独立 CRUD/copy、当前有效物品使用数投影、删除影响数、共享字段规则，以及物品模板专属单位规则归一化。
- `service/locations.rs`：分组树、最多十层的创建/子树移动、库位 CRUD、名称唯一、移库、循环和占用校验。
- `service/inbound.rs`：独立入库模板推导/选择、实际入库属性校验、图片引用、直接入库权限判断、审批和错误定位。
- `service/outbound.rs`：出库创建、搜索、审批、拒绝和库存不足映射。
- `service/dashboard.rs`：库存总览、趋势和呆滞料查询。
- `service/substitutes.rs`：替代料关系、自引用/重复/循环校验和审计。
- `service/events.rs`：审计事件筛选与分页。
- `service/error.rs`：`StockApiError` 和 repository 错误收敛。
- `service/pagination.rs`：库存通用分页结构。
- `service/response.rs`：repository 记录到 HTTP DTO 的投影。
  - 轻量选择投影包含推荐入库模板 ID 与可用状态；软删除入库模板仍返回原始 ID 并标记不可用。
- `service/validation.rs`：库存文本、数值、ID 和 JSON 归一化。

## 启动补齐

- `core/src/stock/bootstrap/`
  - 分别补齐三个分类、三套物品属性预设和三套入库模板。
  - 只在没有任何有效库位时创建 `默认库区`/`默认库位`。
  - 不覆盖用户修改，不恢复被软删除的模板。
