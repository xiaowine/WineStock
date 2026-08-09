// 本文件拥有 frontend i18n 运行时：vue-i18n 实例、语言决议与持久化、标题翻译与文档标题同步。
// 语言包随前端静态打包，切换语言不发起任何网络请求；语言偏好属于本机偏好，不进 Shell 运行配置。
import { readonly, ref, watch, type Ref } from "vue";
import type { Router } from "vue-router";
import { createI18n } from "vue-i18n";
import { enUS } from "./locales/en-US";
import { zhCN } from "./locales/zh-CN";
import {
  LOCALE_STORAGE_KEY,
  parseLocale,
  resolveLocaleFromNavigator,
  type AppLocale,
} from "./model";
import type { MessageKeyPath } from "./schema";

export type { AppLocale } from "./model";

/** vue-i18n 实例；zh-CN 为默认语言与 fallback，en-US 经 satisfies 与 zh-CN 键对齐。 */
export const i18n = createI18n({
  legacy: false,
  locale: resolveInitialLocale(),
  fallbackLocale: "zh-CN",
  globalInjection: true,
  messages: {
    "zh-CN": zhCN,
    "en-US": enUS,
  },
});

/** 当前应用语言的只读响应式状态；切换由 setAppLocale 驱动并持久化。 */
const localeState = ref<AppLocale>(resolveInitialLocale());
export const appLocale: Readonly<Ref<AppLocale>> = readonly(localeState);

let initialized = false;

/**
 * 在 Vue 挂载和任何文案渲染前装配语言，并监听跨标签页语言同步。
 * 初始化幂等；非浏览器环境（node 测试）下安全跳过存储与 DOM 副作用。
 */
export function initializeI18n(): void {
  if (initialized) {
    return;
  }
  initialized = true;
  applyLocale(appLocale.value);
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.addEventListener("storage", handleStorageChange);
  } catch {
    // 存储事件不可用时保留本次会话内的语言选择。
  }
}

/** 立即应用并尽力持久化用户选择；存储失败不撤销本会话中的语言。 */
export function setAppLocale(locale: AppLocale): void {
  applyLocale(locale);
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  } catch {
    // 隐私模式或存储被禁用时保留当前会话选择，下次启动按系统语言回退。
  }
}

/** 翻译路由标题消息键；传入非键（如品牌名）时原样返回。 */
export function translateTitle(key: unknown): string {
  const value = String(key ?? "");
  return translateMessageOrNull(value) ?? value;
}

/** 按消息键翻译；key 不存在时返回 null，供错误/校验码兜底链路使用。 */
export function translateMessageOrNull(key: string): string | null {
  if (!i18n.global.te(key)) {
    return null;
  }
  return i18n.global.t(key as MessageKeyPath);
}

/** 路由变化或语言切换时按 meta.title（消息键）同步 document.title。 */
export function installDocumentTitleSync(router: Router): void {
  router.afterEach((to) => {
    applyDocumentTitle(to.meta.title);
  });
  watch(appLocale, () => {
    applyDocumentTitle(router.currentRoute.value.meta.title);
  });
}

function resolveInitialLocale(): AppLocale {
  const stored = readStoredLocale();
  return stored ?? resolveLocaleFromNavigator(readNavigatorLanguage());
}

function readStoredLocale(): AppLocale | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    return parseLocale(window.localStorage.getItem(LOCALE_STORAGE_KEY));
  } catch {
    return null;
  }
}

function readNavigatorLanguage(): string {
  return typeof navigator !== "undefined" ? navigator.language : "zh-CN";
}

function applyLocale(locale: AppLocale): void {
  localeState.value = locale;
  i18n.global.locale.value = locale;
  try {
    document.documentElement.lang = locale;
  } catch {
    // 非浏览器环境忽略。
  }
}

function handleStorageChange(event: StorageEvent): void {
  if (event.key !== LOCALE_STORAGE_KEY) {
    return;
  }
  const parsed = parseLocale(event.newValue);
  if (parsed && parsed !== appLocale.value) {
    applyLocale(parsed);
  }
}

function applyDocumentTitle(title: unknown): void {
  try {
    document.title = translateTitle(title);
  } catch {
    // 非浏览器环境忽略。
  }
}
