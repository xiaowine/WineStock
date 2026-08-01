# 立创商品资料查询与新建物品回填实施方案

> 历史方案：其中 EDA 双接口与 Core 图片代理已由
> [`lcsc-phone-query-and-direct-image.md`](lcsc-phone-query-and-direct-image.md) 取代；本文只保留首版交互与字段
> 回填决策记录，不再作为当前上游协议依据。

## 1. 状态与目标

本文是 WineStock 在物品管理的新建会话中，根据单个立创商城商品编号查询电子元器件资料并回填草稿的实施方案。

方案已经确认以下产品约束：

- 功能入口位于物品管理现有“新建物品” Dialog 内，不新增一级导航、目录工具栏入口或独立导入页面。
- 用户输入一个立创商品编号；格式为大写 `C` 开头，后续全部为数字，例如 `C2983288`。
- 一次只查询一个编号。
- WineStock 前端不能直接读取立创接口响应，因为正式 POST 响应缺少浏览器需要的
  `Access-Control-Allow-Origin`；查询必须由 `core` 转发。
- 立创上游请求地址固定为 `https://pro.lceda.cn/api/devices/search`，明确不使用 `searchByCodes`。
- 上游请求中的 `path` 和 `uid` 固定为 `0819f05c4eef4c71ace90d822a990e87`；`page = 1`、
  `pageSize = 50`、`tag = []`、`attributes = {}` 固定不变，只有 `wd` 使用规范化后的单个客编。
- 价格请求固定使用 `https://pro.lceda.cn/api/components/getSmtPartInfo`，`path` 相同，`numbers` 始终只含
  当前规范化客编。价格取可售且有库存记录中 `startNumber` 最小的有效阶梯价；价格失败或不可售不阻断资料查询。
- 查询成功后先提示用户是否填写；用户选择“覆盖填写”时，候选资料直接覆盖当前草稿中的对应字段，
  不进行逐字段冲突选择；选择“不填写”时草稿完全不变。
- 确认覆盖后，前端通过 Core 受控图片读取接口取得首张商品图片，作为普通待上传图片草稿覆盖当前主图；
  图片缺失或失败不阻断其它资料回填。
- 查询和回填不创建物品、不写库存、不立即上传文件；用户仍需在外层 Dialog 检查资料并点击“保存物品”。

本方案覆盖 `core` HTTP 适配、前端查询 Dialog、草稿覆盖、错误处理、测试和文档同步。正式实施前不改变数据库结构。

## 2. 非目标

首版不实现：

- 多编号或文件批量导入；
- 编辑已有物品时从立创覆盖资料；
- 自动创建或修改物品分类和模板字段；属性模板只允许从现有模板中选择；
- 根据立创分类自动选择 WineStock 分类；
- 导入库存数量、批次、库位、补货点或采购信息；
- 下载封装、符号或 3D 模型文件；商品首图属于本方案范围；
- 保存立创原始响应或增加第三方资料缓存表；
- 绕过现有物品表单校验、主图上传和 `POST /api/items` 创建事务；
- 将上游 URL、固定 `path` 或任意目标地址开放给前端配置。

## 3. 当前基线

### 3.1 前端

- `ItemsPage.vue` 的加号按钮直接创建 `create` 模式草稿并打开 `ItemEditorDialog`。
- `ItemEditorDialog.vue` 在 `create` 模式只显示物品资料，不挂载库存或替代关系工作区。
- `ItemEditorDialog` 的 `modal-actions` 当前包含“取消”和“保存物品”。
- `ItemDraft` 由 `ItemsPage` 持有；`ItemEditorDialog` 和 `ItemEditor` 编辑同一草稿对象。
- 新建草稿默认计量单位为“个”；确认覆盖后可以用立创首张商品图替换主图草稿。
- 物品属性支持模板字段和物品私有自定义字段；模板字段类型与单位规则不能由查询结果改写。

### 3.2 Core

- 物品路由属于 `core/src/stock`，`stock.item.manage` 控制物品创建、更新和删除。
- `CoreState` 当前持有存储与安全运行时，所有库存 handler 通过 Axum `State<CoreState>` 取依赖。
- `StockApiError` 统一把库存领域错误映射为稳定 HTTP 状态和错误码。
- 工作区尚未引入服务端 HTTP client 依赖。
- 查询不需要 repository，不应进入物品创建事务或审计事件。

### 3.3 已验证的上游行为

不携带 Cookie 的服务端 POST 可以返回 `200 application/json`。示例结果包含：

- `product_code`；
- `attributes["LCSC Part Name"]`；
- `attributes["Supplier Part"]`；
- `attributes["Manufacturer"]`；
- `attributes["Manufacturer Part"]`；
- `attributes["Supplier Footprint"]`；
- `attributes["Datasheet"]`；
- 其它随商品类别变化的参数；
- EasyEDA 专用的 Symbol、Footprint 和 3D Model 标识。

该接口没有已纳入项目的正式公开契约。实现必须把它视为不稳定上游，并隔离原始 DTO、字段缺失和结构变化。

## 4. 目标用户流程

```text
物品管理
  -> 点击加号
  -> 打开“新建物品” Dialog
  -> 点击 modal-actions 左侧“查询立创资料”
  -> 打开嵌套“查询立创资料” Dialog
  -> 输入 C2983288
  -> core 查询立创接口
  -> 查询成功后显示命中商品摘要、属性模板选择和覆盖提示
     -> 不填写：关闭嵌套 Dialog，外层草稿不变
     -> 覆盖填写：覆盖当前草稿，关闭嵌套 Dialog
  -> 用户继续检查、补充或修改外层表单
  -> 用户点击“保存物品”
  -> 继续走现有 POST /api/items
```

外层 Dialog 在整个查询、确认和关闭动画期间保持挂载。嵌套 Dialog 不直接持有或修改 `ItemDraft`，只在用户确认
“覆盖填写”后向外层发出一次候选资料事件。

## 5. 跨组件边界

```text
ItemEditorDialog
  -> frontend/src/api/items.ts
  -> GET /api/items/lookups/lcsc/{product_code}
  -> stock controller
  -> item lookup service
  -> LcscLookupClient
  -> 并发 POST /api/devices/search 与 /api/components/getSmtPartInfo
  <- 上游原始 JSON
  <- WineStock 归一化候选 DTO
  <- 用户确认
  -> 覆盖 ItemDraft
  -> GET /api/items/lookups/lcsc/{product_code}/image
  -> Core 校验并代理立创首张商品图
  -> 转换为普通待上传图片草稿
  -> 现有 POST /api/items
```

职责固定如下：

| 层                 | 负责                                         | 不负责                      |
| ------------------ | -------------------------------------------- | --------------------------- |
| 前端查询 Dialog    | 输入、等待、错误、结果确认                   | 直接请求立创、创建物品      |
| 前端草稿映射       | 将候选资料覆盖到 `ItemDraft`                 | 信任候选值绕过表单校验      |
| Core controller    | WineStock DTO、OpenAPI、handler              | 解析上游任意字段            |
| Core service       | 编号归一化、结果选择、候选资料投影、错误归类 | 数据库写入                  |
| `LcscLookupClient` | 上游 HTTP、固定请求、原始 DTO、响应大小限制  | WineStock UI 或物品创建规则 |
| repository         | 无职责                                       | 保存查询结果或参与查询      |

## 6. WineStock HTTP 契约

### 6.1 请求

```http
GET /api/items/lookups/lcsc/C2983288
Authorization: Bearer <access-token>
```

采用 GET 是因为 WineStock 侧查询没有副作用；`core` 内部仍按上游要求发送 POST。路径中的商品编号必须：

1. 裁剪首尾空白；
2. 统一转为大写；
3. 完整匹配 `^C[0-9]+$`；
4. 总长度不超过 32 个字符。

路由使用 `stock.item.manage`，因为入口只存在于有权创建物品的新建 Dialog。`stock.item.read` 不单独获得外部查询能力。

### 6.2 成功响应

```json
{
  "source": "lcsc",
  "product_code": "C2983288",
  "name": "BER-04",
  "description": "旋转编码开关/8421开关/BCD编码开关",
  "manufacturer": "SM Switch",
  "manufacturer_part": "BER-04",
  "footprint": "插件",
  "datasheet_url": "https://item.szlcsc.com/datasheet/BER-04/3419457.html",
  "default_price": 9.91,
  "parameters": [
    { "name": "Number of Coded Gears", "value": "4位" },
    { "name": "Coded Form", "value": "正码" },
    { "name": "Operating Temperature", "value": "-40℃~+85℃" }
  ]
}
```

响应字段规则：

- `source` 固定为 `lcsc`。
- `product_code` 使用命中结果的规范化商品编号。
- `name` 优先使用 `Manufacturer Part`，其次使用 `LCSC Part Name`，最后回退为商品编号。
- `description` 优先使用搜索结果顶层 `description`，其次使用 `LCSC Part Name`；均不存在时为 `null`。
- `manufacturer`、`manufacturer_part`、`footprint`、`datasheet_url` 均允许 `null`。
- `default_price` 只在价格记录客编精确匹配、`onSale = 1`、`stock_num > 0` 且存在有效正数阶梯价时返回；
  取 `startNumber` 最小档的 `productPrice`，其它情况为 `null`。
- `parameters` 只包含值为有限长度字符串、数字或布尔值的业务参数；系统字段和资源内部标识必须排除。
- 不返回上游 `uuid`、Symbol、Footprint UUID、3D Model UUID、transform、BOM/PCB 开关或会话信息。
- 资料响应不包含上游图片 URL；图片只能通过 WineStock 的受控图片接口读取。
- 所有字符串裁剪首尾空白；单字段最长 1024 字符，参数名最长 128 字符，参数最多 64 项。
- 无法确认为 HTTP/HTTPS 的数据手册地址返回 `null`。

### 6.3 受控图片响应

用户确认“覆盖填写”后，前端再请求：

```http
GET /api/items/lookups/lcsc/C2983288/image
Authorization: Bearer <access-token>
```

Core 重复执行单客编精确资料查询，只使用命中记录的首张图片。图片源必须是
`https://alimg.szlcsc.com/...`，禁止重定向。搜索接口返回 `/upload/public/product/middle/` 路径时，Core
优先把受控路径转换为同日期、同文件名的 `/upload/public/product/source/` 高清版本；高清版本不可用时回退搜索结果
原地址，无法识别的其它路径不做推导。Core 对每个实际下载候选都校验成功状态、15 MiB 上限、声明 MIME 与
PNG/JPEG/WebP 文件签名一致，再返回图片字节和实际 `Content-Type`。

该接口不返回第三方 URL，不创建 WineStock 文件对象，也不写数据库。前端把响应 Blob 转为普通 `File` 和
`ImageDraftValue`；只有用户最终保存物品时，才沿用 `POST /api/files/images` 上传并取得 `image_file_id`。

图片不存在返回 404；下载超时、上游失败、大小超限、MIME 或签名无效按统一立创错误返回。图片读取失败只显示
非阻断提示并保留当前主图，已经覆盖的资料、模板属性和参考价格不回滚。

### 6.4 错误响应

继续使用项目统一 `ApiErrorResponse`：

| 场景                          | HTTP | code                        | 前端文案方向                        |
| ----------------------------- | ---: | --------------------------- | ----------------------------------- |
| 编号格式无效                  |  400 | `invalid_lcsc_product_code` | 请输入 C 开头、后续为数字的商品编号 |
| 上游成功但没有精确匹配项      |  404 | `lcsc_product_not_found`    | 未查询到该立创商品                  |
| 等待可用请求槽超限            |  429 | `lcsc_lookup_busy`          | 查询繁忙，请稍后重试                |
| 连接或完整请求超时            |  504 | `lcsc_lookup_timeout`       | 立创服务响应超时                    |
| 上游非 2xx 或连接失败         |  502 | `lcsc_lookup_failed`        | 暂时无法查询立创资料                |
| 响应超限、JSON 损坏或结构不符 |  502 | `lcsc_invalid_response`     | 立创返回了无法识别的数据            |

错误响应不得包含 Reqwest 调试文本、上游响应体、内部 URL、请求头、Cookie 或立创会话信息。

### 6.5 OpenAPI

新增 handler 必须加入 Debug OpenAPI：

- 资料与图片两条路径、路径参数和 bearer auth；
- `200/400/401/403/404/429/502/504` 响应；
- 候选资料与参数 DTO schema，以及图片成功响应的二进制内容类型；
- `items` tag。

`http_openapi.rs` 增加路径、权限响应和 schema 断言。

## 7. 立创上游适配

### 7.1 固定请求

生产实现固定：

```rust
const LCEDA_SEARCH_URL: &str = "https://pro.lceda.cn/api/devices/search";
const LCEDA_SEARCH_PATH: &str = "0819f05c4eef4c71ace90d822a990e87";
```

原始请求 DTO 固定搜索上下文，只把单个规范化客编放入 `wd`：

```rust
struct LcedaSearchRequest<'a> {
    attributes: std::collections::HashMap<&'static str, &'static str>,
    page: u8,
    #[serde(rename = "pageSize")]
    page_size: u8,
    path: &'static str,
    tag: [&'static str; 0],
    uid: &'static str,
    wd: &'a str,
}
```

生产请求中 `attributes` 必须序列化为 `{}`，其余字段值固定为 `page = 1`、`pageSize = 50`、
`tag = []`、`path = uid = LCEDA_SEARCH_PATH`。请求体不得包含 `codes`。

只设置 `Accept: application/json` 和由 `.json()` 生成的 `Content-Type`。明确禁止：

- Cookie store 和手工 Cookie；
- 浏览器 `Origin`、`Referer`、`Sec-Fetch-*`；
- 从用户请求复制任意上游 header；
- 接受前端提供的 URL 或 `path`；
- 自动跟随重定向到其它主机。

### 7.2 Reqwest client

在工作区根声明 `reqwest`，`core` 通过 `workspace = true` 使用。实施时选择当前兼容稳定版本，并最小化 feature：

- 关闭 default features；
- 启用 `json` 和 Rustls TLS；
- 不启用 cookies；
- HTTPS only；
- redirect policy 为 `none`；
- connect timeout 为 3 秒；
- 单次完整请求 timeout 为 8 秒；
- 每主机空闲连接池上限为 2；
- 不设置上游 JSON 响应体大小上限；仍受完整请求超时约束。

Reqwest `Client` 是可克隆的连接池句柄，应构建一次并随 `CoreState` 共享，不能在 handler 内重复创建。Axum 0.8
通过 `State` 注入共享依赖，handler 返回实现 `IntoResponse` 的具体领域错误。

### 7.3 运行时归属

新增 `ExternalCatalogRuntime` 或等价窄对象，由 core bootstrap 构建并放入 `LocalServiceBootstrap`，随后复制到
`CoreState`。构建 HTTP client 是本地初始化，不探测立创网络可用性；WineStock 启动不能依赖立创在线。

如果 client 本地构建失败，bootstrap 返回新的明确错误变体。平台 shell 继续只负责启动同一 core，不持有第三方配置。

### 7.4 并发边界

`LcscLookupClient` 内使用共享 `Semaphore`，首版最多同时执行 4 次上游查询：

- 超过四个并发请求时立即返回 `lcsc_lookup_busy`，不让大量请求在进程内排队；
- 不实现永久缓存或数据库缓存；
- 用户关闭 Dialog 后前端取消 WineStock 请求，但 core 已发出的上游请求只做尽力取消；取消不能写任何状态。

### 7.5 响应读取

处理顺序固定：

1. 检查 HTTP 状态，非 2xx 不读取并透传错误页；
2. 逐 chunk 读取完整响应；
3. 反序列化到私有上游 DTO；
4. 要求 `success == true` 且 `code == 0`；
5. 稳定展开 `result.lists` 中的记录，优先处理 `lists.lcsc`，其它分组按名称稳定展开；
6. 以顶层 `product_code` 为主、`attributes["Supplier Part"]` 为回退，查找与规范化请求值完全相同的唯一结果；
7. 将已知字段投影到 WineStock DTO，描述优先使用顶层 `description`，再回退 `LCSC Part Name`；
8. 过滤系统字段后生成附加参数列表。

上游结果包含多个不相关商品时不使用第一项；出现两个按上述规则命中同一客编的记录时视为无效响应，避免非确定性覆盖。

## 8. Core 模块与文件改动

建议文件所有权：

```text
core/src/
├─ external/
│  ├─ mod.rs
│  └─ lcsc.rs                    # Reqwest client、固定请求和上游私有 DTO
├─ stock/
│  ├─ controller/
│  │  └─ item_lookup.rs          # WineStock HTTP DTO、OpenAPI 和 handler
│  └─ service/
│     └─ item_lookup.rs          # 校验、精确匹配、归一化和错误转换
├─ bootstrap.rs                  # 构建外部 catalog runtime
└─ state.rs                      # 向业务层公开窄 lookup client getter
```

同时修改：

- 根 `Cargo.toml` 与 `core/Cargo.toml`：新增 Reqwest 依赖；
- `stock/mod.rs`：注册 `/api/items/lookups/lcsc/{product_code}`，使用 `item_manage`；
- `stock/controller.rs`、`stock/service.rs`：声明并导出新模块；
- `stock/service/error.rs`：增加立创查询错误变体与稳定 HTTP 映射；
- `http/docs.rs`：注册 OpenAPI handler/schema；
- `core/docs/business-api/items.md`：记录查询契约与无写入边界；
- `docs/code-map/core/stock.md`：更新 controller/service 所有权；
- 如新增通用 `external/` 层，同步新增或更新 core 代码地图入口。

不要把上游 client 放入 repository，也不要把上游 DTO放入 `controller/items.rs` 的物品创建 DTO中。

## 9. 前端交互设计

### 9.1 外层操作区

仅在 `ItemEditorDialog` 满足以下条件时显示“查询立创资料”：

- `mode === "create"`；
- `readOnly === false`；
- 外层没有保存；
- 当前用户已经通过页面权限获得创建入口。

操作区桌面布局：

```text
[查询立创资料]                              [取消] [保存物品]
```

“查询立创资料”使用 `secondary-button`，通过组件自有 class 设置 `margin-right: auto`；不得修改所有
`ModalDialog` 的操作区布局。移动端允许三按钮保持一行；只有实测无法容纳时，查询按钮单独占第一行，右侧主操作保持稳定。

保存期间按钮禁用。查询 Dialog 打开时外层表单保持挂载并通过最上层 Modal 阻止交互。

### 9.2 查询 Dialog

新增 `LcscItemLookupDialog.vue`，组合现有 `ModalDialog`：

- `compact`；
- `nested`；
- 标题“查询立创资料”；
- 描述“输入立创商城 C 开头的商品编号”；
- 一个可见 label 的文本输入框；
- 默认空值，不从外层 SKU 静默触发网络请求；
- 若外层 SKU 已匹配 `^C[0-9]+$`，打开时可将其作为输入初值；
- Enter 提交查询；
- 查询中关闭按钮和输入仍可保留，但不得重复提交；
- 请求完成后焦点移动到结果摘要或错误恢复入口；
- 关闭后焦点返回外层“查询立创资料”按钮。

查询成功后，同一个 Dialog 从输入态切换为确认态，不再叠加第三层确认 Dialog：

```text
已查询到 BER-04
立创商品编号 C2983288 · SM Switch

是否使用查询结果填写当前表单？
查询结果中的有效字段将覆盖当前内容。

属性模板 [电子元器件 v]

[不填写] [覆盖填写]
```

模板选择默认使用当前模板列表第一项，也允许改选其它模板或“不使用模板”。“不填写”关闭嵌套 Dialog且不发出
候选资料；“覆盖填写”发出一次包含候选资料和模板 ID 的 `apply` 事件后关闭。按钮不用“否/是”。

### 9.3 状态机

```text
closed
  -> input
input
  -> loading
loading
  -> confirm | error | input（主动取消）
confirm
  -> closed（不填写）
  -> closed + emit apply（覆盖填写）
error
  -> loading（重试）
  -> input（修改编号）
  -> closed
```

每次打开创建独立会话：

- 清除旧错误和旧候选结果；
- 使用新的 `AbortController`；
- 关闭或外层卸载时中止请求；
- 用请求 generation 防止旧响应覆盖新查询；
- `AbortError` 不显示错误 Notice；
- 查询错误保留输入值，允许原地重试。

### 9.4 前端 API

在 `frontend/src/api/items.ts` 增加：

- `LcscItemLookupResponse`；
- `LcscItemLookupParameterResponse`；
- `lookupLcscItem(productCode, signal)`。

URL path segment 必须使用 `encodeURIComponent`。前端先做相同格式校验改善反馈，但以 core 校验为最终边界。

### 9.5 草稿覆盖规则

新增纯函数 `applyLcscLookupToDraft(draft, lookup, template)`，只在用户确认后调用。先应用所选模板，再同步覆盖候选资料，
以便指纹和未保存状态立即更新。

基础字段：

| 候选字段       | 草稿字段      | 行为       |
| -------------- | ------------- | ---------- |
| `product_code` | `sku`         | 始终覆盖   |
| `name`         | `name`        | 非空时覆盖 |
| `description`  | `description` | 非空时覆盖 |

查询结果不修改：

- `categoryId`；
- `unit`，保留新建默认“个”或用户当前值；
- `reorderPoint`。

候选 `default_price` 为有效正数时覆盖 `defaultPrice`；为 `null` 时保留当前值。可售条件必须同时满足
精确客编、`onSale = 1`、`stock_num > 0` 和有效阶梯价，不能仅使用可能残留的 `maxPrice` 或历史 `priceList`。

图片覆盖规则：

1. 只使用精确命中器件响应中的首张图片，不接受前端提供任意第三方 URL；
2. Core 只允许固定立创图片主机的 HTTPS 地址；对受控 `middle` 路径优先尝试同文件的 `source` 高清版本，
   不可用时回退原地址，且禁止重定向并校验响应上限、MIME 与 PNG/JPEG/WebP 文件签名；
3. 前端读取 Blob 后创建普通 `File`/图片草稿，释放被替换的本地临时预览；
4. 图片不在查询确认阶段上传，最终保存时继续走现有 `POST /api/files/images`；
5. 无图片、404、超时、上游失败或校验失败时保留当前主图并显示非阻断提示，不回滚已应用的资料和价格。

属性候选映射：

| 候选字段            | WineStock 属性名 | 类型                        |
| ------------------- | ---------------- | --------------------------- |
| `manufacturer_part` | `型号`           | text                        |
| `manufacturer`      | `品牌`           | text                        |
| `footprint`         | `封装`           | text                        |
| `datasheet_url`     | `数据手册`       | url                         |
| `parameters`        | `参数`           | text，格式为每行 `名称：值` |

处理算法：

1. 只处理非空候选值；
2. 在当前 `draft.attributes` 中按裁剪后、不区分大小写的属性名查找；
3. 命中 text 字段时将候选值转换为字符串并直接覆盖；
4. “数据手册”只覆盖 url 或 text 字段，值必须是有效 HTTP/HTTPS URL；
5. 命中自定义字段且类型不兼容时，把该自定义字段改成目标类型并清理不适用的 options/unit；
6. 命中模板字段且类型不兼容时不改模板定义，将可安全表示的值写为模板类型；无法安全表示时跳过该字段；
7. 没有同名字段时新增物品私有自定义属性；
8. 同名属性出现多次属于异常草稿，只覆盖第一项并沿用现有表单唯一性校验暴露问题；
9. 模板 ID 必须来自当前已加载模板列表；确认态默认选择列表第一项，不根据展示名称推断实体；
10. 应用完成后显示信息型 Notice，例如“已使用 C2983288 填写物品资料”。

用户要求的“无论是否冲突都覆盖”特指候选值与当前字段值冲突时不做逐字段选择；它不能绕过模板字段类型、URL
合法性和本地表单结构约束。未返回、空字符串或被过滤的上游字段不清空草稿原值。

### 9.6 未保存状态

查询输入和查询结果不进入 `ItemDraft`，因此单纯查询或选择“不填写”不应触发外层未保存状态。

确认“覆盖填写”后：

- `itemDraftFingerprint` 自然发生变化；
- 关闭外层 Dialog 时沿用现有放弃修改确认；
- 不更新 `baselineDraft`；
- 不自动提交或保存。

### 9.7 原生返回和焦点

- 查询 Dialog 使用 `nested` Modal，沿用优先级 400；
- Android 返回只关闭最上层查询 Dialog，不关闭外层新建 Dialog；
- 查询中 Modal 的 busy 行为必须明确：若允许关闭则先同步取消请求再开始离场；若标记 busy，则原生返回只消费不关闭；
  首版选择允许关闭并取消请求，避免外部服务超时时困住用户；
- 查询 Dialog 离场完成后焦点回到“查询立创资料”；
- 应用资料后焦点同样回到触发按钮，不自动跳到被覆盖字段。

## 10. 安全与稳定性

### 10.1 SSRF 与请求污染

- endpoint、scheme、host、path 全部由代码常量拥有；
- 前端只提供规范化商品编号；
- 商品编号不能插入 URL、header 或上游 `path`；
- 禁止自动重定向；
- 不记录 access token、第三方响应体或请求 header。

### 10.2 Cookie 与会话

- Reqwest 不启用 cookie store；
- 不接受或转发浏览器 Cookie；
- 上游 `Set-Cookie` 可以出现在响应中，但 client 不保存也不回传；
- 测试夹具不得包含真实站点 Cookie。

### 10.3 数据可信度

- 立创资料是候选输入，不是 WineStock 已验证业务实体；
- 最终创建仍由现有 `ItemCreateRequest` 校验 SKU、主图、单位、模板和属性；
- core lookup 只做结构、长度和 URL 安全校验，不声称第三方资料准确；
- 不把立创内部 UUID 当成可公开业务标识。

### 10.4 平台与网络

- self-hosted 模式由本机 core 对外访问立创；
- remote/client-only 模式由远端 WineStock server 对外访问立创；
- 前端所在浏览器或 WebView 不直接连接立创，因此不新增 WebView origin、CORS 或 Shell Bridge 权限；
- Android 已有网络权限继续覆盖本地 core 的 HTTPS 请求，不新增 native 业务桥；
- 查询失败不触发全局“服务不可用”覆盖层，因为 WineStock core 本身仍可用。

## 11. 测试方案

### 11.1 Core 纯函数测试

- 小写输入规范化为大写；
- 合法边界与非法编号；
- 按顶层 `product_code` 或 `attributes["Supplier Part"]` 精确选择匹配记录；
- 空结果、没有客编的结果、多个相似结果和不相关结果；
- 名称与描述回退顺序；
- 已知字段映射；
- 系统字段过滤；
- 标量参数保留，数组/对象参数丢弃；
- 字段长度、参数数量和 URL 校验；
- 不把 `Name = "={Manufacturer Part}"` 当作最终名称。
- 价格按最小 `startNumber` 选择，不按数组顺序或最低批量价选择；
- `C2983288` 可售且有库存时返回第一阶梯价，`C2982` 零库存和 `C9900201662` 空价格结果均不返回价格。

### 11.2 Core HTTP client 测试

使用本地 mock HTTP server 和仅测试可用的 endpoint 构造入口，不请求真实立创服务。验证：

- 请求方法是 POST；
- body 中 `wd` 严格等于规范化输入，且不存在 `codes`；
- `path` 和 `uid` 等于固定值；
- `page = 1`、`pageSize = 50`、`tag = []`、`attributes = {}`；
- 价格 body 的 `numbers` 只含规范化客编，`path` 固定；
- 价格接口失败、空结果、零库存、非在售和无有效阶梯价；
- 图片地址主机/协议、`middle -> source` 候选顺序与回退、重定向、状态、MIME、签名和大小限制；
- 没有 Cookie、Origin 和 Referer；
- 2xx 正常 JSON；
- 4xx/5xx；
- 连接失败和超时；
- redirect 不被跟随；
- Content-Length 超限；
- chunked 响应累计超限；
- JSON 损坏、`success=false` 和结构缺失；
- 并发许可上限。

### 11.3 Core 路由测试

- 未登录为 401；
- 只有 `stock.item.read` 为 403；
- `stock.item.manage` 可以查询；
- 非法编号为 400；
- 每类外部错误映射为稳定状态和 code；
- 查询前后 `stock_items`、属性、文件、审计和库存表计数不变；
- OpenAPI 路径、schema 和响应完整。

为避免测试 Router 强依赖互联网，测试 bootstrap 必须能注入 mock lookup client。生产构造器仍固定真实 endpoint，测试入口保持
`pub(crate)` 或 `#[cfg(test)]`，不能成为平台配置 API。

### 11.4 前端单元测试

为纯映射函数增加覆盖：

- 基础字段覆盖；
- 空候选不清空原值；
- 同名模板属性覆盖；
- 同名自定义属性覆盖；
- 缺失属性创建自定义字段；
- 参数格式化；
- URL 无效时不写入；
- 默认选择第一项模板并允许改选，候选值优先写入同名模板字段；
- 模板缺少的候选字段创建自定义属性；
- 有效第一阶梯价覆盖参考单价，空价格保留当前值；
- 图片 Blob 转换为普通待上传草稿，图片失败保留当前主图；
- 分类、单位和补货点保持不变；
- 查询或“不填写”不改变指纹；
- “覆盖填写”改变指纹。

### 11.5 浏览器与响应式验收

视口：

- `1440 × 900`；
- 约 `768px`；
- `390 × 844`。

状态：

- 初始输入、非法编号、查询中、查询成功、未找到、超时、上游失败和重试；
- 外层草稿为空、已有手填值、已有模板属性和已有自定义属性；
- 不填写、覆盖填写、再次查询；
- 查询中关闭、Escape、遮罩关闭和 Android 返回；
- 外层保存中按钮禁用；
- 长名称、长描述和大量参数。
- `C2983288` 回填第一阶梯参考价 `9.91` 并准备商品首图；
- `C2982` 因库存为零保留当前参考价，`C9900201662` 因无价格结果保留当前参考价；
- 商品无图、图片下载失败或校验失败时保留当前主图，资料与价格仍保持已覆盖状态。

量化检查：

- 外层三按钮不溢出，主操作位置稳定；
- 嵌套 Dialog 的 `clientWidth === scrollWidth`；
- 输入、结果摘要和操作区位于安全区内；
- 关闭后焦点回到触发按钮；
- 最上层 Dialog 关闭不会连带关闭外层；
- 控制台无 error/warning；
- 网络面板只出现前端到 WineStock core 的请求，不出现浏览器到 `pro.lceda.cn` 的请求。

## 12. 实施顺序

### 阶段 A：Core 外部适配器

1. 添加 Reqwest 工作区依赖及最小 feature。
2. 新增外部 catalog runtime 和 `LcscLookupClient`。
3. 实现固定单编号上游请求、超时、重定向、响应上限和私有 DTO。
4. 实现纯归一化与候选投影。
5. 用 mock server 完成 client 和解析测试。

### 阶段 B：Core HTTP 契约

1. 新增 WineStock lookup DTO、handler 和 service。
2. 注册 `item_manage` 路由。
3. 增加稳定错误映射。
4. 更新 OpenAPI 与路由测试。
5. 更新 core API 文档和代码地图。

### 阶段 C：前端查询 Dialog

1. 在 `items.ts` 增加 API DTO 和请求函数。
2. 新增 `LcscItemLookupDialog` 及局部样式。
3. 在 `ItemEditorDialog` create 模式的 modal-actions 添加入口。
4. 接入取消、generation、错误重试、确认和焦点恢复。
5. 验证 nested Modal 和 Android 返回顺序。

### 阶段 D：草稿回填

1. 在 `pages/items/model.ts` 增加纯覆盖函数。
2. 实现基础字段和属性映射。
3. 接入确认后的单次 apply 事件和 Notice。
4. 验证未保存指纹、外层放弃确认和最终创建请求。

### 阶段 E：清理与最终验收

1. 删除临时 `frontend/lcsc-cors-test.html`；其结论已由正式 core 方案取代。
2. 更新前端 API、物品页面文档和前端代码地图。
3. 运行格式、构建、最窄 core 测试、OpenAPI 测试和前端映射测试。
4. 在三档视口执行真实浏览器交互、坐标、溢出、焦点和控制台检查。
5. 如具备 Android 环境，执行新建 Dialog -> 查询 Dialog -> 返回/确认的 WebView smoke。

## 13. 文件清单

预计新增：

- `core/src/external/mod.rs`
- `core/src/external/lcsc.rs`
- `core/src/stock/controller/item_lookup.rs`
- `core/src/stock/service/item_lookup.rs`
- 对应 core 测试文件或现有 `stock_items.rs` 中的聚焦测试
- `frontend/src/components/items/LcscItemLookupDialog.vue`
- `frontend/src/components/items/LcscItemLookupDialog.scss`
- 前端草稿映射测试文件

预计修改：

- `Cargo.toml`
- `Cargo.lock`
- `core/Cargo.toml`
- `core/src/bootstrap.rs`
- `core/src/state.rs`
- `core/src/stock/mod.rs`
- `core/src/stock/controller.rs`
- `core/src/stock/service.rs`
- `core/src/stock/service/error.rs`
- `core/src/http/docs.rs`
- `core/src/tests/http_openapi.rs`
- `core/docs/business-api/items.md`
- `docs/code-map/core/stock.md`
- `frontend/src/api/items.ts`
- `frontend/src/components/items/ItemEditorDialog.vue`
- `frontend/src/components/items/ItemEditorDialog.scss`
- `frontend/src/pages/items/model.ts`
- `frontend/docs/api-client.md`
- 前端物品页面相关文档；若仍无独立页面文档，则新增 `frontend/docs/page-items.md` 并挂入入口
- `docs/code-map/frontend.md`

预计删除：

- `frontend/lcsc-cors-test.html`

实际实施时以职责为准；如果已有测试辅助层能承载 mock endpoint，不重复建立第二套测试框架。

## 14. 完成门槛

功能完成必须同时满足：

- 单个合法立创编号能通过 core 查询并返回归一化候选资料；
- 前端没有直接请求立创域名；
- 查询只在新建物品 Dialog 出现；
- 查询成功后必须经过“不填写/覆盖填写”确认；
- 不填写完全不改变草稿；
- 覆盖填写直接覆盖对应有效字段，不弹逐字段冲突选择；
- 查询不会创建物品、文件、审计记录或库存记录；
- 最终创建仍完整经过现有物品校验和事务；
- 上游超时、失败、畸形数据和无结果均有稳定可重试反馈；
- Cookie、真实会话信息和上游原始响应不进入代码、日志、测试夹具或文档；
- `cargo fmt --check`、覆盖变更的 core 测试、OpenAPI 测试、`pnpm build`、前端聚焦测试和
  `git diff --check` 全部通过；
- 桌面、断点附近和移动视口无溢出，焦点与 Android 返回只作用于最上层 Dialog；
- 临时 CORS 测试页已删除，相关规范与代码地图已同步。
