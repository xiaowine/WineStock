# 启动漏斗：运行设置优先于认证

> 已实施基线。与 `routes.md`、`page-runtime-settings.md`、`auth-logout-and-route-guards.md` 一致。

## 目标

界面顺序：**运行设置 → `/auth` → 注册/登录 → 业务**。

先确认连哪个服务（含确认 Shell 自动默认），再认证。

## 双判定

| 函数 | 含义 |
| --- | --- |
| `isRuntimeServiceReady` | `configured` + 有效 `apiBaseUrl`（自动默认时也可为 true） |
| `isRuntimeSetupFinished` | 上式 **且** `createdDefault !== true` |

守卫只看 **`runtimeSetupFinished`**。

## 产品规则

- Shell **可**自动起服并注入 `apiBaseUrl`，但自动默认 **`createdDefault=true` 且不得落盘**；用户未「保存设置」前冷启动仍强制设置页。
- **`createdDefault` 仅由「保存设置」→ `apply` 清除**；离开路径不 apply。
- 未确认时：保存按钮可用（即使表单未改）；不展示「继续」。
- 保存成功且匿名：自动进 `/auth`（`returnTo` 桥接为 `redirect`）。
- 已确认后：页头「继续」/「返回应用」；登出落地 `/auth`。
- 不强制：已确认后的日常冷启动、登出再登录；Web `VITE_API_BASE_URL` 注入（`createdDefault=false`）。

## 路由参数

| 参数 | 路由 | 含义 |
| --- | --- | --- |
| `returnTo` | `/settings/runtime` | 离开设置后的目标 |
| `redirect` | `/auth`、`/login` 等 | 登录后业务目标 |

`returnTo` 为业务路径 → 作为 `redirect`；为认证路径 → 提取其内嵌 `redirect`；设置页或非法 → 丢弃。

## 守卫顺序

1. `requiresService === false` → 放行  
2. `!runtimeSetupFinished` → 运行设置 + `returnTo`  
3. 会话初始化 → 匿名/已认证、强制改密、权限  

## 代码锚点

| 位置 | 职责 |
| --- | --- |
| `shell/runtimeReadiness.ts` | 纯判定 |
| `shell/runtime.ts` | `runtimeSetupFinished` |
| `router/guards.ts` | 设置完成门禁 |
| `pages/runtime-settings/leave.ts` | 离开目标与 redirect 桥接 |
| `pages/RuntimeSettingsPage.vue` | 保存确认、自动前进 |
| Android `LocalCoreRuntimeManager` | 自动默认不落盘；apply 落盘并清 `createdDefault` |
| `tests/runtimeFunnel.test.mjs` | 纯逻辑单测 |

## 验收要点

1. 无配置 / `createdDefault` → 强制设置页（默认「在本机使用」）。  
2. 未保存杀进程再进 → 仍是设置页，不进登录。  
3. 点「保存设置」→ `createdDefault=false` 并落盘 → `/auth`。  
4. 已确认 + 未登录 → `/auth`，不强制设置。  
5. 匿名离开设置不直达 `/login`。  
