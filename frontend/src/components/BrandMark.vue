<!-- 本组件呈现跨页面复用的 WineStock 品牌母版；只拥有图形语义，不拥有布局或平台资源。 -->
<template>
  <svg
    class="brand-mark"
    viewBox="0 0 108 108"
    :role="label ? 'img' : undefined"
    :aria-label="label || undefined"
    :aria-hidden="label ? undefined : 'true'"
    focusable="false"
  >
    <path
      fill="currentColor"
      fill-rule="evenodd"
      d="M54 18 88 35 54 52 20 35l34-17ZM20 43l30 15v32L20 75V43Zm38 15 30-15v32L58 90V58Zm7 5 16-8v7l-16 8v-7Z"
    />
  </svg>
</template>

<script setup lang="ts">
defineProps<{
  label?: string;
}>();
</script>
