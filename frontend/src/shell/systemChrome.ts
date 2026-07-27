// 本文件拥有前端主题与 Android 系统栏薄接口之间的外观协调；不进入 Shell Bridge，也不拥有平台生命周期。

let baseDarkContent = false;
let darkContentOverrideCount = 0;

/** 更新普通页面的系统栏基线；深色主题使用浅色系统栏图标。 */
export function setSystemChromeBaseDarkContent(enabled: boolean): void {
  baseDarkContent = enabled;
  applySystemChromeAppearance();
}

/**
 * 为图片查看等固定深色临时层申请浅色系统栏图标。
 * 返回的释放函数幂等，并恢复当时仍然有效的主题基线。
 */
export function acquireSystemChromeDarkContent(): () => void {
  darkContentOverrideCount += 1;
  applySystemChromeAppearance();
  let released = false;
  return () => {
    if (released) return;
    released = true;
    darkContentOverrideCount = Math.max(0, darkContentOverrideCount - 1);
    applySystemChromeAppearance();
  };
}

function applySystemChromeAppearance(): void {
  try {
    window.WineStockSystemChrome?.setDarkContent(baseDarkContent || darkContentOverrideCount > 0);
  } catch {
    // Web、桌面或平台接口暂不可用时保持 no-op，页面主题本身仍正常工作。
  }
}
