# 立创商城查询与图片直连实施

状态：已实施。

## 目标

WineStock 的单商品查询由立创 EDA 器件库切换到立创商城移动查询接口，解决商城商品未进入 EDA
器件库时被误判为不存在的问题。商品资料仍由 Core 查询、裁剪并通过稳定 HTTP 契约返回；商品图片由
前端直接从允许跨域读取的立创图片域名获取，不再由 Core 中转二进制内容。

## 边界

```text
frontend -> WineStock Core -> POST https://so.szlcsc.com/phone/global/query
frontend ------------------> GET  https://alimg.szlcsc.com/...
frontend -> WineStock Core -> POST /api/files/images
```

- 浏览器不能跨域读取商城查询接口，因此查询必须留在 Core。
- Core 固定上游 endpoint 和请求结构，不接受前端提供 URL、请求头、Cookie、token 或批量编号。
- Core 只公开归一化资料和通过白名单校验的图片 URL，不公开完整上游 JSON。
- 前端对图片只发无凭据、无自定义头的普通 GET；图片域名不允许依赖 OPTIONS 预检。
- 最终物品主图仍上传为 WineStock 受控文件，数据库不保存第三方图片 URL。

## 上游请求

Core 固定发送：

```json
{
  "keyword": "C41408468",
  "pageSize": 10,
  "currentPage": 1,
  "searchSource": "main_so",
  "asyncRequest": false
}
```

只设置 `Accept: application/json` 和 `Content-Type: application/json`。不复制微信 User-Agent、Referer、
`Sec-Fetch-*`、`x-lc-accesstoken`、`x-lc-accesssharecode` 或手工 `Content-Length`。保持现有 HTTPS-only、
禁重定向、连接/完整请求超时和共享并发限制；不对上游 JSON 响应体设置大小上限。

响应必须满足 HTTP 2xx、顶层 `code = 200`，若存在 `ok` 则必须为 `true`，并从
`result.searchResult.productRecordList` 中按 `productVO.productCode` 选择唯一精确匹配项。空列表或没有精确
匹配返回 `lcsc_product_not_found`；HTTP、超时、JSON 或结构错误继续映射到现有稳定错误码。

## 字段映射

- 商品编号：`productVO.productCode`。
- 名称：`lightProductModel`，回退 `productVO.productModel`、`lightProductName`、`productVO.productName`。
- 描述：`lightProductIntro`，回退 `productVO.productName`。
- 制造商：`lightBrandName`，回退 `productVO.productGradePlateName`。
- 制造商型号：`lightProductModel`，回退 `productVO.productModel`。
- 封装：`lightStandard`，回退 `productVO.encapsulationModel`。
- 数据手册：`fileTypeVOList[fileType=pdf_property]` 的首个 `fileUrl`，只接受 `/upload/public/pdf/` 路径并
  组合到 `https://atta.szlcsc.com`。
- 参考单价：`productPriceList` 中起订量最小且价格为正的档位。
- 参数：`paramLinkedMap` 中的标量文本，继续执行数量、名称和值长度限制。
- 图片：优先 `bigImageUrl`，回退 `breviaryImageUrl`；仅接受 HTTPS、主机严格等于
  `alimg.szlcsc.com`，且路径位于 `/upload/public/product/` 或
  `/upload/public/brand/product/certificate/`（部分商品如 `C53309018` 使用后者）。

## 前端图片读取

`LcscItemLookupResponse` 新增可空 `image_url`。用户确认覆盖后，前端直接读取该地址，要求：

- HTTP 成功；
- 响应及 Blob MIME 为 PNG、JPEG 或 WebP；
- `Content-Length`（存在时）和最终 Blob 均不超过 15 MiB；
- 不携带 WineStock Bearer token、Cookie 或其它自定义头；
- 图片失败不回滚已经应用的候选资料，单个创建保留现有主图；批量创建遇到上游明确无图时生成带客编的
  “暂无商品图片”PNG，真实图片地址存在但读取失败时仍令该行失败。

旧的 `GET /api/items/lookups/lcsc/{product_code}/image` 在同一改动中删除，不保留兼容层。

## 验证

- Core mock 测试固定请求体、精确匹配、字段映射、图片 URL 白名单和稳定错误码。
- OpenAPI 删除图片代理路径并为候选响应生成 `image_url`。
- 前端覆盖图片直连成功、非法 URL、MIME、大小、取消和失败路径。
- 真实 smoke 使用已确认 EDA 查不到但商城可查的 `C41408468`。

实施验收结果：

- `cargo +stable test -p winestock-core`：117 项通过；
- `pnpm test:lcsc-image`：3 项通过；
- `pnpm test:item-lcsc`：2 项通过；
- `pnpm build`：通过；
- OpenAPI 连续生成结果稳定，旧图片代理路径已移除，候选响应已生成 `image_url`；
- `C41408468` 商城查询返回 HTTP 200 和精确商品，浏览器从 WineStock 来源经 `lcsc/image.ts` 直连读取
  `alimg.szlcsc.com` JPEG 成功（10,604 bytes）。
- `C53309018` 的真实图片位于 `/upload/public/brand/product/certificate/`，浏览器经同一模块读取 JPEG
  成功（200,210 bytes）；`C3288081`、`C20190920` 的商城搜索与商品详情均明确无图，一键批量创建
  改用带客编的“暂无商品图片”PNG，不将有效器件判为创建失败。
