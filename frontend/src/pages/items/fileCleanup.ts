// 本文件拥有物品编辑草稿中未绑定图片的清理，属于 frontend 页面协作层；它不保存物品资料。
import { deleteImage } from '../../api/files'
import type { FileDraftValue } from '../inbound-draft/model'
import type { ItemAttributeDraft, ItemDraft } from './model'

/** 中止上传、释放预览，并删除当前属性尚未绑定的服务端文件。 */
export async function discardTemporaryAttributeFile(attribute: ItemAttributeDraft): Promise<void> {
  if (!attribute.fileTemporary || !isFileDraftValue(attribute.value)) return
  attribute.value.abortController?.abort()
  if (attribute.value.previewUrl) URL.revokeObjectURL(attribute.value.previewUrl)
  if (attribute.value.fileId) await deleteImage(attribute.value.fileId)
}

/** 并行清理整个物品草稿中的未绑定图片。 */
export async function discardTemporaryItemFiles(draft: ItemDraft): Promise<void> {
  await Promise.all(draft.attributes.map(discardTemporaryAttributeFile))
}

function isFileDraftValue(value: ItemAttributeDraft['value']): value is FileDraftValue {
  return typeof value === 'object' && value?.kind === 'file'
}
