// 本文件拥有前端图片草稿、纯色 PNG 生成和延迟上传；它不决定图片最终绑定到物品还是入库明细。
import { uploadImage } from "../../api/files";
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from "../../api/errors";

const solidColorPalette = [
  "#d97757",
  "#d59b45",
  "#86a45d",
  "#4f9b83",
  "#4f91aa",
  "#657fba",
  "#8174b2",
  "#ad6f92",
];

/** 编辑期间保留的单张图片；只有业务表单提交时才会上传。 */
export interface ImageDraftValue {
  /** 区分普通模板属性的稳定判别值。 */
  kind: "file";
  /** 上传成功后由服务端返回的文件对象 ID。 */
  fileId?: number;
  /** 展示和 multipart 上传使用的文件名。 */
  name: string;
  /** 浏览器声明或服务端确认的图片 MIME。 */
  mimeType: string;
  /** 图片大小，单位字节。 */
  sizeBytes: number;
  /** 图片从本地待处理到服务端已接收的状态。 */
  status: "pending" | "uploading" | "uploaded" | "failed";
  /** 当前上传百分比；无法计算时保持最近值。 */
  progress: number;
  /** 最近一次生成、恢复或上传失败原因。 */
  error: string;
  /** 页面持有的临时 Blob URL，离开时必须释放。 */
  previewUrl?: string;
  /** 尚未上传或允许重试时保留的浏览器文件。 */
  localFile?: File;
  /** 当前上传请求的取消控制器。 */
  abortController?: AbortController;
}

/** 把用户选择或前端生成的图片转换为尚未上传的统一草稿。 */
export function createPendingImageDraft(file: File): ImageDraftValue {
  return {
    kind: "file",
    name: file.name,
    mimeType: file.type,
    sizeBytes: file.size,
    status: "pending",
    progress: 0,
    error: "",
    localFile: file,
    previewUrl: URL.createObjectURL(file),
  };
}

/** 从粘贴事件提取首个图片文件；剪贴板没有图片时返回 null，不判断粘贴目标。 */
export function extractClipboardImageFile(event: ClipboardEvent): File | null {
  const files = event.clipboardData?.files;
  if (!files?.length) return null;
  return [...files].find((file) => file.type.startsWith("image/")) ?? null;
}

/** 在浏览器内生成纯色 PNG；结果与用户选择的图片走相同上传流程。 */
export async function createSolidColorImage(color: string, size = 512): Promise<File> {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("当前浏览器无法生成纯色图片");
  context.fillStyle = color;
  context.fillRect(0, 0, size, size);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      (value) => (value ? resolve(value) : reject(new Error("纯色图片生成失败"))),
      "image/png",
    );
  });
  return new File([blob], `solid-${color.replace("#", "").toLowerCase()}.png`, {
    type: "image/png",
  });
}

/** 从受控色板随机生成默认物品主图草稿。 */
export async function createRandomSolidColorImageDraft(): Promise<ImageDraftValue> {
  const color =
    solidColorPalette[Math.floor(Math.random() * solidColorPalette.length)] ?? "#657fba";
  return createPendingImageDraft(await createSolidColorImage(color));
}

/** 返回颜色选择器的随机初始值；用户不调整时即可直接生成。 */
export function randomSolidColor(): string {
  return solidColorPalette[Math.floor(Math.random() * solidColorPalette.length)] ?? "#657fba";
}

/** 上传一张待处理图片；失败时保留本地文件，使下一次提交可以直接重试。 */
export async function uploadImageDraft(target: ImageDraftValue): Promise<void> {
  if (target.fileId && target.status === "uploaded") return;
  if (!target.localFile) {
    target.status = "failed";
    target.error = "本地图片已丢失，请重新选择";
    throw new Error(target.error);
  }
  const controller = new AbortController();
  target.abortController = controller;
  target.status = "uploading";
  target.progress = 0;
  target.error = "";
  try {
    const response = await uploadImage(target.localFile, controller.signal, (progress) => {
      target.progress = progress.percent ?? target.progress;
    });
    target.fileId = response.id;
    target.name = response.name;
    target.mimeType = response.mime_type;
    target.sizeBytes = response.size_bytes;
    target.status = "uploaded";
    target.progress = 100;
    target.localFile = undefined;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") throw error;
    target.status = "failed";
    target.error = imageUploadErrorMessage(error);
    throw error;
  } finally {
    if (target.abortController === controller) target.abortController = undefined;
  }
}

/** 并行上传全部待处理图片；任意失败都会阻止后续业务表单提交。 */
export async function uploadImageDrafts(values: Iterable<ImageDraftValue>): Promise<void> {
  const pending = [...values].filter((value) => value.status !== "uploaded" || !value.fileId);
  const results = await Promise.allSettled(pending.map(uploadImageDraft));
  const failure = results.find(
    (result): result is PromiseRejectedResult => result.status === "rejected",
  );
  if (failure) throw failure.reason;
}

/** 释放当前页面拥有的 Blob URL，并中止仍在进行的上传。 */
export function releaseImageDraft(value: ImageDraftValue | undefined): void {
  value?.abortController?.abort();
  if (value?.previewUrl) URL.revokeObjectURL(value.previewUrl);
}

export function isImageDraftValue(value: unknown): value is ImageDraftValue {
  return typeof value === "object" && value !== null && "kind" in value && value.kind === "file";
}

function imageUploadErrorMessage(error: unknown): string {
  if (error instanceof ApiError || error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError) return "无法连接到 WineStock 服务";
  if (error instanceof ApiResponseError) return "服务响应格式无效，请检查前后端版本";
  return "图片上传失败，请在提交时重试";
}
