<!--
  本组件只呈现 Shell 已确认的局域网连接 URL，并提供复制反馈。
  它不读取运行快照、不枚举网卡，也不推导监听地址。
-->
<template>
  <ModalDialog
    :open="open"
    :title="$t('runtime.lanDialogTitle')"
    :description="$t('runtime.lanDialogDescription')"
    compact
    @close="emit('close')"
  >
    <div class="lan-access-dialog">
      <ul
        v-if="urls.length"
        class="lan-access-dialog__list"
        :aria-label="$t('runtime.lanDialogTitle')"
      >
        <li v-for="url in urls" :key="url" class="lan-access-dialog__item">
          <code class="lan-access-dialog__address">{{ url }}</code>
          <button
            v-copyable="{ text: url, label: $t('runtime.lanCopyLabel') }"
            class="icon-button lan-access-dialog__copy"
            type="button"
            :title="$t('runtime.copyAddress')"
            :aria-label="$t('runtime.copyAddressAria', { url })"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="M8 8h11v11H8z" />
              <path d="M5 16H4V5h11v1" />
            </svg>
          </button>
        </li>
      </ul>
      <p v-else class="lan-access-dialog__empty" role="status">
        {{ $t("runtime.lanEmpty") }}
      </p>

      <p class="lan-access-dialog__guidance">{{ $t("runtime.lanGuidance") }}</p>
      <div v-if="hasInsecureUrl" class="form-warning lan-access-dialog__warning" role="status">
        {{ $t("runtime.lanInsecureWarning") }}
      </div>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">
        {{ $t("runtime.close") }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ModalDialog from "../ModalDialog.vue";

const props = defineProps<{
  open: boolean;
  urls: readonly string[];
}>();

const emit = defineEmits<{
  close: [];
}>();

const hasInsecureUrl = computed(() => props.urls.some((url) => url.startsWith("http://")));
</script>

<style lang="scss" src="./LanAccessDialog.scss"></style>
