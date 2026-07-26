// 本文件拥有物品编辑草稿中未绑定图片的共享清理逻辑；它不保存物品资料。
import { deleteImage } from "../../api/files";
import type { ImageDraftValue } from "../../components/attributes/imageDraft";
import type { ItemAttributeDraft, ItemDraft } from "./model";
import { releaseImageDraft } from "../../components/attributes/imageDraft";

/** 中止上传、释放预览，并删除当前属性尚未绑定的服务端文件。 */
export async function discardTemporaryAttributeFile(attribute: ItemAttributeDraft): Promise<void> {
  if (!attribute.fileTemporary || !isImageDraftValue(attribute.value)) return;
  attribute.value.abortController?.abort();
  if (attribute.value.previewUrl) URL.revokeObjectURL(attribute.value.previewUrl);
  if (attribute.value.fileId) await deleteImage(attribute.value.fileId);
}

/** 并行清理整个物品草稿中的未绑定图片。 */
export async function discardTemporaryItemFiles(draft: ItemDraft): Promise<void> {
  const deletions = draft.attributes.map(discardTemporaryAttributeFile);
  if (draft.imageTemporary && draft.image) {
    releaseImageDraft(draft.image);
    if (draft.image.fileId) deletions.push(deleteImage(draft.image.fileId));
  }
  await Promise.all(deletions);
}

function isImageDraftValue(value: ItemAttributeDraft["value"]): value is ImageDraftValue {
  return typeof value === "object" && value?.kind === "file";
}
