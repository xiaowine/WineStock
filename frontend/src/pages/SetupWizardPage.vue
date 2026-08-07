<!--
  本页面拥有首次初始化向导的分步选择与一次性 apply 编排；它不拥有配置校验规则、
  Shell 生命周期或运行设置页的高级配置能力。
  设计与文案定稿见 docs/implementation-notes/first-run-setup-wizard.md。
-->
<template>
  <main v-overlay-scrollbar class="auth-page">
    <section class="auth-panel setup-wizard" aria-labelledby="setup-wizard-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
        </div>
      </header>

      <div class="setup-wizard__body">
        <Transition :name="stepTransitionName">
          <!-- 第 1 页：欢迎 + 使用方式 -->
          <div v-if="step === 'mode'" key="mode" class="setup-wizard__step">
            <div class="setup-wizard__head">
              <h1 id="setup-wizard-title">欢迎使用 WineStock</h1>
              <p>先选择这台设备的使用方式，稍后可以随时在设置中更改。</p>
            </div>
            <div class="choice-card-group" role="radiogroup" aria-label="使用方式">
              <label
                v-for="option in modeOptions"
                :key="option.value"
                class="choice-card"
                :class="{ 'choice-card--selected': mode === option.value }"
              >
                <input
                  v-model="mode"
                  type="radio"
                  name="setup_mode"
                  :value="option.value"
                  :disabled="option.disabled"
                />
                <strong>
                  {{ option.label }}
                  <span v-if="option.recommended" class="choice-card__badge">推荐</span>
                </strong>
                <span>{{ option.description }}</span>
              </label>
            </div>
          </div>

          <!-- 第 2 页：服务器地址（条件页） -->
          <div v-else-if="step === 'server'" key="server" class="setup-wizard__step">
            <div class="setup-wizard__head">
              <h1 id="setup-wizard-title">连接服务器</h1>
              <p>输入服务器地址，通常由部署 WineStock 的人提供。</p>
            </div>
            <form class="auth-form" novalidate @submit.prevent="goForward">
              <FormInput
                v-model="serverUrl"
                label="服务器地址"
                validation-key="remoteBaseUrl"
                :error="serverUrlError"
                hint="示例：http://192.168.1.10:17890"
                name="server-url"
                type="url"
                autocomplete="off"
                inputmode="url"
                placeholder="http://192.168.1.10:17890"
              />
            </form>
          </div>

          <!-- 第 3 页：本机偏好 -->
          <div v-else-if="step === 'consent'" key="consent" class="setup-wizard__step">
            <div class="setup-wizard__head">
              <h1 id="setup-wizard-title">偏好设置</h1>
              <p>这些选项只影响当前设备，稍后可以随时更改。</p>
            </div>
            <div class="setup-wizard__preferences">
              <section
                class="setup-wizard__preference-section"
                aria-labelledby="setup-appearance-title"
              >
                <h2 id="setup-appearance-title">外观</h2>
                <ThemePreferenceSelector />
              </section>
              <section
                class="setup-wizard__preference-section"
                aria-labelledby="setup-telemetry-title"
              >
                <h2 id="setup-telemetry-title">数据收集</h2>
                <label class="consent-toggle">
                  <input v-model="telemetryConsent" type="checkbox" name="telemetry-consent" />
                  <span class="consent-toggle__copy">
                    <strong>发送匿名使用数据</strong>
                    <small
                      >帮助开发者定位和排查问题；不包含库存内容与账户信息，仅在联网时生效。分析服务由
                      Microsoft Clarity 提供。</small
                    >
                  </span>
                </label>
                <p class="auth-runtime-note setup-wizard__consent-note">
                  默认开启，可取消。
                  <a href="#" @click.prevent="openTelemetryPolicy">查看 Microsoft 隐私声明</a>
                </p>
              </section>
            </div>
          </div>

          <!-- 完成态：apply 进行中 / 失败重试 -->
          <div v-else key="applying" class="setup-wizard__step setup-wizard__step--applying">
            <div v-if="!applyError" class="setup-wizard__applying" role="status">
              <span class="setup-wizard__spinner" aria-hidden="true"></span>
              <p>加载中…</p>
            </div>
            <div v-else class="setup-wizard__apply-error">
              <div class="auth-page-actions">
                <button class="secondary-button" type="button" @click="restartWizard">
                  返回修改
                </button>
                <button class="primary-button" type="button" @click="applyConfiguration">
                  重试
                </button>
              </div>
            </div>
          </div>
        </Transition>
      </div>

      <footer
        class="setup-wizard__footer"
        :class="{ 'setup-wizard__footer--hidden': step === 'applying' }"
      >
        <div class="auth-page-actions setup-wizard__actions">
          <button v-if="!isFirstStep" class="secondary-button" type="button" @click="goBack">
            上一步
          </button>
          <button class="primary-button" type="button" @click="goForward">
            {{ isLastChoiceStep ? "完成" : "下一步" }}
          </button>
        </div>
        <div class="setup-wizard__dots" aria-hidden="true">
          <span
            v-for="(dotStep, index) in stepSequence"
            :key="dotStep"
            :class="{ 'is-on': index <= currentStepIndex || step === 'applying' }"
          ></span>
        </div>
      </footer>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import BrandMark from "../components/BrandMark.vue";
import FormInput from "../components/forms/FormInput.vue";
import ThemePreferenceSelector from "../components/preferences/ThemePreferenceSelector.vue";
import { startTelemetryIfConsented } from "../telemetry/clarity";
import { TELEMETRY_POLICY_URL, saveTelemetryConsent } from "../telemetry/consent";
import { notice } from "../notices/notice";
import type { EditableRuntimeConfig } from "../shell/contract";
import {
  applyRuntimeConfig,
  openExternal,
  runtimeSnapshot,
  validateRuntimeConfig,
} from "../shell/runtime";

type SetupMode = "local" | "server" | "remote";
type SetupStep = "mode" | "server" | "consent" | "applying";

const CONNECTION_TEST_TIMEOUT_MS = 4_000;

const router = useRouter();

/**
 * 纯网页端只有「连接远端」一种能力：使用方式页没有可做的决策，
 * 整页跳过，向导直接从服务器地址页开始。平台 shell 内不受影响。
 */
const isPureWebPlatform = runtimeSnapshot.value?.platform === "web";

const step = ref<SetupStep>(isPureWebPlatform ? "server" : "mode");
const direction = ref<"forward" | "back">("forward");
const mode = ref<SetupMode>(isPureWebPlatform ? "remote" : "local");
const serverUrl = ref("");
const serverUrlError = ref("");
const telemetryConsent = ref(true);
const applying = ref(false);
const applyError = ref("");

const modeOptions = [
  {
    value: "local" as const,
    label: "本机自用",
    description: "服务和数据运行在这台设备上，适合个人或单设备使用。",
    recommended: true,
    disabled: false,
  },
  {
    value: "server" as const,
    label: "共享服务",
    description: "在这台设备运行服务，允许其他设备连接使用。",
    recommended: false,
    disabled:
      runtimeSnapshot.value?.platform !== "desktop" ||
      !(runtimeSnapshot.value?.capabilities.serverMode ?? false),
  },
  {
    value: "remote" as const,
    label: "连接远端",
    description: "不启动本地服务，连接已经部署好的 WineStock 服务。",
    recommended: false,
    disabled: false,
  },
];

/** 决策步骤序列（不含完成态）；本机路径跳过服务器页，纯网页端跳过使用方式页。 */
const stepSequence = computed<readonly SetupStep[]>(() => {
  if (isPureWebPlatform) {
    return ["server", "consent"];
  }
  const steps: SetupStep[] = ["mode"];
  if (mode.value === "remote") {
    steps.push("server");
  }
  steps.push("consent");
  return steps;
});
const currentStepIndex = computed(() => stepSequence.value.indexOf(step.value));
const isFirstStep = computed(() => currentStepIndex.value <= 0 || step.value === "applying");
const isLastChoiceStep = computed(() => step.value === "consent");
const stepTransitionName = computed(() =>
  direction.value === "back" ? "setup-step-back" : "setup-step",
);

function goForward(): void {
  direction.value = "forward";
  if (step.value === "mode") {
    step.value = stepSequence.value[currentStepIndex.value + 1];
    return;
  }
  if (step.value === "server") {
    void advanceFromServerStep();
    return;
  }
  if (step.value === "consent") {
    saveTelemetryConsent(telemetryConsent.value);
    // 同意即从当前会话开始采集，不必等下次冷启动；未勾选时为空操作。
    startTelemetryIfConsented();
    void applyConfiguration();
  }
}

function goBack(): void {
  direction.value = "back";
  const index = currentStepIndex.value;
  if (index > 0) {
    step.value = stepSequence.value[index - 1];
  }
}

/** 服务器地址先走权威校验（shared 规则），字段错误映射到输入框后才允许前进。 */
async function advanceFromServerStep(): Promise<void> {
  serverUrlError.value = "";
  const validation = await validateRuntimeConfig(buildCandidateConfig());
  const fieldError = validation.fieldErrors.remoteBaseUrl?.[0];
  if (!validation.valid && fieldError) {
    serverUrlError.value = fieldError;
    notice.warning("请检查服务器地址", { detail: fieldError });
    return;
  }
  if (!validation.valid) {
    serverUrlError.value = "服务器地址无效，请检查后重试";
    notice.warning("请检查服务器地址", { detail: serverUrlError.value });
    return;
  }
  step.value = "consent";
}

/** 远端配置在 apply 前必须通过健康检查，失败时不保存配置。 */
async function testRemoteConnection(): Promise<boolean> {
  const base = serverUrl.value.trim().replace(/\/+$/, "");
  if (!base) {
    return false;
  }
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), CONNECTION_TEST_TIMEOUT_MS);
  try {
    const response = await fetch(`${base}/api/health`, {
      headers: { accept: "application/json" },
      credentials: "omit",
      signal: controller.signal,
    });
    const payload = (await response.json()) as unknown;
    if (!response.ok || !isHealthPayload(payload)) {
      throw new Error("invalid health response");
    }
    return true;
  } catch (error) {
    const detail =
      error instanceof DOMException && error.name === "AbortError"
        ? "连接超时，请检查服务器地址和网络连接。"
        : "暂时无法连接，请检查服务器地址和网络连接。";
    notice.error("远端连接测试失败", { detail });
    return false;
  } finally {
    window.clearTimeout(timeout);
  }
}

function buildCandidateConfig(): EditableRuntimeConfig {
  // 向导只决定 mode 与远端地址；bindHost/port 沿用 Shell 默认草稿，
  // self-hosted 的端口分配由 Shell apply 链路自理。
  const base = runtimeSnapshot.value?.config;
  const draft: EditableRuntimeConfig = base
    ? { ...base }
    : { mode: "self-hosted", bindHost: "127.0.0.1", port: 0, remoteBaseUrl: "" };
  if (mode.value === "local") {
    return { ...draft, mode: "self-hosted" };
  }
  if (mode.value === "server") {
    return { ...draft, mode: "server-mode" };
  }
  return { ...draft, mode: "client-only", remoteBaseUrl: serverUrl.value.trim() };
}

async function applyConfiguration(): Promise<void> {
  if (applying.value) {
    return;
  }
  direction.value = "forward";
  step.value = "applying";
  applying.value = true;
  applyError.value = "";
  try {
    if (mode.value === "remote" && !(await testRemoteConnection())) {
      applyError.value = "远端连接测试失败，请返回修改服务器地址或检查网络连接";
      return;
    }
    const result = await applyRuntimeConfig(buildCandidateConfig());
    if (result.applied) {
      await router.replace({ name: "auth-entry" });
      return;
    }
    applyError.value =
      result.error?.message ??
      Object.values(result.fieldErrors)[0]?.[0] ??
      "无法应用当前配置，请返回修改后重试";
    notice.error("应用运行配置失败", {
      detail: applyError.value,
      onClick: () => void applyConfiguration(),
    });
  } catch (error) {
    applyError.value = error instanceof Error ? error.message : "无法应用当前配置，请稍后重试";
    notice.error("应用运行配置失败", {
      detail: applyError.value,
      onClick: () => void applyConfiguration(),
    });
  } finally {
    applying.value = false;
  }
}

/** 打开 Microsoft 隐私声明；平台不支持外链能力时静默忽略，不阻断向导。 */
function openTelemetryPolicy(): void {
  void openExternal(TELEMETRY_POLICY_URL).catch(() => undefined);
}

/** apply 失败后返回首个决策页重来；已填选择保留。 */
function restartWizard(): void {
  applyError.value = "";
  direction.value = "back";
  step.value = stepSequence.value[0];
}

function isHealthPayload(value: unknown): value is { status: "OK" } {
  return typeof value === "object" && value !== null && "status" in value && value.status === "OK";
}
</script>

<style scoped lang="scss" src="./SetupWizardPage.scss"></style>
