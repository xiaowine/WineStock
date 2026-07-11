<!-- 本组件拥有物品或入库图片属性的选择、签名预检、上传进度、缩略图、替换、重试和临时删除。 -->
<template>
  <div class="inbound-file-field" :class="{ 'inbound-control--error': invalid }" :title="title" tabindex="-1">
    <div class="inbound-file-field__preview" :class="{ 'inbound-file-field__preview--empty': !value }">
      <template v-if="value">
        <img v-if="value.previewUrl" :src="value.previewUrl" alt="上传图片缩略图" />
        <div>
          <strong>{{ value.name }}</strong>
          <span v-if="value.status === 'uploading'">上传中 {{ value.progress }}%</span>
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
      <button v-if="value?.status === 'failed'" class="text-button" type="button" @click="retry">重试</button>
      <button v-if="value" class="text-button" type="button" @click="remove">删除</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, toRef } from 'vue'
import { deleteImage, readImage, uploadImage, validateImageFile } from '../../api/files'
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from '../../api/errors'
import { notice } from '../../notices/notice'
import type { FileDraftValue } from '../../pages/inbound-draft/model'

const props = withDefaults(defineProps<{ modelValue?: FileDraftValue; invalid?: boolean; title?: string; deleteOnRemove?: boolean }>(), { deleteOnRemove: true })
const emit = defineEmits<{ 'update:modelValue': [value: FileDraftValue | undefined] }>()
const value = toRef(props, 'modelValue')

onMounted(() => { if (value.value?.fileId && !value.value.previewUrl) void loadPreview(value.value) })
onBeforeUnmount(() => {
  value.value?.abortController?.abort()
  if (value.value?.previewUrl) URL.revokeObjectURL(value.value.previewUrl)
})

async function selectFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  const error = await validateImageFile(file)
  if (error) { notice.warning('无法选择该图片', { detail: error }); return }
  await replace(file)
}

async function replace(file: File): Promise<void> {
  value.value?.abortController?.abort()
  if (value.value?.previewUrl) URL.revokeObjectURL(value.value.previewUrl)
  if (value.value?.fileId && props.deleteOnRemove) void deleteImage(value.value.fileId).catch(() => undefined)
  const next = reactive<FileDraftValue>({
    kind: 'file', name: file.name, mimeType: file.type, sizeBytes: file.size,
    status: 'uploading', progress: 0, error: '', localFile: file, previewUrl: URL.createObjectURL(file),
  })
  emit('update:modelValue', next)
  await upload(next)
}

async function upload(target: FileDraftValue): Promise<void> {
  if (!target.localFile) { target.status = 'failed'; target.error = '请重新选择图片'; return }
  const controller = new AbortController()
  target.abortController = controller
  target.status = 'uploading'
  target.error = ''
  try {
    const response = await uploadImage(target.localFile, controller.signal, (progress) => {
      target.progress = progress.percent ?? target.progress
    })
    target.fileId = response.id
    target.name = response.name
    target.mimeType = response.mime_type
    target.sizeBytes = response.size_bytes
    target.status = 'uploaded'
    target.progress = 100
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return
    target.status = 'failed'
    target.error = errorMessage(error, '上传失败，请重试')
  } finally {
    if (target.abortController === controller) target.abortController = undefined
  }
}

function retry(): void { if (value.value) void upload(value.value) }

async function remove(): Promise<void> {
  const current = value.value
  if (!current) return
  current.abortController?.abort()
  if (current.previewUrl) URL.revokeObjectURL(current.previewUrl)
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
