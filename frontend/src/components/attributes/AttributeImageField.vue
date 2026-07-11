<!-- 本组件拥有图片选择、纯色 PNG 生成和本地预览；上传由所属业务表单在提交阶段统一执行。 -->
<template>
  <div class="inbound-file-field" :class="{ 'inbound-control--error': invalid }" :title="title" :aria-label="label" tabindex="-1">
    <div class="inbound-file-field__preview" :class="{ 'inbound-file-field__preview--empty': !value }">
      <template v-if="value">
        <img v-if="value.previewUrl" :src="value.previewUrl" alt="上传图片缩略图" />
        <div>
          <strong>{{ value.name }}</strong>
          <span v-if="value.status === 'pending'">将在提交时上传</span>
          <span v-else-if="value.status === 'uploading'">上传中 {{ value.progress }}%</span>
          <span v-else-if="value.status === 'failed'">{{ value.error }}</span>
          <span v-else>{{ formatFileSize(value.sizeBytes) }}</span>
        </div>
      </template>
      <div v-else>
        <strong>尚未选择图片</strong>
        <span>PNG、JPEG 或 WebP，最大 15MB</span>
      </div>
    </div>
    <div class="inbound-file-field__source">
      <div class="inbound-file-field__source-tabs" role="group" aria-label="图片来源">
        <button
          type="button"
          :class="{ 'inbound-file-field__source-tab--active': sourceMode === 'file' }"
          :aria-pressed="sourceMode === 'file'"
          @click="sourceMode = 'file'"
        >
          本地图片
        </button>
        <button
          type="button"
          :class="{ 'inbound-file-field__source-tab--active': sourceMode === 'solid' }"
          :aria-pressed="sourceMode === 'solid'"
          @click="sourceMode = 'solid'"
        >
          纯色图片
        </button>
      </div>
      <div class="inbound-file-field__source-controls">
        <label v-if="sourceMode === 'file'" class="secondary-button inbound-file-field__file-button">
          {{ value ? '替换图片' : '选择图片' }}
          <input type="file" accept="image/png,image/jpeg,image/webp" @change="selectFile" />
        </label>
        <div v-else class="inbound-file-field__solid-controls">
          <label>
            <span>颜色</span>
            <input v-model="solidColor" type="color" aria-label="纯色图片颜色" @change="generateSolidColor" />
          </label>
        </div>
        <button v-if="value" class="text-button inbound-file-field__remove" type="button" @click="remove">删除当前图片</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, toRef, watch } from 'vue'
import { deleteImage, readImage, validateImageFile } from '../../api/files'
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from '../../api/errors'
import { notice } from '../../notices/notice'
import type { FileDraftValue } from '../../pages/inbound-draft/model'
import { createPendingImageDraft, createSolidColorImage, randomSolidColor, releaseImageDraft } from './imageDraft'

const props = withDefaults(defineProps<{ modelValue?: FileDraftValue; invalid?: boolean; title?: string; label?: string; deleteOnRemove?: boolean }>(), { deleteOnRemove: true, label: '图片属性' })
const emit = defineEmits<{ 'update:modelValue': [value: FileDraftValue | undefined] }>()
const value = toRef(props, 'modelValue')
const solidColor = ref(randomSolidColor())
const sourceMode = ref<'file' | 'solid'>(imageSourceMode(value.value))

watch(value, (next, previous) => {
  if (previous && previous !== next) releaseImageDraft(previous)
  if (next !== previous && next) sourceMode.value = imageSourceMode(next)
  if (next?.localFile && !next.previewUrl) next.previewUrl = URL.createObjectURL(next.localFile)
  else if (next?.fileId && !next.previewUrl) void loadPreview(next)
}, { immediate: true })
onBeforeUnmount(() => {
  const current = value.value
  releaseImageDraft(current)
  if (current) current.previewUrl = undefined
})

async function selectFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  const error = await validateImageFile(file)
  if (error) { notice.warning('无法选择该图片', { detail: error }); return }
  sourceMode.value = 'file'
  replace(file)
}

function replace(file: File): void {
  const current = value.value
  releaseImageDraft(current)
  if (current?.fileId && props.deleteOnRemove) void deleteImage(current.fileId).catch(() => undefined)
  emit('update:modelValue', createPendingImageDraft(file))
}

async function generateSolidColor(): Promise<void> {
  try {
    sourceMode.value = 'solid'
    replace(await createSolidColorImage(solidColor.value))
  }
  catch (error) { notice.error(errorMessage(error, '纯色图片生成失败')) }
}

async function remove(): Promise<void> {
  const current = value.value
  if (!current) return
  releaseImageDraft(current)
  emit('update:modelValue', undefined)
  if (current.fileId && props.deleteOnRemove) {
    try { await deleteImage(current.fileId) } catch (error) { notice.error(errorMessage(error, '删除临时图片失败')) }
  }
}

async function loadPreview(target: FileDraftValue): Promise<void> {
  try { target.previewUrl = URL.createObjectURL(await readImage(target.fileId as number)) }
  catch (error) { target.status = 'failed'; target.error = errorMessage(error, '无法读取已上传图片') }
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) return error.message
  if (error instanceof ApiConfigurationError) return error.message
  if (error instanceof ApiNetworkError) return '无法连接到 WineStock 服务'
  if (error instanceof ApiResponseError) return '服务响应格式无效，请检查前后端版本'
  return fallback
}

function formatFileSize(bytes: number): string {
  return bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`
}

function imageSourceMode(target: FileDraftValue | undefined): 'file' | 'solid' {
  return target?.name.startsWith('solid-') ? 'solid' : 'file'
}

</script>

<style scoped>
.inbound-file-field {
  display: grid;
  min-width: 0;
  gap: 10px;
}

.inbound-file-field.inbound-control--error {
  padding: 6px;
  border: 1px solid var(--color-danger);
  border-radius: var(--radius-sm);
  background: var(--color-danger-soft);
}

.inbound-file-field__preview {
  display: flex;
  min-width: 0;
  min-height: 64px;
  align-items: center;
  gap: 9px;
  padding: 7px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
}

.inbound-file-field__preview--empty {
  border-style: dashed;
  color: var(--color-muted);
}

.inbound-file-field__preview img {
  width: 48px;
  height: 48px;
  flex: 0 0 auto;
  border-radius: var(--radius-sm);
  object-fit: cover;
}

.inbound-file-field__preview > div {
  display: grid;
  min-width: 0;
  gap: 2px;
}

.inbound-file-field__preview strong,
.inbound-file-field__preview span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.inbound-file-field__preview span {
  color: var(--color-muted);
  font-size: 11px;
}

.inbound-file-field__source {
  display: grid;
  gap: 9px;
}

.inbound-file-field__source-tabs {
  display: inline-grid;
  width: fit-content;
  grid-template-columns: repeat(2, 1fr);
  padding: 2px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface-raised);
}

.inbound-file-field__source-tabs button {
  min-width: 88px;
  min-height: 30px;
  padding: 0 10px;
  border: 0;
  border-radius: 3px;
  background: transparent;
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 650;
}

.inbound-file-field__source-tabs button:hover {
  color: var(--color-text);
}

.inbound-file-field__source-tabs .inbound-file-field__source-tab--active {
  background: var(--color-surface);
  color: var(--color-accent-strong);
  box-shadow: 0 1px 2px rgb(23 32 42 / 10%);
}

.inbound-file-field__source-controls {
  display: flex;
  min-height: 34px;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.inbound-file-field__source-controls .secondary-button {
  min-height: 34px;
  padding: 0 10px;
  font-size: 12px;
  cursor: pointer;
}

.inbound-file-field__file-button input[type='file'] {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  clip-path: inset(50%);
  white-space: nowrap;
}

.inbound-file-field__solid-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.inbound-file-field__solid-controls label {
  display: inline-flex;
  min-height: 34px;
  align-items: center;
  gap: 7px;
  padding: 0 8px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 650;
}

.inbound-file-field__solid-controls input[type='color'] {
  width: 24px;
  height: 24px;
  padding: 1px;
  border: 0;
  border-radius: 3px;
  background: transparent;
  cursor: pointer;
}

.inbound-file-field__remove {
  margin-left: auto;
}

@media (max-width: 480px) {
  .inbound-file-field__source-tabs {
    width: 100%;
  }

  .inbound-file-field__source-controls,
  .inbound-file-field__solid-controls {
    align-items: stretch;
  }

  .inbound-file-field__source-controls .secondary-button {
    min-height: 38px;
  }

  .inbound-file-field__remove {
    margin-left: 0;
  }
}
</style>
