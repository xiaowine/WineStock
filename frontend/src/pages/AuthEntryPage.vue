<!-- 本页面拥有匿名认证入口的首用户状态分流；它不渲染登录或注册表单。 -->
<template>
  <main class="auth-page">
    <section class="auth-panel" aria-labelledby="auth-entry-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <span class="brand-mark">W</span>
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="auth-entry-title">准备连接</h1>
          <p>{{ errorMessage || "正在确认当前服务的账户状态。" }}</p>
        </div>
      </header>
      <p v-if="checking" class="auth-runtime-note" role="status">正在检查服务状态…</p>
      <div v-else class="auth-page-actions">
        <button class="primary-button" type="button" @click="resolveEntry">重试</button>
        <RouterLink
          class="secondary-button"
          :to="{ name: 'runtime-settings', query: { returnTo: route.fullPath } }"
        >
          运行设置
        </RouterLink>
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { getAuthBootstrapStatus } from "../api/auth";

const router = useRouter();
const route = useRoute();
const checking = ref(true);
const errorMessage = ref("");

onMounted(resolveEntry);

async function resolveEntry(): Promise<void> {
  checking.value = true;
  errorMessage.value = "";
  try {
    const status = await getAuthBootstrapStatus();
    await router.replace({
      name: status.requires_initial_user ? "register" : "login",
      query: route.query,
    });
  } catch {
    errorMessage.value = "无法确认账户状态，请检查服务连接后重试。";
  } finally {
    checking.value = false;
  }
}
</script>
