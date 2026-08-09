<!-- 本组件拥有联系作者 Dialog 的内容编排；联系方式数据、偏好和外链能力由各自模块提供。 -->
<template>
  <ModalDialog
    :open="open"
    :title="$t('misc.contactTitle')"
    :description="$t('misc.contactDescription')"
    @close="emit('close')"
  >
    <div class="contact-dialog">
      <section class="contact-dialog__highlight" :aria-label="$t('misc.contactFeedbackAria')">
        <div class="contact-dialog__highlight-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" focusable="false">
            <path d="M4 5.5h16v11H8l-4 3v-14Z" />
            <path d="M8 10h8M8 13h5" />
          </svg>
        </div>
        <div>
          <strong>{{ $t("misc.contactProjectFeedback") }}</strong>
          <p>{{ $t("misc.contactFeedbackViaGithub") }}</p>
        </div>
        <button class="primary-button" type="button" @click="openFeedback">
          {{ $t("misc.contactOpenFeedbackPage") }}
        </button>
      </section>

      <section class="contact-dialog__section" aria-labelledby="contact-details-title">
        <h3 id="contact-details-title">{{ $t("misc.contactDetailsTitle") }}</h3>
        <dl class="contact-dialog__details">
          <div>
            <dt>{{ $t("misc.contactEmail") }}</dt>
            <dd v-copyable="{ text: CONTACT_INFO.email, label: $t('misc.contactEmail') }">
              {{ CONTACT_INFO.email }}
            </dd>
          </div>
          <div>
            <dt>{{ $t("misc.contactQqGroup") }}</dt>
            <dd>
              <button
                class="text-button contact-dialog__link"
                type="button"
                @click="openContactLink(CONTACT_INFO.qqGroupUrl, $t('misc.contactQqGroup'))"
              >
                {{ CONTACT_INFO.qqGroup }}
              </button>
            </dd>
          </div>
          <div>
            <dt>GitHub</dt>
            <dd>
              <button
                class="text-button contact-dialog__link"
                type="button"
                @click="openContactLink(CONTACT_INFO.feedbackUrl, $t('misc.contactGithubAddress'))"
              >
                {{ CONTACT_INFO.feedbackUrl }}
              </button>
            </dd>
          </div>
        </dl>
      </section>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">
        {{ $t("common.close") }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import ModalDialog from "../components/ModalDialog.vue";
import { notice } from "../notices/notice";
import { openExternal } from "../shell/runtime";
import { CONTACT_INFO } from "./contactInfo";

defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

function openFeedback(): void {
  openContactLink(CONTACT_INFO.feedbackUrl, t("misc.contactFeedbackPage"));
}

function openContactLink(url: string, label: string): void {
  void openExternal(url).catch((error: unknown) => {
    notice.error(t("misc.contactCannotOpen", { name: label }), {
      detail: error instanceof Error ? error.message : t("misc.contactRetryLater"),
    });
  });
}
</script>

<style scoped lang="scss">
@use "../styles/foundation/mixins" as mixins;

.contact-dialog {
  display: grid;
  gap: 18px;

  &__highlight {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--color-accent-border);
    border-radius: var(--radius-md);
    background: var(--color-accent-soft);

    p {
      margin-top: 3px;
      color: var(--color-muted);
      font-size: 12px;
      line-height: 1.45;
    }
  }

  &__highlight-icon {
    display: grid;
    width: 36px;
    height: 36px;
    place-items: center;
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-accent);

    svg {
      width: 21px;
      height: 21px;
      fill: none;
      stroke: currentcolor;
      stroke-linecap: round;
      stroke-linejoin: round;
      stroke-width: 1.7;
    }
  }

  &__section {
    display: grid;
    gap: 10px;

    h3 {
      margin: 0;
      font-size: 13px;
    }
  }

  &__details {
    display: grid;
    margin: 0;
    border-top: 1px solid var(--color-border);

    > div {
      display: grid;
      grid-template-columns: 64px minmax(0, 1fr);
      gap: 12px;
      padding: 10px 0;
      border-bottom: 1px solid var(--color-border);
    }

    dt {
      color: var(--color-muted);
      font-size: 13px;
    }

    dd {
      min-width: 0;
      margin: 0;
      color: var(--color-text);
      font-size: 13px;
      overflow-wrap: anywhere;
    }

    .contact-dialog__link {
      min-width: 0;
      min-height: auto;
      padding: 0;
      color: var(--color-accent);
      font-size: 13px;
      line-height: 1.45;
      text-align: left;
      overflow-wrap: anywhere;
      white-space: normal;
    }
  }
}

@include mixins.mobile {
  .contact-dialog__highlight {
    grid-template-columns: auto minmax(0, 1fr);

    .primary-button {
      grid-column: 1 / -1;
      width: 100%;
    }
  }
}
</style>
