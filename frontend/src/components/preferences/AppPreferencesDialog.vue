<!--
  本组件拥有账户弹层入口的「偏好设置」Dialog：承载主题、窗口行为和匿名数据收集等本机偏好。
  改动即时生效并持久化；它不拥有偏好存储格式、主题运行时或采集 SDK 生命周期细节。
-->
<template>
  <ModalDialog
    :open="open"
    title="偏好设置"
    description="只影响这台设备上的使用体验，更改立即保存。"
    @close="emit('close')"
  >
    <div class="app-preferences">
      <section class="app-preferences__section" aria-labelledby="preferences-appearance-title">
        <h3 id="preferences-appearance-title">外观</h3>
        <ThemePreferenceSelector />
      </section>

      <section class="app-preferences__section" aria-labelledby="preferences-contact-title">
        <h3 id="preferences-contact-title">联系与反馈</h3>
        <label class="consent-toggle">
          <input
            v-model="contactVisible"
            type="checkbox"
            name="preferences-contact-visible"
            @change="handleContactVisibilityChange"
          />
          <span class="consent-toggle__copy">
            <strong>显示联系与反馈入口</strong>
            <small>在总览页和账户菜单中显示联系作者入口。</small>
          </span>
        </label>
      </section>

      <section
        v-if="isDesktop"
        class="app-preferences__section"
        aria-labelledby="preferences-window-title"
      >
        <h3 id="preferences-window-title">窗口</h3>
        <fieldset
          class="window-close-preference"
          :disabled="
            desktopPreferencesLoading || desktopPreferencesSaving || !desktopPreferencesLoaded
          "
        >
          <legend>关闭窗口时</legend>
          <label
            class="window-close-preference__option"
            :class="{
              'window-close-preference__option--selected': closeBehavior === 'minimize-to-tray',
              'window-close-preference__option--disabled':
                desktopPreferencesLoading || desktopPreferencesSaving || !desktopPreferencesLoaded,
            }"
          >
            <input
              v-model="closeBehavior"
              type="radio"
              name="preferences-close-behavior"
              value="minimize-to-tray"
              @change="handleCloseBehaviorChange"
            />
            <span class="window-close-preference__indicator" aria-hidden="true"></span>
            <span class="window-close-preference__copy">
              <strong>最小化到系统托盘</strong>
              <small>应用继续运行，可从系统托盘重新打开。</small>
            </span>
          </label>
          <label
            class="window-close-preference__option"
            :class="{
              'window-close-preference__option--selected': closeBehavior === 'exit-application',
              'window-close-preference__option--disabled':
                desktopPreferencesLoading || desktopPreferencesSaving || !desktopPreferencesLoaded,
            }"
          >
            <input
              v-model="closeBehavior"
              type="radio"
              name="preferences-close-behavior"
              value="exit-application"
              @change="handleCloseBehaviorChange"
            />
            <span class="window-close-preference__indicator" aria-hidden="true"></span>
            <span class="window-close-preference__copy">
              <strong>退出应用</strong>
              <small>关闭窗口时停止本机服务并退出 WineStock。</small>
            </span>
          </label>
        </fieldset>
      </section>

      <section
        v-if="isDesktop"
        class="app-preferences__section"
        aria-labelledby="preferences-startup-title"
      >
        <h3 id="preferences-startup-title">启动</h3>
        <label
          class="consent-toggle startup-preference"
          :class="{ 'startup-preference--disabled': desktopPreferencesUnavailable }"
        >
          <input
            v-model="autostartEnabled"
            type="checkbox"
            name="preferences-autostart"
            :disabled="desktopPreferencesUnavailable"
            @change="handleDesktopPreferencesChange"
          />
          <span class="consent-toggle__copy">
            <strong>开机自启</strong>
            <small>系统登录后自动启动 WineStock。</small>
          </span>
        </label>
        <label
          class="consent-toggle startup-preference"
          :class="{
            'startup-preference--disabled': desktopPreferencesUnavailable || !autostartEnabled,
          }"
        >
          <input
            v-model="autostartSilent"
            type="checkbox"
            name="preferences-autostart-silent"
            :disabled="desktopPreferencesUnavailable || !autostartEnabled"
            @change="handleDesktopPreferencesChange"
          />
          <span class="consent-toggle__copy">
            <strong>静默启动</strong>
            <small>随系统启动时保持窗口隐藏，可从系统托盘重新打开。</small>
          </span>
        </label>
      </section>

      <section class="app-preferences__section" aria-label="数据收集">
        <h3>数据收集</h3>
        <label class="consent-toggle">
          <input
            v-model="telemetryEnabled"
            type="checkbox"
            name="preferences-telemetry"
            @change="handleTelemetryChange"
          />
          <span class="consent-toggle__copy">
            <strong>发送匿名使用数据</strong>
            <small
              >帮助开发者定位和排查问题；不包含库存内容与账户信息，仅在联网时生效。分析服务由
              Microsoft Clarity 提供。</small
            >
          </span>
        </label>
        <p class="app-preferences__policy">
          <a href="#" @click.prevent="openTelemetryPolicy">查看 Microsoft 隐私声明</a>
        </p>
      </section>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">关闭</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import ModalDialog from "../ModalDialog.vue";
import { startTelemetryIfConsented, stopTelemetry } from "../../telemetry/clarity";
import {
  TELEMETRY_POLICY_URL,
  readTelemetryConsent,
  saveTelemetryConsent,
} from "../../telemetry/consent";
import {
  getDesktopPreferences,
  openExternal,
  runtimeSnapshot,
  setDesktopPreferences,
} from "../../shell/runtime";
import { defaultDesktopPreferences } from "../../shell/contract";
import type { DesktopCloseBehavior, DesktopPreferences } from "../../shell/contract";
import { notice } from "../../notices/notice";
import ThemePreferenceSelector from "./ThemePreferenceSelector.vue";
import { contactEntryVisible, setContactEntryVisible } from "../../contact/contactPreferences";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const telemetryEnabled = ref(false);
const contactVisible = ref(contactEntryVisible.value);
const closeBehavior = ref<DesktopCloseBehavior>(defaultDesktopPreferences.closeBehavior);
const autostartEnabled = ref(defaultDesktopPreferences.autostartEnabled);
const autostartSilent = ref(defaultDesktopPreferences.autostartSilent);
const desktopPreferencesLoading = ref(false);
const desktopPreferencesLoaded = ref(false);
const desktopPreferencesSaving = ref(false);
const isDesktop = computed(() => runtimeSnapshot.value?.platform === "desktop");
const desktopPreferencesUnavailable = computed(
  () =>
    desktopPreferencesLoading.value ||
    desktopPreferencesSaving.value ||
    !desktopPreferencesLoaded.value,
);
let desktopPreferencesRequest = 0;

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    telemetryEnabled.value = readTelemetryConsent() === true;
    contactVisible.value = contactEntryVisible.value;
    void loadDesktopPreferences();
  },
);

function handleContactVisibilityChange(): void {
  setContactEntryVisible(contactVisible.value);
}

async function loadDesktopPreferences(): Promise<void> {
  const request = ++desktopPreferencesRequest;
  desktopPreferencesLoading.value = true;
  desktopPreferencesLoaded.value = false;
  try {
    const preferences = await getDesktopPreferences();
    if (request !== desktopPreferencesRequest) return;
    if (!preferences) return;
    closeBehavior.value = preferences.closeBehavior;
    autostartEnabled.value = preferences.autostartEnabled;
    autostartSilent.value = preferences.autostartSilent;
    desktopPreferencesLoaded.value = true;
  } catch (error) {
    if (request !== desktopPreferencesRequest) return;
    notice.error("无法读取桌面偏好", { detail: errorMessage(error) });
  } finally {
    if (request === desktopPreferencesRequest) {
      desktopPreferencesLoading.value = false;
    }
  }
}

async function handleCloseBehaviorChange(): Promise<void> {
  const preferences = await saveDesktopPreferences();
  if (
    preferences?.closeBehavior === "exit-application" &&
    runtimeSnapshot.value?.config.mode === "server-mode"
  ) {
    notice.warning("关闭窗口将停止服务", {
      detail: "当前运行方式允许他人连接，关闭窗口后将无法连接。",
    });
  }
}

async function handleDesktopPreferencesChange(): Promise<void> {
  await saveDesktopPreferences();
}

async function saveDesktopPreferences(): Promise<DesktopPreferences | null> {
  desktopPreferencesSaving.value = true;
  try {
    const preferences = await setDesktopPreferences({
      version: 1,
      closeBehavior: closeBehavior.value,
      autostartEnabled: autostartEnabled.value,
      autostartSilent: autostartSilent.value,
    });
    if (preferences) {
      closeBehavior.value = preferences.closeBehavior;
      autostartEnabled.value = preferences.autostartEnabled;
      autostartSilent.value = preferences.autostartSilent;
    }
    return preferences;
  } catch (error) {
    notice.error("无法保存桌面偏好", { detail: errorMessage(error) });
    await loadDesktopPreferences();
    return null;
  } finally {
    desktopPreferencesSaving.value = false;
  }
}

function handleTelemetryChange(): void {
  saveTelemetryConsent(telemetryEnabled.value);
  if (telemetryEnabled.value) {
    // 本会话内停过的采集会静默留待下次启动恢复，偏好本身已即时保存。
    startTelemetryIfConsented();
    return;
  }
  stopTelemetry();
}

/** 打开 Microsoft 隐私声明；平台不支持外链能力时静默忽略。 */
function openTelemetryPolicy(): void {
  void openExternal(TELEMETRY_POLICY_URL).catch(() => undefined);
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "请稍后重试";
}
</script>

<style scoped lang="scss">
@use "../../styles/foundation/mixins" as mixins;

/* 同意开关卡片复用 shared/_consent-toggle.scss；这里只保留偏好分节、主题分段控件与政策链接。 */
.app-preferences {
  display: grid;
  gap: 18px;
}

.app-preferences__section {
  display: grid;
  gap: 10px;

  & + & {
    padding-top: 18px;
    border-top: 1px solid var(--color-border);
  }

  h3 {
    margin: 0;
    font-size: 13px;
  }
}

.app-preferences__policy {
  margin: 0;
  font-size: 12px;

  a {
    color: var(--color-accent);
    font-weight: 650;
  }

  a:hover {
    color: var(--color-accent-strong);
    text-decoration: underline;
  }
}

.startup-preference--disabled {
  cursor: not-allowed;
  opacity: 0.66;
}

.window-close-preference {
  display: grid;
  min-width: 0;
  gap: 4px;
  margin: 0;
  padding: 0;
  border: 0;
}

.window-close-preference legend {
  margin-bottom: 2px;
  padding: 0;
  color: var(--color-muted);
  font-size: 12px;
}

.window-close-preference__option {
  position: relative;
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr);
  gap: 10px;
  min-width: 0;
  padding: 12px 11px;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  color: var(--color-muted);
  cursor: pointer;
  transition:
    border-color var(--motion-duration-fast) var(--motion-ease-standard),
    background var(--motion-duration-fast) var(--motion-ease-standard);

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  &--selected {
    border-color: var(--color-focus-ring);
    background: var(--color-accent-soft);

    .window-close-preference__indicator {
      border-color: var(--color-accent);

      &::after {
        position: absolute;
        inset: 3px;
        border-radius: 50%;
        background: var(--color-accent);
        content: "";
      }
    }
  }

  &--disabled {
    cursor: not-allowed;
    opacity: 0.66;
  }

  &:has(input:focus-visible) {
    @include mixins.focus-ring(var(--color-focus-ring), 2px);
  }
}

.window-close-preference__indicator {
  position: relative;
  width: 16px;
  height: 16px;
  margin-top: 2px;
  border: 1px solid var(--color-border-strong);
  border-radius: 50%;
  background: var(--color-surface);
}

.window-close-preference__copy {
  display: grid;
  min-width: 0;
  gap: 3px;
}

.window-close-preference__option strong,
.window-close-preference__option small {
  overflow-wrap: anywhere;
}

.window-close-preference__option strong {
  color: var(--color-text);
  font-size: 14px;
  font-weight: 670;
}

.window-close-preference__option small {
  color: var(--color-muted);
  font-size: 12px;
  line-height: 1.45;
}

@include mixins.hover-capable {
  .window-close-preference__option:not(.window-close-preference__option--disabled):hover {
    border-color: var(--color-border);
    background: var(--color-surface-raised);
  }

  .window-close-preference__option--selected:not(.window-close-preference__option--disabled):hover {
    border-color: var(--color-accent-border);
    background: var(--color-accent-soft);
  }
}
</style>
