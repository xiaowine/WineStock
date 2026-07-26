// 本文件拥有 frontend 路由切换的稳定等待状态和页面资源加载失败提示；它不执行鉴权判断或决定页面内容。
import { computed, effectScope, ref, watch, type EffectScope } from "vue";
import { isNavigationFailure, NavigationFailureType, type Router } from "vue-router";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";

/** 路由切换等待提示复用异步状态切换规范的默认计时。 */
const ROUTE_PENDING_TIMING = {
  showDelayMs: 200,
  minimumVisibleMs: 350,
} as const;

const navigationPending = ref(false);
const pendingTargetName = ref<string | null>(null);
const indicatorVisible = ref(false);

let installed = false;
let scope: EffectScope | null = null;

/** 路由切换等待提示是否应当可见；已按延迟显示与最短展示稳定。 */
export const routeNavigationIndicatorVisible = computed(() => indicatorVisible.value);

/**
 * 等待提示可见期间的导航目标路由名称，用于侧栏乐观高亮。
 * 目标名称在最短展示期内保持不变，避免导航完成瞬间高亮闪断。
 */
export const pendingNavigationRouteName = computed(() =>
  indicatorVisible.value ? pendingTargetName.value : null,
);

/** 安装一次路由切换等待状态追踪和懒加载 chunk 失败提示。 */
export function installNavigationPendingTracking(router: Router): void {
  if (installed) {
    return;
  }
  installed = true;

  scope = effectScope(true);
  scope.run(() => {
    const stableVisible = useStablePendingIndicator(navigationPending, ROUTE_PENDING_TIMING);
    watch(
      stableVisible,
      (visible) => {
        indicatorVisible.value = visible;
      },
      { immediate: true, flush: "sync" },
    );
  });

  router.beforeEach((to) => {
    // 即时状态必须立刻更新；重定向的每一跳都会把目标名称修正为最终页面。
    navigationPending.value = true;
    pendingTargetName.value = typeof to.name === "string" ? to.name : null;
  });

  router.afterEach((_to, _from, failure) => {
    // 被更新导航取代的旧导航结束时，新导航仍在进行，不能提前清除等待状态。
    if (failure && isNavigationFailure(failure, NavigationFailureType.cancelled)) {
      return;
    }
    navigationPending.value = false;
  });

  router.onError((error, to) => {
    navigationPending.value = false;
    if (!isModuleLoadError(error)) {
      console.error("路由切换失败", error);
      return;
    }

    const targetPath = to.fullPath;
    notice.error("页面加载失败", {
      detail: "网络不稳定，未能下载页面资源。点击此处重试。",
      durationMs: 8_000,
      onClick: () => {
        // Chromium 会缓存失败的模块请求，重新 push 可能立即再次失败；
        // 整页刷新保证重新拉取资源，hash history 下先写入目标路径。
        window.location.hash = targetPath;
        window.location.reload();
      },
    });
  });
}

/** 识别懒加载页面 chunk 或其 CSS 在各浏览器下的网络加载失败。 */
function isModuleLoadError(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false;
  }
  return /Failed to fetch dynamically imported module|error loading dynamically imported module|Importing a module script failed|Unable to preload CSS/i.test(
    error.message,
  );
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    scope?.stop();
    scope = null;
  });
}
