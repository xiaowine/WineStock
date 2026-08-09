<!--
  本组件拥有支持 WineStock Dialog 的内容与二维码展示，属于 frontend 捐赠组件层。
  它只消费捐赠配置并发出用户选择，不拥有自动提示计数或本地存储。
-->
<template>
  <ModalDialog
    :open="open"
    :title="$t('misc.donationTitle')"
    :description="$t('misc.donationDescription')"
    :wide="donationMethods.length > 1"
    @close="emit('close')"
  >
    <div class="donation-dialog">
      <section
        v-if="donationMethods.length"
        class="donation-dialog__methods"
        :aria-label="$t('misc.donationMethodsLabel')"
      >
        <article v-for="method in donationMethods" :key="method.id" class="donation-dialog__method">
          <header class="donation-dialog__method-header">
            <h3>{{ $t(METHOD_LABEL_KEYS[method.id]) }}</h3>
          </header>

          <div class="donation-dialog__qr-frame">
            <img
              v-if="qrUrls[method.id]"
              :src="qrUrls[method.id]"
              :alt="$t('misc.donationQrAlt', { method: $t(METHOD_LABEL_KEYS[method.id]) })"
              class="donation-dialog__qr"
            />
            <span
              v-else-if="qrLoading[method.id]"
              class="donation-dialog__qr-status"
              role="status"
              aria-live="polite"
            >
              {{ $t("misc.donationQrGenerating") }}
            </span>
            <span v-else class="donation-dialog__qr-status donation-dialog__qr-status--error">
              {{ $t("misc.donationQrUnavailable") }}
            </span>
          </div>

          <p v-if="qrErrors[method.id]" class="donation-dialog__error" role="alert">
            {{ qrErrors[method.id] }}
          </p>
        </article>
      </section>
    </div>

    <template #actions>
      <button v-if="automatic" class="secondary-button" type="button" @click="emit('disable')">
        {{ $t("misc.donationNeverPrompt") }}
      </button>
      <button v-if="automatic" class="secondary-button" type="button" @click="emit('snooze')">
        {{ $t("misc.donationLater") }}
      </button>
      <button class="secondary-button" type="button" @click="emit('close')">
        {{ $t("common.close") }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, reactive, watch } from "vue";
import { useI18n } from "vue-i18n";
import ModalDialog from "../ModalDialog.vue";
import { donationMethods, type DonationMethodId } from "../../donation/config";
import { generateDonationQr } from "../../donation/qrGenerator";

/** 捐赠方式的显示名由语言包按 id 解析，标签文案不进入配置数据。 */
const METHOD_LABEL_KEYS: Record<DonationMethodId, string> = {
  wechat: "misc.donationMethodWechat",
  alipay: "misc.donationMethodAlipay",
};

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    /** 是否显示 Dialog。 */
    open: boolean;
    /** 是否由自动里程碑触发；手动入口不显示自动提示选择。 */
    automatic?: boolean;
  }>(),
  { automatic: false },
);
const emit = defineEmits<{
  close: [];
  snooze: [];
  disable: [];
}>();

const qrUrls = reactive<Partial<Record<DonationMethodId, string>>>({});
const qrErrors = reactive<Partial<Record<DonationMethodId, string>>>({});
const qrLoading = reactive<Partial<Record<DonationMethodId, boolean>>>({});
let generationRequest = 0;

watch(
  () => props.open,
  (open) => {
    if (open) {
      void generateQrs();
    } else {
      releaseQrUrls();
    }
  },
  { immediate: true },
);

onBeforeUnmount(releaseQrUrls);

async function generateQrs(): Promise<void> {
  releaseQrUrls();
  const request = ++generationRequest;
  for (const method of donationMethods) {
    qrLoading[method.id] = true;
    delete qrErrors[method.id];
  }

  await Promise.all(
    donationMethods.map(async (method) => {
      try {
        const image = await generateDonationQr(method.content);
        if (request !== generationRequest) return;
        qrUrls[method.id] = URL.createObjectURL(image);
      } catch (error) {
        if (request !== generationRequest) return;
        qrErrors[method.id] = error instanceof Error ? error.message : t("misc.donationQrFailedRetry");
      } finally {
        if (request === generationRequest) qrLoading[method.id] = false;
      }
    }),
  );
}

function releaseQrUrls(): void {
  generationRequest += 1;
  for (const method of donationMethods) {
    const url = qrUrls[method.id];
    if (url) URL.revokeObjectURL(url);
    delete qrUrls[method.id];
    delete qrErrors[method.id];
    delete qrLoading[method.id];
  }
}
</script>

<style scoped lang="scss" src="./DonationDialog.scss"></style>
