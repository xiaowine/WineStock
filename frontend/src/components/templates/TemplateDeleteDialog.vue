<!-- 本组件按业务域呈现删除后果与影响范围的普通确认，不调用删除 API，也不要求输入内容验证。 -->
<template>
  <ModalDialog
    :open="Boolean(target)"
    :title="dialogTitle"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="target" class="dialog-account-context dialog-account-context--danger">
        <span>{{ $t('templates.deleteTargetLabel') }}</span>
        <strong>{{ target.name }}</strong>
      </div>
    </template>
    <div class="template-delete-copy">
      <template v-if="target?.kind === 'category'">
        <p>{{ $t('templates.categoryDeleteWarning') }}</p>
      </template>
      <template v-else-if="target">
        <p><strong>{{ $t('templates.templateDeleteDangerIntro') }}</strong></p>
        <ul>
          <li>{{ $t('templates.templateDeleteDetach') }}</li>
          <li>{{ $t('templates.templateDeleteValues') }}</li>
          <li>{{ $t('templates.templateDeleteIrreversible') }}</li>
        </ul>
      </template>
      <section
        v-if="target"
        class="template-delete-copy__impact"
        :aria-label="$t('templates.deleteImpactAria')"
      >
        <strong>{{ $t('templates.impactScope') }}</strong>
        <p v-if="target.itemUsageCount !== null && target.itemUsageCount > 0">
          {{
            $t('templates.affectedItemsSummary', {
              n: target.itemUsageCount,
              subject: target.kind === 'category' ? $t('templates.subjectCategory') : $t('templates.subjectTemplate'),
            })
          }}
        </p>
        <p v-else>
          {{
            $t('templates.noAffectedItemsSummary', {
              subject: target.kind === 'category' ? $t('templates.subjectCategory') : $t('templates.subjectTemplate'),
            })
          }}
        </p>
      </section>
    </div>
    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t('common.cancel') }}
      </button>
      <button class="danger-button" type="button" :disabled="submitting" @click="emit('submit')">
        {{
          submitting
            ? $t('templates.deletingNow')
            : target?.kind === 'item'
              ? $t('templates.deleteTemplateAndAttributes')
              : $t('templates.confirmDelete')
        }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { TemplateDomain } from "../../pages/templates/model";
import ModalDialog from "../ModalDialog.vue";

export interface TemplateDeleteTarget {
  id: number;
  name: string;
  kind: TemplateDomain;
  /** 打开确认框时从列表响应读取的当前有效物品使用数量。 */
  itemUsageCount: number | null;
}

const props = defineProps<{
  target: TemplateDeleteTarget | null;
  submitting: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [];
}>();

const { t } = useI18n();

const dialogTitle = computed(() =>
  props.target?.kind === "category"
    ? t("templates.deleteCategoryTitle")
    : t("templates.deleteTemplateTitle"),
);
</script>
