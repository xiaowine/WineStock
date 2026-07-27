// 本文件把 Core 选定的立创图片转换为本地主图文件，并统一处理默认占位降级；它不上传或绑定文件。

import { readLcscItemImage } from "../../lcsc/image";
import { createMissingProductImage } from "../attributes/imageDraft";

export interface PreparedLcscItemImage {
  file: File;
  usedPlaceholder: boolean;
}

/** 读取 Core 选定的商品图；地址缺失或读取失败时生成带客编的默认占位图。 */
export async function prepareLcscItemImage(
  imageUrl: string | null,
  productCode: string,
  signal?: AbortSignal,
): Promise<PreparedLcscItemImage> {
  try {
    const blob = await readLcscItemImage(imageUrl, signal);
    const extension =
      blob.type === "image/png" ? "png" : blob.type === "image/webp" ? "webp" : "jpg";
    return {
      file: new File([blob], `${productCode}.${extension}`, { type: blob.type }),
      usedPlaceholder: false,
    };
  } catch (error) {
    if (signal?.aborted) throw error;
    return {
      file: await createMissingProductImage(productCode),
      usedPlaceholder: true,
    };
  }
}
