# 前端异步状态切换

本文记录前端对通用异步呈现规则的实现落地。
通用规则（延迟显示、最短展示、后台刷新保留内容、错误即时呈现等）是平台无关的，权威定义见 [`../../docs/async-state-transitions.md`](../../docs/async-state-transitions.md)。
本文只补充前端特定的实现文件、页面行为和参数落地，不重复通用语义。

## 前端参数落地

- 延迟显示：`200ms`。
- 最短展示：`350ms`。
- 进入和离开动画：复用现有 `120ms` 或 `160ms` motion token。

## 初始启动

- `frontend/src/App.vue`：服务首次检查使用 `200ms` 延迟和 `350ms` 最短展示；延迟窗口保持中性背景，不提前挂载业务路由，也不显示短暂的连接面板。
- `frontend/src/components/ServiceUnavailableScreen.vue`：呈现稳定尺寸的连接或断连全屏状态，不决定计时策略。

## 后台刷新

- 目录筛选、排序和刷新保留当前内容；新请求取消旧请求，主动取消不展示错误。
- 多页 Dialog 的库存数据按需加载并在当前会话缓存；手动刷新保留已有摘要，批次失败保留已加载页并只重试失败页。
- 尚未持久化的创建会话不得创建库存异步状态，也不得因为创建响应获得 ID 而临时挂载库存页面。
- 定时健康检查不切换为全屏“检查中”。

## 当前实现

- `frontend/src/composables/useStablePendingIndicator.ts`：把即时 pending 转换为延迟显示且满足最短展示时间的只读状态。
- `frontend/src/router/navigationPending.ts`：路由切换（含懒加载 chunk 下载和守卫等待）复用 `200ms` 延迟与 `350ms` 最短展示，驱动全局顶部进度条（`frontend/src/components/RouteProgressBar.vue`）和侧栏导航目标的乐观等待反馈；等待可见期间目标高亮不因导航完成瞬间闪断。chunk 加载失败属于明确错误，立即通过全局 Notice 呈现并提供整页刷新重试。
- `frontend/src/router/appPageLoaders.ts`：应用壳页面 chunk 的统一懒加载入口；进入应用壳后在浏览器空闲时按当前权限顺序预取，让弱网下的路由切换尽量不经过网络；预取失败静默，由上述路由错误处理兜底。
- `frontend/src/service/availability.ts`：拥有真实健康检查状态和重试调度，不为视觉稳定性人为延迟请求。
- `frontend/src/pages/ItemsPage.vue`：物品目录的初始加载和分页提示复用 `200ms` 延迟与 `350ms` 最短展示；搜索、刷新和 Dialog 保存后的重新加载保留现有列表与全宽目录骨架，只在稳定等待状态出现后显示局部弱化反馈，不能因打开或关闭编辑 Dialog 清空目录。
- `frontend/src/pages/UsersPage.vue`：用户搜索、状态筛选和手动刷新保留现有列表；超过 `200ms` 后才弱化列表并驱动刷新图标，反馈出现后至少保持 `350ms`，刷新失败继续保留旧结果；数量变化只切换数字提示，零条结果隐藏空表格并由底部提示显示已加载 `0` 个用户。

后续页面、对话框或路由懒加载出现类似闪烁时，应优先复用稳定等待状态，而不是在各组件中散落新的 `setTimeout`。
