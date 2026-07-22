# 运行设置 Tab 与全站切换控件一致性整改方案

## 问题结论

运行设置页的「运行方式」切换，与项目内其它内容页 / 工作区切换控件在**视觉语言、信息结构、无障碍语义和响应式策略**上都不统一。  
上一轮只解决了「移动端 tab 随内容滚动」；为了粘滞可用，又在 `RuntimeModeSelector` 内新增了一套**仅此页面存在**的横向分段样式，进一步拉大了与正式 tab 规范的差距。

这不是业务能力缺失，而是**切换控件没有提升为共享模式**，各页各自实现近似 UI。

## 现状盘点

当前前端至少存在 4 套「互斥切换」呈现：

| 编号 | 场景 | 源码 | 语义意图 | 桌面形态 | 移动形态 | 选中态 | 无障碍 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A | 页面业务域 tab | `TemplatesPage` + `.templates-tabs` | **内容页切换** | 顶部横向 pill 行 | 同形态，可横滚 | accent 字 + soft 底 + 浅 accent 边 | `tablist` / `tab` / `tabpanel` + 左右键 |
| B | 工作区页导航 | `ItemEditorDialog` + `.item-workspace__nav` | **工作区内页面切换** | 左侧竖向列表 | 等分横排 | 桌面左侧 3px accent 条；移动底部 3px accent 条 | `button` + `aria-pressed`（非完整 tab 模式） |
| C | 表单分段 | `ItemUnitSettingsDialog` + `.item-unit-settings__segments` | **高频互斥字段值** | 等分边框分段组 | 同形态 | soft 底 + inset accent 边 | `radiogroup` + `button` |
| D | 运行方式 | `RuntimeModeSelector` | **运行配置工作区模式** | 左侧 radio 卡片（圆点 + 标题 + 说明） | 临时横向边框分段 | 桌面 radio 圆点；移动 soft 底无指示条 | `fieldset` + 隐藏 `radio` |

规范文档对选择控件的边界（见 `ui-design-guidelines.md` / `ui-consistency-checklist.md`）：

- **tab**：切换相对稳定的内容页或工作区。
- **分段控件**：高频互斥输入模式，不用于普通筛选。
- 不得因候选项少就临时发明另一套近似 tab。

对照语义，运行方式更接近 **B（工作区导航）**：切换后右侧配置舞台变化，而不是改一个普通表单字段值。  
但它目前用了 **C/D 混搭视觉 + radio 卡片信息结构**，移动端又临时做成接近 C 的边框分段，与 **A（正式页面 tab）** 也不一致。

## 不一致点（可验收）

### 1. 视觉语言

- A：圆角 pill、透明底、选中 soft + 浅边。
- B：平面列表 / 横排，选中靠 **accent 指示条**（左或底），无 pill 边框。
- C：整组 1px 强边框、等分格、选中 inset。
- D 桌面：独立卡片边框 + **radio 圆点**（全站内容切换中独有）。
- D 移动：边框分段 + soft 底，**无 A 的 pill，也无 B 的指示条**。

用户在「分类与模板」和「运行设置」之间切换时，会感到两套产品语言。

### 2. 信息结构

- A/B/C：选项只显示短标签。
- D 桌面：标签 + 多行说明 + 不可用原因。
- D 移动：隐藏说明，但配置区另有 `modeTitle` 承接——合理，却与桌面信息密度不对齐。

### 3. 布局职责

- A：tab 固定在工作区顶部，与列表同属连续工作区。
- B：桌面左栏 / 移动顶栏，与面板一体。
- D：桌面左栏像 B，但选项外观像设置向导；移动 sticky 条带 blur/阴影，是运行设置页私有壳层，其它 tab 无此处理。

### 4. 无障碍与键盘

- 仅 A 具备完整 `tablist` 键盘左右切换。
- B 用 `aria-pressed`，未实现 roving tabindex。
- D 用原生 radio 语义（对「选一个配置值」合理），但与「切换工作区面板」的 tab 模式并存时，读屏用户听到的交互模型不一致。

### 5. 规范落点缺失

- `page-templates.md` 详细规定了业务域 tab。
- `page-runtime-settings.md` 只写「模式导航 / 移动粘滞 tab」，未引用全站控件模式。
- 没有共享 `SegmentedControl` / `WorkspaceTabs` 组件，新页面只能继续复制。

## 整改目标

1. **统一语义分类**，先定「用哪一类控件」，再改皮肤。
2. **运行方式对齐选定规范模式**，消灭 D 的私有视觉与私有 mobile 分段。
3. **抽出可复用层**，让 A/B/C 收敛，而不是只给运行设置打补丁。
4. **保留已修复的移动粘滞与底部固定操作栏**，不回退滚动问题。
5. **不改变 Shell Bridge 配置契约、模式枚举与校验逻辑**。

## 语义裁定（方案前提）

| 控件场景 | 归类 | 推荐模式 |
| --- | --- | --- |
| 分类与模板业务域 | 内容页切换 | **A：Page Tabs** |
| 物品工作区「资料 / 库存 / 替代」 | 工作区内页面 | **B：Workspace Nav** |
| 单位规则 none/fixed/select | 表单互斥值 | **C：Segmented Control** |
| 运行方式 本机/远程/局域网 | 运行配置工作区模式 | **B：Workspace Nav**（首选） |

**为何运行方式选 B 而不是 A 或 C：**

- 与桌面双栏（模式栏 + 配置舞台）结构一致，和物品工作区同构。
- 不是普通字段筛选，也不是与列表同级的业务域 tab 条。
- 说明文案应落在**配置舞台标题/提示区**，而不是塞进每个 tab 标签（对齐 B/A 的「短标签」）。

备选：若产品坚持「运行设置也是页面级业务域」，可改归 A；但桌面双栏会与 A 的「顶栏横排」冲突，需要同时改桌面 IA，成本更高。本方案默认 **B**。

## 目标形态

### 桌面（≥768px）

```text
┌───────────────┬────────────────────────────┐
│ 运行方式（nav）│ 配置舞台                    │
│ 本机运行       │ eyebrow + 标题 + 未保存     │
│ 连接远程服务 ▎ │ 字段…                      │
│ 局域网服务器   │ 底部操作（页内 footer）     │
└───────────────┴────────────────────────────┘
```

- 左侧使用与 `.item-workspace__nav` **同一选中语言**：透明底、底部分隔线、选中 `background: surface` + `box-shadow: inset 3px 0 accent` + accent 字重。
- **去掉 radio 圆点**与卡片描边。
- 选项仅短标签；说明迁到配置区（已有 `modeTitle` / hint / warning）。
- 不可用项：禁用 + 配置区或选项下方一行 warn 文案（不把长说明堆进每个未选中项）。

### 移动（≤767px）

```text
粘滞页头
服务摘要（可滚走）
粘滞 Workspace Nav（横排等分，底指示条）  ← 对齐 B 移动
配置舞台
固定底栏操作
```

- 横排等分 / 可横滚（>3 项时），选中 **底部 3px accent 条**，与物品工作区移动 nav 一致。
- **不再使用** 当前 Runtime 私有边框分段组（避免与 C 混淆）。
- 保留 sticky：`top = 实测页头高度`；背景用 `surface` / `surface-raised`，阴影克制，不引入另一套 blur 产品语言（若 Shell 顶栏已有透明度可对齐 AppShell，而不是单独发明）。

### 无障碍

短期（与 B 对齐）：

- 容器 `role="navigation"` 或保持 `radiogroup` / 按钮组 + `aria-pressed`。
- 禁用项 `aria-disabled` / `disabled`，不可用原因用 `aria-describedby` 指向可见说明。

中期（与 A 收敛，可选）：

- 若确认「切换配置舞台 = 内容切换」，升级为 `tablist` + `tab` + `tabpanel`，复用 Templates 的左右键 roving tabindex。
- 配置舞台容器成为唯一 `tabpanel`。

本方案 **Phase 1 对齐 B 的视觉与结构；Phase 2 再统一 A/B 的 a11y 模型**。

## 共享组件设计

### 新建（推荐）

```text
frontend/src/components/navigation/WorkspaceNav.vue
frontend/src/components/navigation/WorkspaceNav.scss
```

职责：

- 输入：`items: { id, label, disabled?, descriptionId? }[]`，`modelValue`，`orientation: 'vertical' | 'horizontal' | 'auto'`。
- `auto`：桌面竖、移动横（与物品工作区一致）。
- 输出：`update:modelValue`。
- 不包含业务说明长文、不请求 API、不知晓 runtime/item 领域。

样式 token 全部来自 foundation；**一份 SCSS 服务 Item 工作区与 Runtime 模式栏**。

### 可选第二共享件

```text
frontend/src/components/navigation/PageTabs.vue   // 从 Templates 抽出 A
frontend/src/components/forms/SegmentedControl.vue // 从单位设置抽出 C
```

Phase 1 可不做 A/C 抽取，但文档必须写明「禁止再复制第四套」。

### 废弃方向

- `RuntimeModeSelector` 的移动私有 flex 分段样式删除。
- 若 `RuntimeModeSelector` 仅剩业务选项数据，可降级为「组装 WorkspaceNav + 模式默认值」的薄包装，或直接在 `RuntimeSettingsPage` 使用 `WorkspaceNav`。

## 分阶段实施

### Phase 0 — 规范落盘（0.5d）

1. 在 `ui-design-guidelines.md`「工具栏与控件」补充三类切换控件对照表（A/B/C）及选用规则。
2. 在 `ui-consistency-checklist.md` 增加验收条目：同语义不得出现第二套选中指示（圆点 vs 指示条 vs pill）。
3. 更新 `page-runtime-settings.md`：明确运行方式 = Workspace Nav，不再称「临时分段 tab」。

**完成门槛：** 文档评审通过，无代码行为变化。

### Phase 1 — 共享 WorkspaceNav + 运行设置接入（1–1.5d）

1. 实现 `WorkspaceNav`（竖/横/auto + 禁用 + 焦点环 + reduced-motion）。
2. `ItemEditorDialog` 改为消费 `WorkspaceNav`（行为不变，防回归）。
3. `RuntimeModeSelector` / 运行设置页改为同一组件：
   - 去掉 radio 圆点与卡片边框；
   - 桌面竖栏、移动横栏；
   - 说明只留在配置舞台；
   - 保留 sticky 与底栏，粘滞层背景对齐 WorkspaceNav 容器而不是私有 blur 块。
4. 局域网不可用：禁用对应项 + 配置区/摘要一条 warn。

**完成门槛：**

- 390×844：滚动时模式栏粘在页头下；选中底指示条；无横向整页溢出。
- 1440×900：左栏指示条选中，与物品工作区视觉一致。
- 物品工作区三页切换、替代关系显隐回归通过。

### Phase 2 — Page Tabs / Segmented 收敛（1d，可排期）

1. 抽出 `PageTabs`，Templates 接入。
2. 抽出 `SegmentedControl`，单位设置接入。
3. 全局 grep 禁止新增 `.xxx-tabs` / `__segments` 私有复制，除非页面文档声明例外。

### Phase 3 — a11y 统一（0.5–1d，可选）

1. 决策：Workspace Nav 是否升级为完整 tab 模式。
2. 若升级：共享 roving tabindex composable（从 Templates `handleTabKeydown` 提取）。
3. 审计：运行设置、物品工作区、分类模板三处键盘路径一致。

## 明确不做

- 不改 `RuntimeMode` 枚举、默认端口、Shell Bridge、校验与 apply 流程。
- 不把运行设置塞进 `AppShell` 业务导航。
- 不用 Material / 大圆角消费级 tab。
- 不把单位规则等表单分段改成 Page Tabs。
- 不在 Phase 1 重做服务摘要卡片或底栏交互。

## 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| Item 工作区回归 | Phase 1 先迁 Item 再迁 Runtime；截图对比选中/hover/移动横排 |
| 说明文案变短后用户不懂模式 | 配置舞台保留标题 + 既有 hint/warning；高级说明放 details |
| sticky 与 WorkspaceNav 背景缝隙 | sticky 容器与 nav 同色，负 margin 与 workspace padding 对齐实测 |
| a11y 从 radio 改为 button 的读屏变化 | Phase 1 文档记录；Phase 3 再升 tab 模式时补验收 |

## 验收清单

### 视觉

- [ ] 运行方式选中指示与物品工作区一致（桌面左条 / 移动底条）。
- [ ] 无 radio 圆点、无私有边框分段组。
- [ ] 与 Templates 的 pill tab 可区分（A ≠ B），但同属 token 与字重体系。
- [ ] 桌面 1440、断点附近 768、移动 390 无横向溢出。

### 交互

- [ ] 三模式切换只换配置舞台，不丢草稿字段校验状态策略与现网一致。
- [ ] 禁用「局域网」时不可点，原因可见。
- [ ] 移动滚动：页头 + 模式栏粘滞，底栏固定，末项不被挡。

### 工程

- [ ] 无第二套 Workspace Nav SCSS 复制在 `RuntimeModeSelector`。
- [ ] `ItemEditorDialog` 与 Runtime 共用组件。
- [ ] 文档：`ui-design-guidelines`、`page-runtime-settings`、本 notes 同步。
- [ ] 代码地图：若新增 `components/navigation/`，更新 `frontend` 代码地图条目。

## 建议实施顺序（一句话）

**先定 B 为运行方式规范 → 抽 WorkspaceNav → Item 回归 → Runtime 换皮并对齐粘滞 → 再抽 A/C 防再分裂。**

## 相关文件

| 文件 | 关系 |
| --- | --- |
| `frontend/src/components/runtime/RuntimeModeSelector.*` | 当前不一致实现 |
| `frontend/src/pages/RuntimeSettingsPage.*` | 粘滞壳、配置舞台 |
| `frontend/src/components/items/ItemEditorDialog.*` | 应对齐的 Workspace Nav 参考实现 |
| `frontend/src/pages/TemplatesPage.*` | Page Tabs 参考实现 |
| `frontend/src/components/items/ItemUnitSettingsDialog.vue` | Segmented 参考实现 |
| `frontend/docs/ui-design-guidelines.md` | 需补三类控件规则 |
| `frontend/docs/page-runtime-settings.md` | 需改模式栏定义 |
| `frontend/docs/page-templates.md` | A 的既有规范 |

## 预估

| 阶段 | 工作量 | 可交付 |
| --- | --- | --- |
| Phase 0 | 0.5d | 规范与选用表 |
| Phase 1 | 1–1.5d | 运行设置与物品工作区视觉统一 |
| Phase 2 | 1d | A/C 共享化 |
| Phase 3 | 0.5–1d | 键盘 / tab 语义统一 |

**最小可合并增量：** 仅 Phase 0 + Phase 1，即可消除用户感知的「运行页 tab 和别的页不像一套」。
