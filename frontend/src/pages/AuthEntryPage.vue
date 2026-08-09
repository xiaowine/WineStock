<!-- 本页面拥有匿名认证入口的首用户状态分流；它不渲染登录或注册表单。 -->
<template>
  <main v-overlay-scrollbar class="auth-page">
    <section class="auth-panel" aria-labelledby="auth-entry-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="auth-entry-title">{{ $t("auth.preparingConnection") }}</h1>
          <p>{{ errorMessage || $t("auth.confirmingAccountStatus") }}</p>
        </div>
      </header>
      <p v-if="checking" class="auth-runtime-note" role="status">
        {{ $t("auth.checkingService") }}
      </p>
      <div v-else class="auth-page-actions">
        <button class="primary-button" type="button" @click="resolveEntry">
          {{ $t("auth.retry") }}
        </button>
        <!-- returnTo 使用完整 fullPath（含 redirect），便于设置完成后桥接回业务目标。 -->
        <RouterLink
          class="secondary-button"
          :to="{ name: 'runtime-settings', query: { returnTo: route.fullPath } }"
        >
          {{ $t("auth.runtimeMode") }}
        </RouterLink>
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getAuthBootstrapStatus } from "../api/auth";
import BrandMark from "../components/BrandMark.vue";

const router = useRouter();
const route = useRoute();
const { t } = useI18n();
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
    errorMessage.value = t("auth.entryStatusCheckFailed");
  } finally {
    checking.value = false;
  }
}
</script>
