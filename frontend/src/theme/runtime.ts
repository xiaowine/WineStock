// 本文件拥有主题偏好的本机持久化、Vue 只读状态、系统媒体查询和页面/平台外观同步；不拥有具体控件样式。
import { computed, readonly, ref, type Ref } from "vue";
import { setSystemChromeBaseDarkContent } from "../shell/systemChrome";
import {
  parseThemePreference,
  resolveTheme,
  THEME_STORAGE_KEY,
  type ResolvedTheme,
  type ThemePreference,
} from "./model";

const DARK_THEME_QUERY = "(prefers-color-scheme: dark)";
const SYSTEM_THEME_REFRESH_EVENT = "winestock:system-theme-refresh";

const preferenceState = ref<ThemePreference>("system");
const systemDarkState = ref(false);
const resolvedThemeState = computed<ResolvedTheme>(() =>
  resolveTheme(preferenceState.value, systemDarkState.value),
);

export const themePreference: Readonly<Ref<ThemePreference>> = readonly(preferenceState);
export const resolvedTheme: Readonly<Ref<ResolvedTheme>> = readonly(resolvedThemeState);

let initialized = false;
let mediaQuery: MediaQueryList | null = null;

/**
 * 在 Vue 挂载和任何异步启动工作前装配主题。
 * 初始化幂等；存储或媒体查询不可用时按 system/light 安全降级。
 */
export function initializeTheme(): void {
  if (initialized) {
    syncExternalAppearance();
    return;
  }

  preferenceState.value = readStoredPreference();
  mediaQuery = readSystemThemeQuery();
  systemDarkState.value = mediaQuery?.matches ?? false;
  applyPreferenceToRoot(preferenceState.value);
  syncExternalAppearance();

  addSystemThemeListener(mediaQuery);
  window.addEventListener(SYSTEM_THEME_REFRESH_EVENT, handleSystemThemeRefresh);
  window.addEventListener("storage", handleStorageChange);
  initialized = true;
}

/** 立即应用并尽力持久化用户选择；存储失败不撤销本会话中的主题。 */
export function setThemePreference(value: ThemePreference): void {
  applyPreferenceToRoot(value);
  preferenceState.value = value;
  if (value === "system") systemDarkState.value = mediaQuery?.matches ?? false;
  syncExternalAppearance();
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, value);
  } catch {
    // 隐私模式或存储被禁用时保留当前会话选择，下次启动按 system 回退。
  }
}

/** HMR 或宿主销毁时解除全局监听，防止开发期重复订阅。 */
export function disposeThemeRuntime(): void {
  if (!initialized) return;
  removeSystemThemeListener(mediaQuery);
  window.removeEventListener(SYSTEM_THEME_REFRESH_EVENT, handleSystemThemeRefresh);
  window.removeEventListener("storage", handleStorageChange);
  mediaQuery = null;
  initialized = false;
}

function readStoredPreference(): ThemePreference {
  try {
    return parseThemePreference(window.localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return "system";
  }
}

function readSystemThemeQuery(): MediaQueryList | null {
  try {
    return typeof window.matchMedia === "function" ? window.matchMedia(DARK_THEME_QUERY) : null;
  } catch {
    return null;
  }
}

function addSystemThemeListener(query: MediaQueryList | null): void {
  if (!query) return;
  if (typeof query.addEventListener === "function") {
    query.addEventListener("change", handleSystemThemeChange);
    return;
  }
  query.addListener(handleSystemThemeChange);
}

function removeSystemThemeListener(query: MediaQueryList | null): void {
  if (!query) return;
  if (typeof query.removeEventListener === "function") {
    query.removeEventListener("change", handleSystemThemeChange);
    return;
  }
  query.removeListener(handleSystemThemeChange);
}

function handleSystemThemeChange(event: MediaQueryListEvent): void {
  updateSystemTheme(event.matches);
}

/** Android WebView 可能只更新 media query 结果而不派发 change，宿主配置回调后主动重读。 */
function handleSystemThemeRefresh(): void {
  updateSystemTheme(mediaQuery?.matches ?? false);
}

function updateSystemTheme(matches: boolean): void {
  systemDarkState.value = matches;
  // 手动主题的解析结果不变，但 Android 配置更新可能重写系统栏，仍需重放当前外观。
  syncExternalAppearance();
}

function handleStorageChange(event: StorageEvent): void {
  if (event.key !== THEME_STORAGE_KEY && event.key !== null) return;
  const preference = parseThemePreference(event.newValue);
  applyPreferenceToRoot(preference);
  preferenceState.value = preference;
  if (preference === "system") systemDarkState.value = mediaQuery?.matches ?? false;
  syncExternalAppearance();
}

function applyPreferenceToRoot(preference: ThemePreference): void {
  document.documentElement.dataset.theme = preference;
}

function syncExternalAppearance(): void {
  const theme = resolvedThemeState.value;
  setSystemChromeBaseDarkContent(theme === "dark", preferenceState.value);
  const meta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
  const pageColor = getComputedStyle(document.documentElement)
    .getPropertyValue("--color-page")
    .trim();
  if (meta && pageColor) meta.content = pageColor;
}
