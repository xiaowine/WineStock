# 文本长度校验口径统一整改方案

状态：已实施。全部用户可见文本改用 `length(utf16, ...)`，技术字段、JSON 载荷与凭据显式标注 `bytes`，密码同时保留 UTF-16 界面契约与独立字节上限，立创归一化改为 UTF-16 安全裁剪。`cargo fmt --check`、`cargo test -p winestock-core`（119 通过）、`pnpm gen:api`（无结构差异）与 `pnpm build` 均已通过。

## 1. 背景

立创器件 `C2688377` 的候选描述约为 510 个 Unicode 字符，但 UTF-8 编码后为 1112 字节。前端显示与输入限制均未超过 1024，提交 `POST /api/items` 时却收到：

```json
{
  "error": {
    "code": "invalid_request",
    "details": {
      "fields": [
        {
          "message": "length is greater than 1024",
          "path": "description"
        }
      ],
      "kind": "validation"
    }
  }
}
```

根因不是立创描述异常，而是前后端长度单位不同：

| 位置 | 当前计数单位 |
| --- | --- |
| HTML `maxlength` | UTF-16 code unit |
| JavaScript `String.length` | UTF-16 code unit |
| Garde `length(...)` 默认 `simple` 模式作用于 Rust `String` | UTF-8 字节 |
| 立创查询 `truncate_chars` | Unicode scalar value |
| SQLite `TEXT` | 当前没有统一的应用层字符数约束 |

因此中文通常在只达到界面标称上限约三分之一时就被 Core 拒绝。该问题不只影响 1024 长描述，也影响允许中文的名称、来源、去向、属性名、单位和默认值等短文本。

## 2. 目标

- 前端、Core HTTP DTO、service 归一化和 repository 输入对同一业务字段使用同一长度单位。
- 用户可见文本的“最多 N 个字符”与浏览器实际输入行为一致。
- 保留技术字段、协议字段、原始载荷和集合数量原有的安全边界，不把所有 `length` 机械改成文本字符计数。
- 创建和更新使用相同规则；字段缺失、显式 `null` 和非空文本继续保持现有语义。
- 立创查询、ERP 导入和批量创建等程序化赋值路径不能绕过同一限制。
- 不修改数据库结构，不引入新依赖，不截断用户手工输入。

## 3. 统一口径

### 3.1 用户可见文本

用户可见、由浏览器表单编辑且界面描述为“字符”的字段统一按 UTF-16 code unit 计数。Core 使用 Garde 0.23 原生模式：

```rust
#[garde(
    length(utf16, min = 1, max = 1024),
    custom(validate_optional_not_blank)
)]
pub description: Option<String>,
```

选择 UTF-16 的原因：

- 与 HTML `maxlength` 一致；
- 与项目现有 JavaScript `String.length` 校验一致；
- 中文字符按 1 计数，可直接解决 `C2688377`；
- 无需前端引入另一套字符计数工具或改变输入控件行为；
- Garde 0.23 已原生支持 `utf16`，无需增加 crate。

UTF-16 是本项目的前后端契约单位，不等同于 UTF-8 字节数，也不承诺按用户感知的 Unicode grapheme cluster 计数。组合字符和 emoji 可能占多个 code unit，这是浏览器原生输入限制的既有行为。

### 3.2 技术字段与安全上限

以下限制不改为用户文本字符数，应保留字节、格式或固定协议语义；实施时宜显式标注 `bytes`，避免再次依赖 Garde 默认行为：

- SKU、权限代码、实体类型、动作代码和客户端版本；
- URL、IP、MIME、日期、数据库路径和存储路径；
- access token、refresh token、exchange token 和哈希；
- HTTP/JSON/图片响应体等原始载荷大小；
- 立创客编和其它已限定 ASCII 格式的外部编号。

密码需要同时满足用户界面的 UTF-16 长度契约和独立的请求字节安全上限。不能仅把现有字节限制放宽后取消总载荷保护。

### 3.3 集合数量

`Vec`、候选项数量、筛选字段数量和分页条数继续按元素数量校验。Garde 的集合 `length` 不属于本次字符串字节误判，不需要修改。

## 4. 当前影响范围

本次静态扫描在 `core/src` 与 `shared/src` 中发现 147 处 Garde `length` 标注，其中包含字符串、集合、请求 DTO、响应 DTO 和 repository 输入，不能按命中数量直接批量替换。

当前仍有 27 处 `max = 1024` 使用默认字符串长度模式，覆盖以下业务域：

| 业务域 | 字段 | 主要层次 |
| --- | --- | --- |
| 物品 | 描述 | HTTP 响应及物品相关内部模型 |
| 分类 | 分类说明 | create/update/response、repository |
| 属性模板 | 模板说明 | create/update/response、repository |
| 入库 | 入库备注 | create/response、repository |
| 出库 | 出库备注 | create/response、repository |
| 库位 | 库位备注、移库备注 | create/update/response、repository |
| 替代关系 | 兼容性备注 | bind/update/response、repository 投影 |

除 1024 字段外，下列允许中文的短文本也应按相同原则整改：

- 物品、分类、模板、库位和库位组名称；
- 入库来源、出库去向；
- 属性字段名称、固定单位、单位候选项和文本默认值；
- 用户名与可见设备名称，前提是产品未限定为 ASCII；
- 其它前端使用 `maxlength` 或 `String.length` 表达字符上限的业务文本。

以下字段需要逐项确认后再决定，不能机械归类：

- 批次号：可能是外部技术编号，也可能允许人工中文标签；
- 原始文件名：是用户可见元数据，但还受平台文件系统和上传安全上限影响；
- 密码：字符规则和抗超大请求的字节规则应并存；
- repository 中只承载内部枚举字符串的字段：应保持技术代码语义。

## 5. Core 整改

### 5.1 普通字符串

必填 `String` 和普通 `Option<String>` 优先直接使用 Garde 原生 `length(utf16, ...)`。非空白校验继续由 `validate_not_blank` 或 `validate_optional_not_blank` 独立承担。

物品描述当前临时使用的 `validate_optional_description` 在原生长度模式覆盖后应删除，避免为每个长度上限继续增加专用函数。

### 5.2 可空更新字段

`Option<Option<String>>` 用于区分：

- 外层 `None`：请求未携带字段，保留原值；
- `Some(None)`：显式传 `null`，清空字段；
- `Some(Some(value))`：更新为新文本。

这种嵌套结构保留最小的 nullable 适配校验。适配器只负责展开三态并调用统一 UTF-16/非空白规则，不复制 controller、service 和 repository 的业务流程。创建和更新必须覆盖相同最大长度。

### 5.3 HTTP 与 repository 双层约束

- HTTP DTO 负责向客户端返回稳定的 `400 invalid_request` 和字段路径。
- repository 输入保留同一静态约束，防止内部调用绕过 HTTP 后写入无效数据。
- service 继续负责 trim、可空语义、业务关联和数据库查询，不再另建互相冲突的长度单位。
- response DTO 若保留 Garde 派生，其人类可读文本标注也应使用相同模式，防止内部验证或未来复用重新产生字节误判。

### 5.4 立创数据归一化

`core/src/stock/service/item_lookup.rs` 当前按 Rust `chars()` 裁剪描述、参数名和参数值。为与 HTTP 契约严格一致，应改为 UTF-16 安全裁剪：

- 遍历 Unicode scalar value；
- 累加 `char::len_utf16()`；
- 加入下一个字符会超过上限时停止；
- 不在代理对中间切断，也不生成非法 Rust 字符串。

描述、参数名和参数值分别沿用现有最大值，只改变计数单位。不得用 UTF-8 字节切片，也不得直接对字节索引截断。

## 6. 前端整改

普通表单不需要替换现有 HTML `maxlength`，现有 JavaScript `.length` 也无需改成 `Array.from(...).length`。两者已经是目标 UTF-16 语义。

需要审计所有不经过用户键盘输入的赋值路径：

- 立创单个查询覆盖物品草稿；
- 立创订单未命中物品的一键批量创建；
- ERP 备份导入后的批量创建；
- 本地草稿恢复；
- API 数据复制、模板复制及其它程序化表单回填。

这些路径必须在启用提交或创建按钮前执行与普通表单相同的 `.length` 校验。程序化赋值可能绕过 DOM `maxlength`，不能只依赖输入控件。

处理原则：

- 用户手工文本超限：显示字段错误并禁止提交，不静默截断；
- 受控上游候选数据：优先由 Core 在归一化阶段安全裁剪；
- 导入文件中的用户数据：预览中标记具体行和字段，禁止该行创建，不静默改变原数据；
- 批量流程：单项字段错误按现有失败隔离规则记录，不阻塞其它合法项目。

## 7. 实施步骤

1. 建立字段清单，将 147 处标注分类为 `utf16`、`bytes`、集合数量或待确认项。
2. 先整改物品、分类、模板、库位、入库、出库和替代关系中的用户可见文本。
3. 同步整改 HTTP DTO、repository 输入和仍会被验证的响应/内部模型。
4. 删除被 Garde 原生模式取代的 `validate_optional_description`，只保留嵌套 nullable 所需的最小适配器。
5. 将立创查询字符串裁剪统一为 UTF-16 安全裁剪，并补边界测试。
6. 审计前端普通表单与程序化赋值路径，补齐缺失的提交前校验。
7. 单独确认密码、批次号、原始文件名和用户名的产品语义，再处理剩余待确认字段。
8. 更新受影响的 Core/前端业务文档，将“字符”明确为与浏览器一致的 UTF-16 code unit。
9. 重新生成 OpenAPI 前端契约类型；预计结构类型不变，但仍按项目规则执行 `pnpm gen:api`。

不建议一次提交同时重构所有校验模块。实施提交可按“通用规则与物品”“库存单据与库位”“分类模板与替代关系”“认证及边界字段”拆分，但每个字段必须在同一提交中完成 HTTP、repository、前端和测试闭环。

## 8. 测试方案

### 8.1 Core 单元测试

每种长度模式至少覆盖：

- ASCII 恰好达到上限；
- 中文恰好达到上限且 UTF-8 字节数超过上限；
- 中文超过上限 1 个 UTF-16 code unit；
- emoji 或增补平面字符在 UTF-16 中占 2 个 code unit；
- `None`、显式 `null`、空字符串和纯空白；
- 技术字段仍按其字节或格式规则拒绝非法输入；
- UTF-16 安全裁剪不会切断字符。

### 8.2 Core 接口测试

不必为每个重复 DTO 建立完整 CRUD 测试，但每个规则类别至少选择一个真实接口：

- 物品描述：使用 `C2688377` 等价中文长文本创建成功，1025 个中文字符失败；
- 分类或模板说明：中文长文本创建和更新一致；
- 入库或出库备注：程序化请求与界面标称上限一致；
- 库位或移库备注：HTTP DTO 与 repository 双层规则一致；
- 嵌套 nullable：字段缺失保留、`null` 清空、合法文本更新、超限文本拒绝。

### 8.3 前端测试

- 表单边界值不会错误禁用提交；
- `.length === max` 可提交，`.length === max + 1` 显示字段错误；
- 立创回填、订单导入和 ERP 导入不能绕过长度校验；
- 批量创建只隔离失败行；
- 服务端字段错误仍能映射到对应控件或批量结果。

### 8.4 验证命令

按受影响范围执行最窄有效检查，完整整改至少包括：

```text
cargo +stable fmt --all -- --check
cargo +stable test -p winestock-core
cd frontend
pnpm gen:api
pnpm build
```

另执行现有立创、批量创建和 ERP 导入相关 Node 测试。若生成的 OpenAPI TypeScript 文件无差异，也应记录已重新生成。

## 9. 验收标准

- `C2688377` 的立创描述可直接创建物品，不再因 UTF-8 字节数超过 1024 被拒绝。
- 所有界面标注“最多 N 个字符”的用户文本，在前端和 Core 使用同一 UTF-16 计数。
- 普通创建、编辑、立创回填和两类批量导入使用同一规则。
- 1024 个中文字符通过，1025 个中文字符稳定返回字段级 `invalid_request`。
- 技术字段、集合数量和原始载荷大小限制没有被意外放宽。
- 创建和更新、HTTP 与 repository、单项与批量路径均有代表性回归测试。
- 不新增数据库迁移或第三方依赖，不修改现有持久化数据。

## 10. 非目标

- 不把“字符”改为 Unicode grapheme cluster，也不引入 Unicode 分词依赖。
- 不修改数据库列类型或为 SQLite `TEXT` 增加破坏性迁移。
- 不对存量数据做自动截断或清洗。
- 不把所有字符串都改成 UTF-16；技术字段继续遵守各自协议与安全边界。
- 不借本次整改调整业务字段最大值、错误响应结构或页面文案。
