<!-- 本组件拥有本机主题三态选择控件；主题状态解析、持久化和平台同步仍由 theme runtime 负责。 -->
<template>
  <SegmentedPreferenceSelector
    :options="themeChoices"
    :model-value="themePreference"
    :ariaLabel="t('components.theme')"
    @change="handleThemeChange"
  />
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { setThemePreference, themePreference } from "../../theme/runtime";
import type { ThemePreference } from "../../theme/model";
import SegmentedPreferenceSelector from "./SegmentedPreferenceSelector.vue";

const { t } = useI18n();
const themeChoices = computed<readonly { value: ThemePreference; label: string }[]>(() => [
  { value: "system", label: t("components.themeFollowSystem") },
  { value: "light", label: t("components.themeLight") },
  { value: "dark", label: t("components.themeDark") },
]);

/** 通用分段控件以字符串上抛；此处收窄为主题偏好并交给 theme runtime。 */
function handleThemeChange(value: string): void {
  setThemePreference(value as ThemePreference);
}
</script>
