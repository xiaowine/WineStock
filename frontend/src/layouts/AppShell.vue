<!--
  本文件拥有已登录应用区域的稳定响应式应用框架，属于 frontend 布局层。
  它通过 CSS 调整桌面和移动排布，始终只保留一个路由出口；不拥有平台 Shell 生命周期。
-->
<template>
  <div class="app-shell" data-shell="responsive">
    <header class="app-topbar">
      <div class="app-topbar__brand">
        <button
          ref="navTrigger"
          class="icon-button app-mobile-nav-trigger"
          type="button"
          aria-label="打开导航"
          aria-controls="app-navigation-pane"
          :aria-expanded="navOpen"
          @click="openNavigation"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M4 7h16M4 12h16M4 17h16" />
          </svg>
        </button>

        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
          <span v-if="APP_STAGE_LABEL" class="brand-badge">{{ APP_STAGE_LABEL }}</span>
        </div>

        <div class="app-topbar__context">
          <BrandMark class="app-topbar__context-mark" />
          <span class="app-topbar__context-copy">
            <small
              >WineStock<span v-if="APP_STAGE_LABEL" class="brand-badge">{{
                APP_STAGE_LABEL
              }}</span></small
            >
            <strong>{{ pageTitle }}</strong>
          </span>
        </div>
      </div>

      <div v-if="userDisplayName || silentLocalMode" class="app-account">
        <button
          ref="accountTrigger"
          class="app-account__trigger"
          type="button"
          :aria-expanded="accountMenuOpen"
          aria-controls="app-account-popover"
          :aria-label="
            userDisplayName ? `查看当前用户 ${userDisplayName} 的账户与本机选项` : '查看本机选项'
          "
          @click="toggleAccountMenu"
        >
          <AccountUserSummary
            :initials="accountInitials"
            :display-name="accountDisplayName"
            :subtitle="runtimeModeLabel"
          />
        </button>
        <button
          v-if="accountMenuOpen"
          class="account-popover-backdrop"
          type="button"
          aria-label="关闭账户与本机选项"
          @click="closeAccountMenu"
        />
        <Transition name="account-popover">
          <AccountPopover
            v-if="accountMenuOpen"
            id="app-account-popover"
            :initials="accountInitials"
            :display-name="accountDisplayName"
            :subtitle="runtimeModeLabel"
            :show-user-summary="Boolean(userDisplayName)"
            :show-lan-access="lanAccessUrls.length > 0"
            :show-contact="contactEntryVisible"
            :show-donation="donationEnabled"
            :show-logout="!silentLocalMode"
            :logout-error="logoutError"
            :is-logging-out="isLoggingOut"
            @preferences="openPreferencesDialog"
            @runtime-settings="openRuntimeSettings"
            @lan-access="openLanAccessDialog"
            @contact="openContactDialog"
            @donation="openDonationDialog"
            @logout="handleLogout"
          />
        </Transition>
      </div>
    </header>

    <div class="app-workspace">
      <aside
        id="app-navigation-pane"
        v-overlay-scrollbar
        class="app-navigation-pane"
        :class="{ 'app-navigation-pane--open': navOpen }"
        aria-label="导航面板"
      >
        <div class="app-navigation-pane__header">
          <div class="app-navigation-pane__brand">
            <BrandMark />
            <div>
              <strong
                >WineStock<span v-if="APP_STAGE_LABEL" class="brand-badge">{{
                  APP_STAGE_LABEL
                }}</span></strong
              >
              <span>应用导航</span>
            </div>
          </div>
          <button class="icon-button" type="button" aria-label="关闭导航" @click="closeNavigation">
            <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="m6 6 12 12M18 6 6 18" />
            </svg>
          </button>
        </div>

        <AppNavigationList
          :items="visibleNavigation"
          :pending-route-name="pendingNavigationRouteName"
          @navigate="closeNavigation"
        />
      </aside>

      <main v-overlay-scrollbar class="app-content-pane" aria-label="主内容">
        <RouteContentView />
        <!-- 触底安全区占位：必须是真实节点以撑开 scrollHeight，内容可仍延伸到导航栏下。 -->
        <div class="app-content-pane__end-inset" aria-hidden="true" />
      </main>
    </div>

    <Transition name="mobile-nav-backdrop">
      <button
        v-if="navOpen"
        class="mobile-nav-backdrop"
        type="button"
        aria-label="关闭导航"
        @click="closeNavigation"
      />
    </Transition>

    <LanAccessDialog
      :open="lanAccessDialogOpen"
      :urls="lanAccessUrls"
      @close="closeLanAccessDialog"
    />

    <AppPreferencesDialog :open="preferencesDialogOpen" @close="preferencesDialogOpen = false" />

    <ContactDialog :open="contactDialogOpen" @close="closeContactDialog" />

    <DonationDialog
      :open="donationDialogOpen"
      :automatic="donationDialogAutomatic"
      @close="closeDonationDialog"
      @snooze="snoozeDonationDialog"
      @disable="disableDonationDialog"
    />

    <RuntimeSettingsDialog
      v-if="runtimeSettingsDialogOpen"
      embedded
      @close="closeRuntimeSettings"
    />
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { useRoute } from "vue-router";
import { authSession, localSilentAuthActive } from "../auth/session";
import AccountPopover from "../components/AccountPopover.vue";
import AccountUserSummary from "../components/AccountUserSummary.vue";
import AppNavigationList from "../components/AppNavigationList.vue";
import BrandMark from "../components/BrandMark.vue";
import RouteContentView from "../components/RouteContentView.vue";
import DonationDialog from "../components/donation/DonationDialog.vue";
import LanAccessDialog from "../components/runtime/LanAccessDialog.vue";
import AppPreferencesDialog from "../components/preferences/AppPreferencesDialog.vue";
import ContactDialog from "../contact/ContactDialog.vue";
import { closeContactDialog, contactDialogOpen, openContactDialog } from "../contact/contactDialog";
import { contactEntryVisible } from "../contact/contactPreferences";
import { useAccountPopover } from "../composables/useAccountPopover";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { useShellLogout } from "../composables/useShellLogout";
import { NativeBackPriority } from "../navigation/nativeBack";
import { schedulePrefetchAppPages } from "../router/appPageLoaders";
import { getVisibleAppNavigation } from "../router/navigation";
import { pendingNavigationRouteName } from "../router/navigationPending";
import { getUsableLanAccessUrls } from "../shell/lanAccess";
import { runtimeSnapshot } from "../shell/runtime";
import { donationEnabled } from "../donation/config";
import { donationController } from "../donation/controller";
import type { DonationMilestone } from "../donation/model";
import { readDonationStartupTestParams } from "../donation/testing";

/** 品牌名后的前端发布阶段徽标文案，来自 package.json `appStage`；为空时徽标整体隐藏。 */
const APP_STAGE_LABEL = __APP_STAGE_LABEL__;
const DESKTOP_QUERY = "(min-width: 768px)";
const RuntimeSettingsDialog = defineAsyncComponent(
  () => import("../pages/RuntimeSettingsPage.vue"),
);
const navOpen = ref(false);
const navTrigger = ref<HTMLButtonElement | null>(null);
const accountTrigger = ref<HTMLButtonElement | null>(null);
const lanAccessDialogOpen = ref(false);
const preferencesDialogOpen = ref(false);
const runtimeSettingsDialogOpen = ref(false);
const donationDialogOpen = ref(false);
const donationDialogAutomatic = ref(false);
const route = useRoute();
const {
  accountMenuOpen,
  closeAccountMenu,
  toggleAccountMenu: toggleAccountPopover,
} = useAccountPopover();
const { handleLogout, isLoggingOut, logoutError } = useShellLogout();
const pageTitle = computed(() => route.meta.title);
// 本机静默免登录模式：保留用户名展示，但隐藏退出登录并收起不适用的账户管理导航。
const silentLocalMode = computed(() => localSilentAuthActive.value);
const visibleNavigation = computed(() =>
  getVisibleAppNavigation(authSession.value?.user.permissions, {
    localSilentMode: silentLocalMode.value,
  }),
);
const userDisplayName = computed(() => authSession.value?.user.username ?? "");
const accountDisplayName = computed(() => userDisplayName.value || "本机");
const runtimeModeLabel = computed(() => {
  switch (runtimeSnapshot.value?.config.mode) {
    case "self-hosted":
      return "本机模式";
    case "server-mode":
      return "服务器模式";
    case "client-only":
    case "connect-to-remote":
      return "远程连接";
    default:
      return "运行模式";
  }
});
const accountInitials = computed(() =>
  Array.from(accountDisplayName.value.trim()).slice(0, 2).join("").toUpperCase(),
);
const lanAccessUrls = computed(() => getUsableLanAccessUrls(runtimeSnapshot.value));
let desktopMediaQuery: MediaQueryList | undefined;
let stopDonationSubscription: (() => void) | undefined;
let donationPromptTimer: ReturnType<typeof setTimeout> | undefined;

useNativeBackHandler({
  id: "app-navigation-drawer",
  active: navOpen,
  priority: NativeBackPriority.Drawer,
  handle: () => {
    if (!navOpen.value) return { handled: false };
    closeNavigation();
    void nextTick(() => navTrigger.value?.focus());
    return { handled: true, reason: "drawer" };
  },
});

function openNavigation(): void {
  navTrigger.value?.blur();
  closeAccountMenu();
  navOpen.value = true;
}

function closeNavigation(): void {
  navOpen.value = false;
}

function toggleAccountMenu(): void {
  closeNavigation();
  logoutError.value = "";
  toggleAccountPopover();
}

async function openRuntimeSettings(): Promise<void> {
  closeAccountMenu();
  await nextTick();
  accountTrigger.value?.focus();
  runtimeSettingsDialogOpen.value = true;
}

function closeRuntimeSettings(): void {
  runtimeSettingsDialogOpen.value = false;
}

async function openPreferencesDialog(): Promise<void> {
  closeAccountMenu();
  await nextTick();
  accountTrigger.value?.focus();
  preferencesDialogOpen.value = true;
}

async function openLanAccessDialog(): Promise<void> {
  closeAccountMenu();
  await nextTick();
  accountTrigger.value?.focus();
  lanAccessDialogOpen.value = true;
}

function closeLanAccessDialog(): void {
  lanAccessDialogOpen.value = false;
}

async function openDonationDialog(): Promise<void> {
  if (!donationEnabled) return;
  closeAccountMenu();
  await nextTick();
  accountTrigger.value?.focus();
  donationDialogAutomatic.value = false;
  donationDialogOpen.value = true;
}

function closeDonationDialog(): void {
  if (donationDialogAutomatic.value) {
    donationController.snooze(14);
  }
  donationDialogOpen.value = false;
  donationDialogAutomatic.value = false;
}

function snoozeDonationDialog(): void {
  donationController.snooze(30);
  donationDialogOpen.value = false;
  donationDialogAutomatic.value = false;
}

function disableDonationDialog(): void {
  donationController.disableAutoPrompt();
  donationDialogOpen.value = false;
  donationDialogAutomatic.value = false;
}

function scheduleDonationPrompt(milestone: DonationMilestone, delay = 1_200): void {
  if (!donationEnabled || donationPromptTimer !== undefined) return;
  donationPromptTimer = setTimeout(() => {
    donationPromptTimer = undefined;
    if (donationDialogOpen.value || document.querySelector(".modal-layer")) {
      scheduleDonationPrompt(milestone, 1_000);
      return;
    }
    donationDialogAutomatic.value = true;
    donationController.markPromptShown(milestone);
    donationDialogOpen.value = true;
  }, delay);
}

/** 断点变化只关闭临时 Drawer，不切换或重挂载应用框架和路由页面。 */
function handleDesktopQueryChange(event: MediaQueryListEvent): void {
  if (event.matches) closeNavigation();
}

/** Escape 只关闭移动导航；账户弹层由共享 composable 处理。 */
function handleEscape(event: KeyboardEvent): void {
  if (event.key === "Escape") closeNavigation();
}

watch(lanAccessUrls, (urls) => {
  if (!urls.length) closeLanAccessDialog();
});

watch(
  () => route.fullPath,
  () => {
    closeNavigation();
    closeLanAccessDialog();
    closeRuntimeSettings();
    if (donationDialogOpen.value) closeDonationDialog();
  },
);

onMounted(() => {
  document.addEventListener("keydown", handleEscape);
  desktopMediaQuery = window.matchMedia(DESKTOP_QUERY);
  desktopMediaQuery.addEventListener("change", handleDesktopQueryChange);
  stopDonationSubscription = donationController.subscribe(({ milestone }) => {
    scheduleDonationPrompt(milestone);
  });
  const donationTestStartup = readDonationStartupTestParams(
    import.meta.env.DEV ? window.location.search : "",
    import.meta.env.DEV ? window.location.hash : "",
    window.__WINESTOCK_DONATION_TEST__,
  );
  if (donationTestStartup) donationController.recordTestStartup(donationTestStartup);
  donationController.recordAppOpenOnce();
  if (donationTestStartup) donationController.notifyPendingPrompt();
  // 空闲预取当前权限可见的页面 chunk，弱网下点击导航直接命中模块缓存。
  schedulePrefetchAppPages(visibleNavigation.value.map((item) => item.routeName));
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", handleEscape);
  desktopMediaQuery?.removeEventListener("change", handleDesktopQueryChange);
  stopDonationSubscription?.();
  if (donationPromptTimer !== undefined) clearTimeout(donationPromptTimer);
});
</script>

<style lang="scss" src="./AppShell.scss"></style>
