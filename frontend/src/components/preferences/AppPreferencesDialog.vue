<!--
  本组件拥有账户弹层入口的「偏好设置」Dialog：承载主题、窗口行为、更新检测和匿名数据收集等本机偏好。
  改动即时生效并持久化；它不拥有偏好存储格式、主题运行时或采集 SDK 生命周期细节。
-->
<template>
  <ModalDialog
    :open="open"
    :title="$t('components.preferences')"
    :description="$t('components.preferencesDescription')"
    @close="emit('close')"
  >
    <div class="app-preferences">
      <section class="app-preferences__section" aria-labelledby="preferences-appearance-title">
        <h3 id="preferences-appearance-title">{{ $t('components.appearance') }}</h3>
        <ThemePreferenceSelector />
      </section>

      <section class="app-preferences__section" aria-labelledby="preferences-language-title">
        <h3 id="preferences-language-title">{{ $t('components.languageHeading') }}</h3>
        <SegmentedPreferenceSelector
          :options="localeChoices"
          :model-value="selectedLocale"
          :ariaLabel="$t('components.appLanguage')"
          @change="handleLocaleChange"
        />
      </section>

      <section class="app-preferences__section" aria-labelledby="preferences-contact-title">
        <h3 id="preferences-contact-title">{{ $t('components.contactFeedback') }}</h3>
        <label class="consent-toggle">
          <input
            v-model="contactVisible"
            type="checkbox"
            name="preferences-contact-visible"
            @change="handleContactVisibilityChange"
          />
          <span class="consent-toggle__copy">
            <strong>{{ $t('components.showContactEntry') }}</strong>
            <small>{{ $t('components.contactEntryHint') }}</small>
          </span>
        </label>
      </section>

      <section
        v-if="updateSupported"
        class="app-preferences__section"
        aria-labelledby="preferences-update-title"
      >
        <h3 id="preferences-update-title">{{ $t('components.appUpdates') }}</h3>
        <label class="consent-toggle">
          <input
            type="checkbox"
            name="preferences-auto-update-check"
            :checked="autoUpdateCheckEnabled"
            @change="handleAutoUpdateCheckChange"
          />
          <span class="consent-toggle__copy">
            <strong>{{ $t('components.autoUpdateCheck') }}</strong>
            <small>{{ $t('components.autoUpdateCheckHint') }}</small>
          </span>
        </label>
        <div class="app-update__row">
          <div class="app-update__copy">
            <strong>{{ $t('components.currentVersion', { version: currentVersion }) }}</strong>
            <small v-if="availableUpdate">
              {{ $t('components.newVersionFound', { version: availableUpdate.latestVersion }) }}
            </small>
            <small v-else-if="updateChecked">{{ $t('components.upToDateChecked') }}</small>
            <small v-else>{{ $t('components.updateCheckIdleHint') }}</small>
          </div>
          <button
            class="secondary-button"
            type="button"
            :disabled="updateChecking"
            @click="handleUpdateCheck"
          >
            {{ updateChecking ? $t('components.updateChecking') : $t('components.updateCheck') }}
          </button>
        </div>
        <p v-if="updateError" class="app-update__error" role="alert">{{ updateError }}</p>
      </section>

      <section
        v-if="isDesktop"
        class="app-preferences__section"
        aria-labelledby="preferences-window-title"
      >
        <h3 id="preferences-window-title">{{ $t('components.windowSection') }}</h3>
        <fieldset
          class="window-close-preference"
          :disabled="
            desktopPreferencesLoading || desktopPreferencesSaving || !desktopPreferencesLoaded
          "
        >
          <legend>{{ $t('components.onCloseWindow') }}</legend>
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
              <strong>{{ $t('components.minimizeToTray') }}</strong>
              <small>{{ $t('components.minimizeToTrayHint') }}</small>
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
              <strong>{{ $t('components.exitApp') }}</strong>
              <small>{{ $t('components.exitAppHint') }}</small>
            </span>
          </label>
        </fieldset>
      </section>

      <section
        v-if="isDesktop"
        class="app-preferences__section"
        aria-labelledby="preferences-startup-title"
      >
        <h3 id="preferences-startup-title">{{ $t('components.startupSection') }}</h3>
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
            <strong>{{ $t('components.launchAtLogin') }}</strong>
            <small>{{ $t('components.launchAtLoginHint') }}</small>
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
            <strong>{{ $t('components.silentStart') }}</strong>
            <small>{{ $t('components.silentStartHint') }}</small>
          </span>
        </label>
        <label
          class="consent-toggle startup-preference"
          :class="{ 'startup-preference--disabled': desktopPreferencesUnavailable }"
        >
          <input
            v-model="webviewReclaimEnabled"
            type="checkbox"
            name="preferences-webview-reclaim"
            :disabled="desktopPreferencesUnavailable"
            @change="handleDesktopPreferencesChange"
          />
          <span class="consent-toggle__copy">
            <strong>{{ $t('components.reclaimWebview') }}</strong>
            <small>{{ $t('components.reclaimWebviewHint') }}</small>
          </span>
        </label>
        <div
          class="webview-reclaim-time"
          :class="{
            'webview-reclaim-time--disabled':
              desktopPreferencesUnavailable || !webviewReclaimEnabled,
          }"
        >
          <label class="webview-reclaim-time__label" for="preferences-webview-reclaim-time">
            {{ $t('components.reclaimWaitTime') }}
          </label>
          <SelectControl
            id="preferences-webview-reclaim-time"
            v-model="webviewReclaimIdleMinutes"
            name="preferences-webview-reclaim-time"
            :disabled="desktopPreferencesUnavailable || !webviewReclaimEnabled"
            match-trigger-width
            @change="handleDesktopPreferencesChange"
          >
            <option :value="5">{{ $t('components.minutes', { n: 5 }) }}</option>
            <option :value="15">{{ $t('components.minutes', { n: 15 }) }}</option>
            <option :value="30">{{ $t('components.minutes', { n: 30 }) }}</option>
            <option :value="60">{{ $t('components.hours', { n: 1 }) }}</option>
            <option :value="120">{{ $t('components.hours', { n: 2 }) }}</option>
            <option :value="240">{{ $t('components.hours', { n: 4 }) }}</option>
          </SelectControl>
        </div>
      </section>

      <section class="app-preferences__section" :aria-label="$t('components.dataCollection')">
        <h3>{{ $t('components.dataCollection') }}</h3>
        <label class="consent-toggle">
          <input
            v-model="telemetryEnabled"
            type="checkbox"
            name="preferences-telemetry"
            @change="handleTelemetryChange"
          />
          <span class="consent-toggle__copy">
            <strong>{{ $t('components.sendAnonymousData') }}</strong>
            <small>{{ $t('components.telemetryHint') }}</small>
          </span>
        </label>
        <p class="app-preferences__policy">
          <a href="#" @click.prevent="openTelemetryPolicy">{{ $t('components.microsoftPrivacy') }}</a>
        </p>
      </section>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">
        {{ $t('common.close') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ModalDialog from "../ModalDialog.vue";
import SelectControl from "../forms/SelectControl.vue";
import { startTelemetryIfConsented, stopTelemetry } from "../../telemetry/clarity";
import {
  TELEMETRY_POLICY_URL,
  readTelemetryConsent,
  saveTelemetryConsent,
} from "../../telemetry/consent";
import {
  checkForUpdate,
  getDesktopPreferences,
  openExternal,
  runtimeSnapshot,
  setDesktopPreferences,
} from "../../shell/runtime";
import { defaultDesktopPreferences } from "../../shell/contract";
import { resolveApiClientMetadata } from "../../api/runtime-config";
import type {
  AppUpdateCheckResult,
  DesktopCloseBehavior,
  DesktopPreferences,
  DesktopWebviewReclaimIdleMinutes,
} from "../../shell/contract";
import { notice } from "../../notices/notice";
import ThemePreferenceSelector from "./ThemePreferenceSelector.vue";
import SegmentedPreferenceSelector from "./SegmentedPreferenceSelector.vue";
import { appLocale, setAppLocale } from "../../i18n";
import type { AppLocale } from "../../i18n";
import { contactEntryVisible, setContactEntryVisible } from "../../contact/contactPreferences";
import { openAppUpdateDialog } from "../../updates/appUpdate";
import { updateCheckErrorMessage } from "../../updates/messages";
import { autoUpdateCheckEnabled, setAutoUpdateCheckEnabled } from "../../updates/updatePreferences";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const telemetryEnabled = ref(false);
const contactVisible = ref(contactEntryVisible.value);
// 语言切换即时生效并持久化；跨标签页变更经 appLocale 同步回本 Dialog。
const selectedLocale = ref<AppLocale>(appLocale.value);
const localeChoices = computed<readonly { value: AppLocale; label: string }[]>(() => [
  // 语言自名按 i18n.md 新增语言规则保持原文，不随界面语言翻译。
  { value: "zh-CN", label: t("common.languageZh") },
  { value: "en-US", label: t("common.languageEn") },
]);
function handleLocaleChange(value: string): void {
  setAppLocale(value as AppLocale);
}
watch(appLocale, (value) => {
  selectedLocale.value = value;
});
const closeBehavior = ref<DesktopCloseBehavior>(defaultDesktopPreferences.closeBehavior);
const autostartEnabled = ref(defaultDesktopPreferences.autostartEnabled);
const autostartSilent = ref(defaultDesktopPreferences.autostartSilent);
const webviewReclaimEnabled = ref(defaultDesktopPreferences.webviewReclaimEnabled);
const webviewReclaimIdleMinutes = ref<DesktopWebviewReclaimIdleMinutes>(
  defaultDesktopPreferences.webviewReclaimIdleMinutes,
);
const desktopPreferencesLoading = ref(false);
const desktopPreferencesLoaded = ref(false);
const desktopPreferencesSaving = ref(false);
const isDesktop = computed(() => runtimeSnapshot.value?.platform === "desktop");
const updateSupported = computed(() => {
  const platform = runtimeSnapshot.value?.platform;
  return platform === "desktop" || platform === "android";
});
const currentVersion = ref(resolveApiClientMetadata().appVersion);
const availableUpdate = ref<AppUpdateCheckResult | null>(null);
const updateChecking = ref(false);
const updateChecked = ref(false);
const updateError = ref("");
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

function handleAutoUpdateCheckChange(event: Event): void {
  const input = event.currentTarget;
  if (input instanceof HTMLInputElement) {
    setAutoUpdateCheckEnabled(input.checked);
  }
}

async function handleUpdateCheck(): Promise<void> {
  updateChecking.value = true;
  updateError.value = "";
  try {
    const result = await checkForUpdate();
    if (!result) {
      updateError.value = t("components.updateNotSupported");
      notice.error(t("components.updateCheckFailed"), { detail: updateError.value });
      return;
    }
    currentVersion.value = result.currentVersion;
    availableUpdate.value = result.latestVersion ? result : null;
    updateChecked.value = true;
    if (result.latestVersion) {
      openAppUpdateDialog(result, "preferences");
    } else {
      notice.success(t("components.upToDate"), {
        detail: t("components.currentVersionDetail", { version: result.currentVersion }),
      });
    }
  } catch (error) {
    availableUpdate.value = null;
    updateError.value = updateCheckErrorMessage(error);
    notice.error(t("components.updateCheckFailed"), { detail: updateError.value });
  } finally {
    updateChecking.value = false;
  }
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
    webviewReclaimEnabled.value = preferences.webviewReclaimEnabled;
    webviewReclaimIdleMinutes.value = preferences.webviewReclaimIdleMinutes;
    desktopPreferencesLoaded.value = true;
  } catch (error) {
    if (request !== desktopPreferencesRequest) return;
    notice.error(t("components.desktopPreferencesLoadFailed"), { detail: errorMessage(error) });
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
    notice.warning(t("components.closingStopsService"), {
      detail: t("components.closingStopsServiceDetail"),
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
      webviewReclaimEnabled: webviewReclaimEnabled.value,
      webviewReclaimIdleMinutes: webviewReclaimIdleMinutes.value,
    });
    if (preferences) {
      closeBehavior.value = preferences.closeBehavior;
      autostartEnabled.value = preferences.autostartEnabled;
      autostartSilent.value = preferences.autostartSilent;
      webviewReclaimEnabled.value = preferences.webviewReclaimEnabled;
      webviewReclaimIdleMinutes.value = preferences.webviewReclaimIdleMinutes;
    }
    return preferences;
  } catch (error) {
    notice.error(t("components.desktopPreferencesSaveFailed"), { detail: errorMessage(error) });
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
  return error instanceof Error ? error.message : t("components.retryLater");
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

.app-update__row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
}

.app-update__copy {
  display: grid;
  min-width: 0;
  gap: 5px;
}

.app-update__copy strong {
  color: var(--color-text);
  font-size: 14px;
  font-weight: 670;
}

.app-update__copy small {
  color: var(--color-muted);
  font-size: 12px;
  line-height: 1.45;
}

.app-update__error {
  margin: 0;
  color: var(--color-danger);
  font-size: 12px;
  line-height: 1.45;
}

.startup-preference--disabled {
  cursor: not-allowed;
  opacity: 0.66;
}

.webview-reclaim-time {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(150px, 190px);
  align-items: center;
  gap: 12px;
  min-width: 0;
  padding-left: 28px;

  &--disabled {
    opacity: 0.66;
  }
}

.webview-reclaim-time__label {
  min-width: 0;
  color: var(--color-muted);
  font-size: 12px;
}

@media (max-width: 540px) {
  .app-update__row {
    grid-template-columns: minmax(0, 1fr);
  }

  .webview-reclaim-time {
    grid-template-columns: minmax(0, 1fr);
    padding-left: 0;
  }
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
