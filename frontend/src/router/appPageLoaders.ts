// 本文件拥有应用壳一级路由页面组件的懒加载入口和空闲预取调度；它不声明路由表、权限或导航呈现。
import type { AppRouteName } from "./appRouteCatalog";

/** 应用壳页面组件的统一懒加载入口；路由表和预取必须复用同一份，保证命中同一个 chunk。 */
export const appPageLoaders = {
  dashboard: () => import("../pages/DashboardPage.vue"),
  items: () => import("../pages/ItemsPage.vue"),
  inbound: () => import("../pages/StockDraftPage.vue"),
  "inbound-orders": () => import("../pages/InboundOrdersPage.vue"),
  outbound: () => import("../pages/StockDraftPage.vue"),
  "outbound-orders": () => import("../pages/OutboundOrdersPage.vue"),
  "inbound-approvals": () => import("../pages/InboundApprovalsPage.vue"),
  "outbound-approvals": () => import("../pages/OutboundApprovalsPage.vue"),
  locations: () => import("../pages/LocationsPage.vue"),
  templates: () => import("../pages/TemplatesPage.vue"),
  substitutes: () => import("../pages/SubstitutesPage.vue"),
  events: () => import("../pages/EventsPage.vue"),
  users: () => import("../pages/UsersPage.vue"),
} satisfies Record<AppRouteName, () => Promise<unknown>>;

const prefetchPromises = new Map<AppRouteName, Promise<void>>();

/**
 * 在浏览器空闲时逐个预热指定页面的 chunk，弱网下点击导航即可命中模块缓存。
 * 预取失败静默放弃并允许下次调度重试，真正的失败反馈由路由错误处理兜底。
 */
export function schedulePrefetchAppPages(routeNames: readonly AppRouteName[]): void {
  const targets = [...routeNames];
  if (targets.length === 0) {
    return;
  }

  scheduleIdle(() => {
    void (async () => {
      // 顺序加载，避免预取并发挤占当前页面数据请求的带宽。
      for (const name of targets) {
        await prefetchAppPage(name);
      }
    })();
  });
}

function prefetchAppPage(name: AppRouteName): Promise<void> {
  let promise = prefetchPromises.get(name);
  if (!promise) {
    promise = appPageLoaders[name]().then(
      () => undefined,
      () => {
        prefetchPromises.delete(name);
      },
    );
    prefetchPromises.set(name, promise);
  }
  return promise;
}

function scheduleIdle(task: () => void): void {
  if (typeof window.requestIdleCallback === "function") {
    window.requestIdleCallback(() => task(), { timeout: 3_000 });
    return;
  }
  setTimeout(task, 1_000);
}
