<!-- 本组件拥有立创候选资料的确认摘要与模板选择内容，供手动查询与扫码查询流共用；它不拥有 Dialog 结构或应用动作。 -->
<template>
  <section class="lcsc-candidate" aria-live="polite">
    <strong>{{ candidate.name }}</strong>
    <span>
      立创商品编号 {{ candidate.product_code
      }}<template v-if="candidate.manufacturer"> · {{ candidate.manufacturer }}</template>
    </span>
    <p>是否使用查询结果填写当前表单？查询结果中的有效字段将覆盖当前内容。</p>
    <FormSelect
      :id="`${panelId}-template`"
      v-model="templateId"
      class="lcsc-candidate__template"
      label="属性模板"
      match-trigger-width
      name="lcsc_template_id"
    >
      <option :value="null">不使用模板</option>
      <option v-for="template in templates" :key="template.id" :value="template.id">
        {{ template.name }}
      </option>
    </FormSelect>
  </section>
</template>

<script setup lang="ts">
import { useId } from "vue";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import type { LcscItemLookupResponse } from "../../api/items";
import FormSelect from "../forms/FormSelect.vue";

defineProps<{
  candidate: LcscItemLookupResponse;
  templates: ItemAttributeTemplateResponse[];
}>();

const templateId = defineModel<number | null>("templateId", { default: null });
const panelId = `lcsc-candidate-${useId()}`;
</script>

<style scoped lang="scss">
.lcsc-candidate {
  display: grid;
  gap: 7px;

  > strong {
    overflow-wrap: anywhere;
    font-size: 17px;
  }

  > span {
    color: var(--color-muted);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  p {
    margin: 10px 0 0;
    padding-top: 14px;
    border-top: 1px solid var(--color-border);
    line-height: 1.6;
  }

  &__template {
    margin-top: 8px;
  }
}
</style>
