<!--
  本组件拥有支持 WineStock Dialog 的内容与二维码展示，属于 frontend 捐赠组件层。
  它只消费捐赠配置并发出用户选择，不拥有自动提示计数或本地存储。
-->
<template>
  <ModalDialog
    :open="open"
    title="支持软件"
    description="感谢你帮助 WineStock 持续维护。"
    :wide="donationMethods.length > 1"
    @close="emit('close')"
  >
    <div class="donation-dialog">
      <section v-if="donationMethods.length" class="donation-dialog__methods" aria-label="捐赠方式">
        <article v-for="method in donationMethods" :key="method.id" class="donation-dialog__method">
          <header class="donation-dialog__method-header">
            <h3>{{ method.label }}</h3>
          </header>

          <div class="donation-dialog__qr-frame">
            <img
              v-if="qrUrls[method.id]"
              :src="qrUrls[method.id]"
              :alt="`${method.label}捐赠二维码`"
              class="donation-dialog__qr"
            />
            <span
              v-else-if="qrLoading[method.id]"
              class="donation-dialog__qr-status"
              role="status"
              aria-live="polite"
            >
              正在生成二维码…
            </span>
            <span v-else class="donation-dialog__qr-status donation-dialog__qr-status--error">
              二维码暂时不可用
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
        不再提示
      </button>
      <button v-if="automatic" class="secondary-button" type="button" @click="emit('snooze')">
        稍后再说
      </button>
      <button class="secondary-button" type="button" @click="emit('close')">关闭</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, reactive, watch } from "vue";
import ModalDialog from "../ModalDialog.vue";
import { donationMethods, type DonationMethodId } from "../../donation/config";
import { generateDonationQr } from "../../donation/qrGenerator";

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
        qrErrors[method.id] =
          error instanceof Error ? error.message : "二维码生成失败，请稍后重试。";
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
