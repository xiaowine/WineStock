// 本文件拥有前端主题与平台原生外观适配器之间的协调；不拥有偏好存储或平台生命周期。

import type { ThemePreference } from "../theme/model";

let baseDarkContent = false;
let baseThemePreference: ThemePreference = "system";
let darkContentOverrideCount = 0;
let appearanceAdapter: SystemChromeAppearanceAdapter | null = null;

export interface SystemChromeAppearance {
  themePreference: ThemePreference;
  darkContent: boolean;
}

export type SystemChromeAppearanceAdapter = (
  appearance: SystemChromeAppearance,
) => void | Promise<void>;

/** 更新普通页面的系统栏/窗口外观基线；深色主题使用深色内容标记。 */
export function setSystemChromeBaseDarkContent(
  enabled: boolean,
  themePreference: ThemePreference = "system",
): void {
  baseDarkContent = enabled;
  baseThemePreference = themePreference;
  applySystemChromeAppearance();
}

/** 注册当前平台的 Shell Bridge 外观适配器，并立即同步当前主题。 */
export function setSystemChromeAppearanceAdapter(
  adapter: SystemChromeAppearanceAdapter | null,
): void {
  appearanceAdapter = adapter;
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
  const darkContent = baseDarkContent || darkContentOverrideCount > 0;
  try {
    window.WineStockSystemChrome?.setDarkContent(darkContent);
  } catch {
    // Web 或平台接口暂不可用时保持 no-op，页面主题本身仍正常工作。
  }
  try {
    const result = appearanceAdapter?.({
      themePreference: baseThemePreference,
      darkContent,
    });
    if (result) {
      void result.catch(() => undefined);
    }
  } catch {
    // 原生窗口外观失败时不阻断页面换肤或业务启动。
  }
}
