<!-- 本组件拥有发现新版本后的统一 Dialog；更新检查、下载和安装由平台 Shell Bridge 执行。 -->
<template>
  <ModalDialog
    :open="open && Boolean(update)"
    :title="$t('misc.updateDialogTitle')"
    :description="$t('misc.updateDescription')"
    :busy="installing"
    :nested="nested"
    compact
    @close="handleClose"
    @after-close="handleAfterClose"
  >
    <div v-if="update" class="app-update-dialog">
      <section class="app-update-dialog__notes" aria-labelledby="app-update-notes-title">
        <h3 id="app-update-notes-title">{{ $t("misc.updateReleaseNotesTitle") }}</h3>
        <p>{{ update.notes || $t("misc.updateNoNotes") }}</p>
      </section>
      <div class="app-update-dialog__version-summary">
        <span class="app-update-dialog__version-label">{{ $t("misc.updateVersionLabel") }}</span>
        <div class="app-update-dialog__version-flow" :aria-label="$t('misc.updateVersionChange')">
          <span class="app-update-dialog__version app-update-dialog__version--current">
            {{ update.currentVersion }}
          </span>
          <span class="app-update-dialog__version-arrow" aria-hidden="true">→</span>
          <strong class="app-update-dialog__version app-update-dialog__version--latest">
            {{ update.latestVersion }}
          </strong>
        </div>
      </div>
      <p v-if="installError" class="app-update-dialog__error" role="alert">{{ installError }}</p>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="installing" @click="handleClose">
        {{ $t("misc.updateLater") }}
      </button>
      <button class="primary-button" type="button" :disabled="installing" @click="handleInstall">
        {{ installing ? $t("misc.updatePreparingInstall") : $t("misc.updateInstallNow") }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ModalDialog from "../ModalDialog.vue";
import { installUpdate } from "../../shell/runtime";
import type { AppUpdateCheckResult } from "../../shell/contract";
import { notice } from "../../notices/notice";
import { closeAppUpdateDialog, clearAppUpdateDialog } from "../../updates/appUpdate";
import { updateInstallErrorMessage } from "../../updates/messages";

const props = defineProps<{
  open: boolean;
  update: AppUpdateCheckResult | null;
  nested?: boolean;
}>();

const installing = ref(false);
const installError = ref("");
const { t } = useI18n();

watch(
  () => [props.open, props.update?.latestVersion] as const,
  ([open, version], previous) => {
    if (open && version !== previous?.[1]) {
      installError.value = "";
    }
  },
);

async function handleInstall(): Promise<void> {
  const version = props.update?.latestVersion;
  if (!version || installing.value) return;

  installing.value = true;
  installError.value = "";
  try {
    await installUpdate(version);
    notice.success(t("misc.updateInstallerStarted"), { detail: t("misc.updateInstallerContinues") });
    closeAppUpdateDialog();
  } catch (error) {
    installError.value = updateInstallErrorMessage(error);
    notice.error(t("misc.updateInstallFailed"), { detail: installError.value });
  } finally {
    installing.value = false;
  }
}

function handleClose(): void {
  if (!installing.value) closeAppUpdateDialog();
}

function handleAfterClose(): void {
  clearAppUpdateDialog();
  installError.value = "";
}
</script>

<style scoped lang="scss">
.app-update-dialog {
  display: grid;
  gap: 18px;

  &__version-summary {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--color-border);
  }

  &__version-label {
    flex: 0 0 auto;
    color: var(--color-muted);
    font-size: 12px;
  }

  &__version-flow {
    display: inline-flex;
    min-width: 0;
    align-items: baseline;
    justify-content: flex-end;
    gap: 9px;
  }

  &__version {
    overflow-wrap: anywhere;
  }

  &__version--current {
    color: var(--color-muted);
    font-size: 13px;
  }

  &__version-arrow {
    color: var(--color-accent);
    font-size: 15px;
  }

  &__version--latest {
    color: var(--color-text);
    font-size: 14px;
    font-weight: 680;
  }

  &__notes {
    display: grid;
    gap: 8px;
    margin: 0;
    min-width: 0;
    padding: 12px 0 12px 15px;
    border-left: 3px solid var(--color-accent);
    background: var(--color-accent-soft);
  }

  &__notes h3 {
    margin: 0;
    color: var(--color-text);
    font-size: 16px;
    font-weight: 700;
  }

  &__notes p,
  &__error {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  &__notes p {
    color: var(--color-text);
    font-size: 14px;
  }

  &__error {
    color: var(--color-danger);
  }
}

@media (max-width: 420px) {
  .app-update-dialog__version-summary {
    align-items: flex-start;
    flex-direction: column;
    gap: 8px;
  }

  .app-update-dialog__version-flow {
    justify-content: flex-start;
  }
}
</style>
