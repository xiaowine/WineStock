<!--
  本组件只呈现 Shell 已确认的局域网连接 URL，并提供复制反馈。
  它不读取运行快照、不枚举网卡，也不推导监听地址。
-->
<template>
  <ModalDialog
    :open="open"
    title="本机局域网地址"
    description="以下地址属于当前设备，供网络可达的其它设备连接 WineStock。"
    compact
    @close="emit('close')"
  >
    <div class="lan-access-dialog">
      <ul v-if="urls.length" class="lan-access-dialog__list" aria-label="本机局域网地址">
        <li v-for="url in urls" :key="url" class="lan-access-dialog__item">
          <code class="lan-access-dialog__address">{{ url }}</code>
          <button
            v-copyable="{ text: url, label: '连接地址' }"
            class="icon-button lan-access-dialog__copy"
            type="button"
            title="复制地址"
            :aria-label="`复制连接地址 ${url}`"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="M8 8h11v11H8z" />
              <path d="M5 16H4V5h11v1" />
            </svg>
          </button>
        </li>
      </ul>
      <p v-else class="lan-access-dialog__empty" role="status">当前设备没有可用的局域网地址。</p>

      <p class="lan-access-dialog__guidance">
        连接失败时，请确认设备与本机网络互通，并检查操作系统防火墙。
      </p>
      <div v-if="hasInsecureUrl" class="form-warning lan-access-dialog__warning" role="status">
        列表中包含 HTTP 明文地址，只应分享给可信网络内的设备。
      </div>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">关闭</button>
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
