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
    <div class="inbound-file-field__actions">
      <label class="secondary-button">
        {{ value ? '替换' : '选择图片' }}
        <input type="file" accept="image/png,image/jpeg,image/webp" @change="selectFile" />
      </label>
      <div class="inbound-file-field__color" title="未选择颜色时使用系统随机色">
        <input v-model="solidColor" type="color" aria-label="纯色图片颜色" @change="generateSolidColor" />
        <button class="secondary-button" type="button" @click="generateSolidColor">纯色图</button>
      </div>
      <button v-if="value" class="text-button" type="button" @click="remove">删除</button>
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

watch(value, (next, previous) => {
  if (previous && previous !== next) releaseImageDraft(previous)
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
  replace(file)
}

function replace(file: File): void {
  const current = value.value
  releaseImageDraft(current)
  if (current?.fileId && props.deleteOnRemove) void deleteImage(current.fileId).catch(() => undefined)
  emit('update:modelValue', createPendingImageDraft(file))
}

async function generateSolidColor(): Promise<void> {
  try { replace(await createSolidColorImage(solidColor.value)) }
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

</script>
