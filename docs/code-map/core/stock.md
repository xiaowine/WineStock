# Core 库存代码地图

`core/src/stock` 是库存业务域，拥有物品、模板、库位、出入库、审批、看板、替代料和事件日志的 HTTP 与服务逻辑。
逐文件职责以源码中文文件头注释为准；本地图记录子域所有权、横切模块和权限边界。

## 路由与权限

- `mod.rs`：以 `/api` 为 base path 注册各子域路由，并在装配阶段声明权限；入库模板两个读取接口按 stock.inbound.create 或 stock.template.read 任一权限放行，写接口仍要求 stock.template.manage。
- `permissions.rs`：库存与审计稳定权限代码，以及历史兼容 `stock.read`、`stock.write`。

## 业务子域

每个子域由 `controller/` 的 DTO/handler/OpenAPI 标注与 `service/` 的业务规则成对构成；controller 不写业务规则，service 不拥有 HTTP 细节。

- 物品（`items`、`item_attributes`）：命令与查询分离、必选主图、目录结构化筛选解析与定义校验、轻量选择（含推荐入库模板 ID 与可用状态）、库存详情、批次分页和任意类型化属性校验。
- 立创查询（`item_lookup`）：客编校验、精确结果选择、已知字段投影、在售/库存/第一阶梯价判断、受控商品图片签名复核和稳定外部错误映射；不写数据库，不公开上游原始响应或图片源地址。
- 模板（`templates/`）：分类与两类模板的独立 CRUD/复制、当前有效物品使用数投影、删除影响数和物品模板专属单位规则归一化。
- 库位（`locations`）：最多十层分组树的创建/子树移动、名称唯一且可带备注的库位 CRUD、整批次移库与循环/占用校验。
- 入库（`inbound`）：提交模式、独立入库模板推导/选择、实际属性与图片引用校验、直接入库权限判断、审批与错误定位。
- 出库（`outbound`）：创建、状态/搜索分页、审批、拒绝与库存不足映射。
- 看板（`dashboard`）：库存总览、趋势和呆滞料查询。
- 替代料（`substitutes`）：整体替换、查询与删除，含自引用/重复/循环校验和审计；指定物品查询附带第一层资料与库存摘要。
- 审计（`events`）：事件筛选与分页查询。

## 横切模块

- `controller/common.rs`：单据状态、筛选值 DTO 和共享正数校验。
- `service/error.rs`：`StockApiError` 与 repository 错误收敛。
- `service/pagination.rs`、`response.rs`、`validation.rs`：库存通用分页、repository 记录到 HTTP DTO 的投影（软删除入库模板保留原始 ID 并标记不可用）和文本/数值/ID/JSON 归一化。

## 启动补齐

- `bootstrap/`：幂等补齐三个分类、三套物品属性预设和三套入库模板；只在没有任何有效库位时创建 `默认库区`/`默认库位`，不覆盖用户修改，不恢复被软删除的模板。
