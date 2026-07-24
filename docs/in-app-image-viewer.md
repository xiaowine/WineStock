# 应用内图片查看（不管理系统栏）

> 状态：已实施（前端组件）。  
> 取代曾尝试的 HTML Fullscreen / 壳 hide 系统栏方案；不扩展 Shell Bridge。

## 目标

- 点击缩略图后在 **应用内** 放大查看图片。
- **不**调用 `requestFullscreen`，**不**隐藏状态栏/导航栏，**不**改 Shell。
- 用 `--safe-area-*` 只避让**可点内容**（关闭按钮、图片终点）；遮罩 **铺满整窗**（方案 B）。
- 遮罩为半透明深色（`__dim` + `rgb(11 15 20 / 62%)`），业务页可透出；不用 `backdrop-filter`，避免进场后模糊延迟出现。
- **不** hide 系统栏，故无 inset 恢复闪烁。

## 组件

| 文件 | 职责 |
|------|------|
| `frontend/src/components/InAppImageViewer.vue` | 查看层：open/close、展开动画、Esc/nativeBack |
| `frontend/src/components/InAppImageViewer.scss` | 固定层 + safe-area + 半透明遮罩 |
| `frontend/src/components/PreviewImage.vue` | 缩略图触发器，打开时传入 `originRect` |

调用方（`AuthenticatedImage` 等）仍只用 `PreviewImage`，无需改业务页。

## 状态机

```text
closed --open--> open
  ^                |
  +---- close / Esc / nativeBack ----+
```

无 Fullscreen、无 immersive 标志、无 after-leave 与壳的时序耦合。

## 明确不做

- `SystemBarsController` / `WebChromeClient` 全屏回调（已回撤）
- Shell Bridge 藏栏 / 改图标能力
- 为图标对比度而在栏后故意留浅色条（旧版行为；现方案 B 优先无白条）

## 相关

- 安全区规则：`frontend/docs/mobile-interactions.md`
- 曾调研的藏栏方案已废弃；本文件为现行方案说明。
