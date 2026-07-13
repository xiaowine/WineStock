// 本文件拥有可跨业务页面复用的物品新建会话，负责元数据、草稿、上传、保存与临时文件清理；它不决定编辑器呈现方式。
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { createItem, getItem, itemOptionFromEditor, type ItemOptionResponse } from '../../api/items'
import { listItemCategories, type ItemCategoryResponse } from '../../api/itemCategories'
import { listItemAttributeTemplates, type ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
import { ApiError } from '../../api/errors'
import { notice } from '../../notices/notice'
import {
  emptyItemDraft, itemCreateRequest, itemDraftFingerprint, itemDraftValidationFromApiError,
  validateItemDraft,
} from '../../pages/items/model'
import { discardTemporaryItemFiles } from '../../pages/items/fileCleanup'
import { isImageDraftValue, uploadImageDrafts } from '../attributes/imageDraft'
import { useFormValidation } from '../../composables/useFormValidation'

/** 创建独立物品新建会话；调用方只处理打开、关闭和创建成功后的业务动作。 */
export function useItemCreateSession() {
  const draft = ref(emptyItemDraft())
  const categories = ref<ItemCategoryResponse[]>([])
  const templates = ref<ItemAttributeTemplateResponse[]>([])
  const saving = ref(false)
  const metadataError = ref('')
  const validationErrors = ref<Record<string, string>>({})
  useFormValidation(validationErrors)
  const baselineFingerprint = ref(itemDraftFingerprint(draft.value))
  let metadataController: AbortController | null = null
  let saved = false

  const hasUnsavedChanges = computed(() => itemDraftFingerprint(draft.value) !== baselineFingerprint.value)

  watch(() => itemDraftFingerprint(draft.value), () => {
    if (Object.keys(validationErrors.value).length > 0) validationErrors.value = {}
  })

  onBeforeUnmount(() => metadataController?.abort())

  /** 加载编辑器分类和属性模板；失败不阻断基础字段录入。 */
  async function loadMetadata(): Promise<void> {
    metadataController?.abort()
    const controller = new AbortController()
    metadataController = controller
    metadataError.value = ''
    try {
      const [nextCategories, nextTemplates] = await Promise.all([
        listItemCategories(controller.signal),
        listItemAttributeTemplates(controller.signal),
      ])
      if (metadataController !== controller) return
      categories.value = nextCategories
      templates.value = nextTemplates
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return
      metadataError.value = itemCreateErrorMessage(error)
      notice.error('物品编辑选项加载失败', { detail: metadataError.value })
    } finally {
      if (metadataController === controller) metadataController = null
    }
  }

  /** 上传草稿图片并创建物品；成功后保留已绑定文件，返回服务端物品快照。 */
  async function save(): Promise<ItemOptionResponse | null> {
    const validation = validateItemDraft(draft.value, templates.value)
    if (validation) {
      validationErrors.value = validation.errors
      notice.warning('请检查物品信息', { detail: validation.firstMessage })
      return null
    }
    validationErrors.value = {}
    if (!draft.value.image) return null
    saving.value = true
    try {
      await uploadImageDrafts([
        draft.value.image,
        ...draft.value.attributes.map((attribute) => attribute.value).filter(isImageDraftValue),
      ])
      const result = await createItem(itemCreateRequest(draft.value))
      const created = itemOptionFromEditor(await getItem(result.id))
      draft.value.attributes.forEach((attribute) => { attribute.fileTemporary = false })
      draft.value.imageTemporary = false
      saved = true
      return created
    } catch (error) {
      if (error instanceof ApiError) {
        const apiValidation = itemDraftValidationFromApiError(error, draft.value)
        if (apiValidation) {
          validationErrors.value = apiValidation.errors
          notice.warning('请检查物品信息', { detail: apiValidation.firstMessage })
          return null
        }
      }
      const imageError = [draft.value.image, ...draft.value.attributes.map((attribute) => attribute.value)]
        .find((value) => isImageDraftValue(value) && value.status === 'failed')
      notice.error(imageError ? '物品图片上传失败' : '保存物品失败', {
        detail: isImageDraftValue(imageError) ? imageError.error : itemCreateErrorMessage(error),
      })
      return null
    } finally {
      saving.value = false
    }
  }

  /** 放弃当前新建会话并清理所有尚未绑定的图片。 */
  async function discard(): Promise<void> {
    if (!saved) {
      try {
        await discardTemporaryItemFiles(draft.value)
      } catch {
        notice.warning('部分临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' })
      }
    }
    draft.value = emptyItemDraft()
    validationErrors.value = {}
    baselineFingerprint.value = itemDraftFingerprint(draft.value)
    saved = false
  }

  return {
    draft,
    categories,
    templates,
    saving,
    metadataError,
    validationErrors,
    hasUnsavedChanges,
    loadMetadata,
    save,
    discard,
  }
}

function itemCreateErrorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : '无法连接到 WineStock 服务'
}
