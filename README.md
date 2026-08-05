<p align="center">
  <img src="brand/winestock-icon.svg" alt="WineStock Logo" width="220">
</p>

<h1 align="center">WineStock</h1>

<p align="center">
  <strong>全平台 • 轻量级 • 全场景物品与库存管理工作台</strong><br>
  让每一件物品有记录，让每一次流转有依据。
</p>

<p align="center">
  <a href="https://github.com/xiaowine/WineStock/releases/latest">
    <img src="https://img.shields.io/github/v/release/xiaowine/WineStock?style=flat-square&color=2b6cb0" alt="Release Version">
  </a>
  <a href="https://github.com/xiaowine/WineStock/releases">
    <img src="https://img.shields.io/github/downloads/xiaowine/WineStock/total?style=flat-square&color=319795" alt="Downloads">
  </a>
  <a href="https://github.com/xiaowine/WineStock/stargazers">
    <img src="https://img.shields.io/github/stars/xiaowine/WineStock?style=flat-square&color=d69e2e" alt="Stars">
  </a>
  <a href="https://github.com/xiaowine/WineStock/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/xiaowine/WineStock?style=flat-square&color=38a169" alt="License">
  </a>
</p>

---

## 项目简介

WineStock 是一款专为个人、团队、工作室和小型仓库打造的全场景物品与库存管理系统。

传统库存软件往往界面复杂、需要昂贵的云端订阅，或者限制在单一设备使用。WineStock 重新设计了库存管理流程，将物品管理、树形库位、批次追溯、出入库审批、替代件网络与团队权限整合到一个现代化、极速响应的工作台中。

无论是整理家庭储物与器材、管理实验室元器件耗材，还是运营小微企业的商品与仓库，WineStock 都能帮助你实现轻松分类、精准定位与清晰流转。

---

## 核心特性

### 1. 全平台一致体验（桌面 + 移动端）
- **多端覆盖**：支持 Windows 桌面客户端与 Android 移动端应用（macOS / Linux 适配中），同时支持标准 Web 浏览器访问。
- **自适应设计**：桌面端提供大屏宽屏的高效批量处理工作台；移动端专门为触控与窄屏优化单手操作。
- **深浅主题**：完美适配深色模式与浅色模式，跟随系统或自由切换，信息层级始终清晰。

### 2. 灵活多变的运行模式
- **本机自用模式（self-hosted）**：默认模式。应用在本地启动服务，数据完备保存在本地设备，离线可用，隐私安全。
- **共享服务模式（server-mode）**：当前设备启动服务并绑定指定监听地址与端口，允许局域网或外部客户端连接协同。
- **连接远端模式（client-only）**：应用不启动本地服务与数据库，直接连接已部署的远端 WineStock 服务地址。

### 3. 精细化批次与先进先出（FIFO）成本预估
- **批次与保质期**：每一批入库物品均可指定批次号、生产日期与保质期，过期自动预警。
- **树形无限库位**：支持“仓库 - 货架 - 抽屉 - 盒”等树形库位结构，物品存放位置一目了然，支持整批快速移库。
- **FIFO 成本预估**：新建出库单时，系统自动根据先进先出（FIFO）原则分摊各批次成本并精准计算预估出库成本。

### 4. 可视化替代关系星链网络
- **星链关系图**：首创可视化“替代关系网络”，采用力导向交互图谱直观展现物品间的兼容与替代关系。
- **快速找替代**：在元器件或耗材短缺时，一键查找可替换的同类物品，大幅提升领用与采购效率。

### 5. 草稿合并工作台与多级审批流
- **串行出入库工作台**：选择物品后立即配置数量、批次与库位，支持出入库草稿保存与合并管理。
- **多级审批机制**：支持直接出入库，也可开启多级审核流。审批人员可预览库存变更影响后再一键通过或驳回。

### 6. 智能数据查询与快捷导入
- **立创商城数据联动**：新建电子元器件等物品时，输入立创商品编号（LCSC/SZLXC）即可自动查询资料并一键填充名称、分类与参数。
- **批量导入与匹配**：支持导入 Excel/CSV 格式的订单与单据，内置智能精准/模糊物品匹配与反馈。

### 7. 细粒度角色权限与完整日志审计
- **多用户权限控制**：按需为团队成员分配查看、录入、管理与审批权限，确保权责清晰。
- **全量日志追溯**：记录每一次库存变动、价格调整与主数据修改，提供三段式详细变更详情查看。

---

## 界面预览

<p align="center">
  <img src="docs/images/desktop-dashboard.png" alt="WineStock 桌面端库存总览（浅色与深色主题拼接展示）" width="100%">
</p>
<p align="center"><em>桌面端库存总览（图中左侧为浅色主题，右侧为深色主题）：库存规模、总价值、流转趋势与告警物品一目了然。</em></p>

<br>

<p align="center">
  <img src="docs/images/android-dashboard.png" alt="WineStock Android 端库存总览（浅色与深色主题拼接展示）" width="380">
</p>
<p align="center"><em>Android 移动端（图中左侧为浅色主题，右侧为深色主题）：延续统一的业务逻辑，为现场手持设备与触控操作打造。</em></p>

---

## 典型使用场景

| 使用场景 | 使用 WineStock 能带来什么？ |
| :--- | :--- |
| **家庭与个人整理** | 记录药物过期时间、家庭工具存放位置、收藏品与备用物资，避免重复购买或遗忘。 |
| **实验室与工作室** | 电子元器件、研发耗材、测试设备分类管理；通过“替代件星链”快速寻找兼容型号。 |
| **小微企业与店铺** | 商品库存管理、批次成本精算、采购入库与销售出库审核，掌握经营资产与资金流向。 |
| **小型仓库与现场仓储** | 树形库位精细划分，手持 Android 设备现场扫码/查看/录入，整批移库与全量日志追踪。 |

---

## 功能一览

- **仪表盘与统计**：库存规模统计、总资产价值计算、出入库趋势折线图、库存告警与保质期预警。
- **物品与分类管理**：多维度筛选、自定义属性模板、图片管理、第三方（立创商城）数据一键回填。
- **库位与批次管理**：无限层级树形库位、批次号追溯、批次过期提示、整批移库。
- **出入库与单据流转**：出入库草稿工作台、FIFO 自动批次扣减与成本预估、单据详情与撤销。
- **库存审批工作台**：待审核单据统一工作台、库存变动影响预览、一键通过/驳回与原因记录。
- **替代关系网络**：可视化星链拓扑图、交互式替代物品检索与关系维护。
- **导入与导出**：支持 Excel/CSV 批量导入单据与订单、自动一次性物品匹配与智能校验提示。
- **用户与权限系统**：多用户管理、细粒度功能权限分配（查看/录入/管理/审批）、个人密码修改。
- **审计与操作日志**：全系统三段式变更日志、历史 JSON 差异对比、多条件服务端筛选。

---

## 快速上手

### 1. 下载应用
访问 [GitHub Releases](https://github.com/xiaowine/WineStock/releases) 下载适合你平台的最新版本：
- **桌面客户端**：Windows (`.exe`)（macOS / Linux 适配中）
- **移动端**：Android (`.apk`)

### 2. 选择适合你的运行方式

WineStock 支持三种运行模式，满足不同场景的使用需求：

- **本机自用（self-hosted）**：默认模式。服务与数据库均在本地设备运行，数据完备保存在本地，离线可用。
- **共享服务（server-mode）**：在本地启动服务并指定监听地址与端口，允许局域网或外部设备的客户端连接协同。
- **连接远端（client-only）**：不启动本地服务与数据库，直接连接已部署的远端 WineStock 服务地址。

> **提示**：应用启动后，你可以随时在“设置 -> 运行设置”中一键切换运行模式或调整连接参数。

---

## 相关文档

- [运行与网络配置指南](docs/runtime-networking.md)
- [多平台支持与说明](docs/platforms.md)
- [开发者与架构文档](docs/README.md)

---


<p align="center">
  Made with ❤️ by <a href="https://github.com/xiaowine">xiaowine</a> and contributors.
</p>