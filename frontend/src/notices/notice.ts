// 本文件拥有 frontend 全局 Notice 状态、详情、点击回调、倒计时和调用 API；它不决定具体页面布局或业务错误文案。
import { readonly, ref } from "vue";

const DEFAULT_DURATION_MS = 5_000;
const ERROR_DURATION_MS = 7_000;
const MIN_DURATION_MS = 1_500;
const MAX_VISIBLE_NOTICES = 5;
const TICK_INTERVAL_MS = 50;

/** Notice 视觉和无障碍语义类型。 */
export type NoticeTone = "success" | "info" | "warning" | "error";

/** 各语义快捷方法允许配置的展示和交互行为。 */
export interface NoticeOptions {
  /** 对结果、影响或下一步的补充说明。 */
  detail?: string;
  /** 自动消失时间，单位毫秒，最短 1500 毫秒。 */
  durationMs?: number;
  /** 点击 Notice 主体时执行的业务回调。 */
  onClick?: () => void;
  /** 点击回调执行前是否关闭当前 Notice，默认关闭。 */
  dismissOnClick?: boolean;
}

/** 底层创建入口接收的完整 Notice 数据。 */
export interface NoticeRequest extends NoticeOptions {
  /** 提示标题，承担主要操作结果。 */
  title: string;
  /** 提示类型，默认使用 `info`。 */
  tone?: NoticeTone;
}

/** Notice 容器消费的只读状态。 */
export interface NoticeItem {
  /** 当前提示的稳定实例 ID。 */
  id: string;
  /** 面向用户的主要提示标题。 */
  title: string;
  /** 面向用户的补充说明。 */
  detail?: string;
  /** 提示类型。 */
  tone: NoticeTone;
  /** 初始展示时长，单位毫秒。 */
  durationMs: number;
  /** 当前剩余时长，单位毫秒。 */
  remainingMs: number;
  /** 最近一次倒计时结算时间。 */
  updatedAt: number;
  /** 鼠标悬浮或键盘焦点进入时暂停倒计时。 */
  paused: boolean;
  /** 点击 Notice 主体时执行的业务回调。 */
  onClick?: () => void;
  /** 点击时是否先关闭当前 Notice。 */
  dismissOnClick: boolean;
}

const mutableNotices = ref<NoticeItem[]>([]);
let nextNoticeId = 0;
let ticker: ReturnType<typeof setInterval> | null = null;

/** 当前全局 Notice 列表。 */
export const notices = readonly(mutableNotices);

/** 创建一条自动消失的 Notice，并返回实例 ID。 */
export function showNotice(request: NoticeRequest): string {
  const title = request.title.trim();
  if (!title) {
    throw new Error("Notice 标题不能为空");
  }
  const detail = request.detail?.trim() || undefined;
  const tone = request.tone ?? "info";
  const durationMs = Math.max(
    MIN_DURATION_MS,
    request.durationMs ?? (tone === "error" ? ERROR_DURATION_MS : DEFAULT_DURATION_MS),
  );
  const now = Date.now();
  const id = `notice-${++nextNoticeId}`;
  const notice: NoticeItem = {
    id,
    title,
    detail,
    tone,
    durationMs,
    remainingMs: durationMs,
    updatedAt: now,
    paused: false,
    onClick: request.onClick,
    dismissOnClick: request.dismissOnClick ?? true,
  };

  mutableNotices.value = [...mutableNotices.value, notice].slice(-MAX_VISIBLE_NOTICES);
  ensureTicker();
  return id;
}

/** 手动关闭指定 Notice。 */
export function dismissNotice(id: string): void {
  mutableNotices.value = mutableNotices.value.filter((notice) => notice.id !== id);
  stopTickerWhenIdle();
}

/** 暂停指定 Notice 倒计时。 */
export function pauseNotice(id: string): void {
  updatePausedState(id, true);
}

/** 继续指定 Notice 倒计时。 */
export function resumeNotice(id: string): void {
  updatePausedState(id, false);
}

/** 执行指定 Notice 的点击回调，并按配置决定是否关闭。 */
export function activateNotice(id: string): void {
  const item = mutableNotices.value.find((notice) => notice.id === id);
  if (!item?.onClick) {
    return;
  }

  const callback = item.onClick;
  if (item.dismissOnClick) {
    dismissNotice(id);
  }
  callback();
}

/** 清空全部 Notice；主要用于退出或测试清理。 */
export function clearNotices(): void {
  mutableNotices.value = [];
  stopTicker();
}

/** 业务页面使用的简洁 Notice 调用入口。 */
export const notice = {
  success(title: string, options: NoticeOptions = {}): string {
    return showNotice({ title, tone: "success", ...options });
  },
  info(title: string, options: NoticeOptions = {}): string {
    return showNotice({ title, tone: "info", ...options });
  },
  warning(title: string, options: NoticeOptions = {}): string {
    return showNotice({ title, tone: "warning", ...options });
  },
  error(title: string, options: NoticeOptions = {}): string {
    return showNotice({ title, tone: "error", ...options });
  },
} as const;

function ensureTicker(): void {
  if (ticker !== null) {
    return;
  }
  ticker = setInterval(updateCountdowns, TICK_INTERVAL_MS);
}

function updateCountdowns(): void {
  const now = Date.now();
  mutableNotices.value = mutableNotices.value.flatMap((notice) => {
    if (notice.paused) {
      return [{ ...notice, updatedAt: now }];
    }

    const remainingMs = Math.max(0, notice.remainingMs - Math.max(0, now - notice.updatedAt));
    return remainingMs === 0 ? [] : [{ ...notice, remainingMs, updatedAt: now }];
  });
  stopTickerWhenIdle();
}

function updatePausedState(id: string, paused: boolean): void {
  const now = Date.now();
  mutableNotices.value = mutableNotices.value.flatMap((notice) => {
    if (notice.id !== id) {
      return [notice];
    }

    const remainingMs = notice.paused
      ? notice.remainingMs
      : Math.max(0, notice.remainingMs - Math.max(0, now - notice.updatedAt));
    return remainingMs === 0 ? [] : [{ ...notice, remainingMs, updatedAt: now, paused }];
  });
  stopTickerWhenIdle();
}

function stopTickerWhenIdle(): void {
  if (mutableNotices.value.length === 0) {
    stopTicker();
  }
}

function stopTicker(): void {
  if (ticker !== null) {
    clearInterval(ticker);
    ticker = null;
  }
}

if (import.meta.hot) {
  import.meta.hot.dispose(stopTicker);
}
