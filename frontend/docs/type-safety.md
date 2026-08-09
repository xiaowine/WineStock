# 前端类型卫生约定（any / unknown）

本文件是 `frontend` 的类型安全约定：禁止显式 `any`；`unknown` 只用于信任边界并必须配运行时守卫；业务内部流转一律使用具体类型。

盘点与整改记录见 [`implementation-notes/type-hygiene-remediation.md`](implementation-notes/type-hygiene-remediation.md)。

## 核心规则

1. **禁止显式 `any`。** 任何类型位置不得出现 `: any`、`as any`、`any[]`、`<any>`、`Record<string, any>`、`Promise<any>` 等。`unknown` 在需要"运行时才知道形状"的场景下严格优于 `any`（`any` 会放行静态检查，`unknown` 强制收窄后才可使用）。
2. **`unknown` 只允许出现在信任边界，且必须立即配运行时守卫**（`isXxx(value: unknown): value is Xxx`、`assertXxx(value: unknown): asserts value is Xxx`、`typeof`/`instanceof` 收窄），守卫通过前不得把值当具体类型使用。
3. **业务内部流转使用具体类型。** 守卫收窄后的值、DTO、页面状态、组件 props/emits、composable 返回都应有明确的联合类型或接口；出现"已有具体类型却写 `unknown`"属于懒类型，须收窄（典型清单见整改方案的 B 表）。
4. **`catch` 子句统一 `(error: unknown)` 再收窄**，不得直接 `(error: any)` 或把 catch 参数当具体类型用。
5. **双强转（`as unknown as X`）只在断言守卫之后使用**（如 `assertRuntimeSnapshotSemantics` 之后的语义层收窄），并加注释说明依据。

## 信任边界清单

以下来源不受 TypeScript 静态类型约束，必须按规则 2 处理：

- **网络响应**：`fetch`/XHR 响应体解析（`api/client.ts` 的 `readResponsePayload`、`parseXhrPayload`）、后端统一错误契约（`api/errors.ts`）。
- **HTTP 契约中的任意 JSON 字段**：`ApiErrorResponse.details`、审计事件 `details`（core OpenAPI 声明为 `unknown`，生成 schema 忠实保留，前端解析必须守卫）。
- **Shell Bridge**：运行快照、桌面偏好、原生返回请求、更新检查结果（`shell/contract.ts` 的 `assert*`/`is*`）。
- **本地持久化**：localStorage/IndexedDB 读出的值（会话、主题、语言、捐赠计数、剪贴板模型、物品图片草稿等）。
- **URL query 与路由参数**：`route.query` 解析（EventsPage、审批、出入库列表等），用 `Record<string, unknown>` + 归一化函数收窄。
- **平台异常与事件**：`catch` 错误对象、vue-router 导航失败、懒加载 chunk 失败、摄像头/DOM 异常。
- **指令绑定值与剪贴板输入**：`v-copyable` 的 directive value、粘贴/拖放内容。
- **外部文件解析**：ERP 备份 xlsx、立创订单导出、料袋码、图片读取结果。

## 守卫与收窄示例

```ts
// 正确：信任边界 unknown + 类型守卫
function isPersistedSession(value: unknown): value is PersistedSession {
  return typeof value === "object" && value !== null && typeof value.refresh_token === "string";
}

// 正确：catch 收窄
catch (error: unknown) {
  if (error instanceof ApiError) { /* 具体处理 */ }
}

// 禁止：显式 any
function parse(v: any): string { return v.toString(); }

// 禁止：信任边界 unknown 不守卫直接使用
const value: unknown = JSON.parse(raw);
return value.refresh_token; // 编译错误——这正是 unknown 的价值
```

## 例外

- `frontend/src/api/generated/` 是 openapi-typescript 生成物，`unknown` 与 `any` 均来自生成器对 OpenAPI 的翻译，禁止手改；需要收紧时在 core 侧完善 schema 后执行 `pnpm gen:api` 重新生成。
- `api/contract.ts` 等桥接层对生成类型只做别名映射，不做手工收窄。
- 经守卫断言后语义收窄所需的结构化转换（`value as unknown as X`）允许，但必须紧邻断言并注释依据。

## 门禁与验证

- `pnpm test:type-hygiene`：扫描 `src/**/*.{ts,vue}`（排除 `api/generated/`），命中任何显式 `any` 类型即失败；`unknown` 只统计不拦截，用于观察趋势。
- 新增/修改前端代码后运行 `npx vue-tsc -b`；涉及本约定区域的改动同时运行 `pnpm test:type-hygiene`。
- 发现懒类型点（B 表模式）时在本文件对应的边界清单或整改记录中同步更新，不得静默放行。
