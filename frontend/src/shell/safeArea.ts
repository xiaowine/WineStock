// 本文件拥有运行时安全区读数：把根元素的安全区 CSS 变量换算为像素，供 JS 定位的浮层预留系统栏空间；
// 它不发布 inset（由 Shell 视口发布器与 env() 负责），也不拥有任何布局。

/** 视口四边安全区像素值；不可用时为 0。 */
export interface SafeAreaInsets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

/**
 * 读取当前生效的安全区（env() 与 Shell 注入取大后的 `--safe-area-*`）。
 * JS 里的 innerWidth/innerHeight 在 edge-to-edge WebView 中包含系统栏覆盖区域，
 * 任何用视口坐标定位的浮层都必须扣除这些读数，否则会被状态栏或导航栏遮挡。
 *
 * 必须经探针元素求值：自定义属性的 getPropertyValue 返回未求值的 `max(env(...), var(...))`
 * 表达式（直接 parseFloat 得到 NaN）；把变量应用为 padding 后浏览器才会解析成像素。
 */
export function readSafeAreaInsets(): SafeAreaInsets {
  const probe = document.createElement("div");
  probe.style.cssText =
    "position:fixed;top:0;left:0;visibility:hidden;pointer-events:none;" +
    "padding:var(--safe-area-top,0px) var(--safe-area-right,0px) " +
    "var(--safe-area-bottom,0px) var(--safe-area-left,0px);";
  document.body.appendChild(probe);
  const styles = getComputedStyle(probe);
  const read = (value: string): number => {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
  };
  const insets = {
    top: read(styles.paddingTop),
    right: read(styles.paddingRight),
    bottom: read(styles.paddingBottom),
    left: read(styles.paddingLeft),
  };
  probe.remove();
  return insets;
}
