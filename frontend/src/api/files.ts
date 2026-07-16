// 本文件拥有物品与入库图片属性共用的 DTO、客户端签名预检和文件 HTTP 调用；它不管理页面草稿或缩略图生命周期。
import { apiClient, type ApiUploadProgress } from "./client";

/** 单张模板图片最大字节数，与服务端固定规则一致。 */
export const maxImageBytes = 15 * 1024 * 1024;
export const allowedImageTypes = ["image/png", "image/jpeg", "image/webp"] as const;

/** 图片上传后返回的稳定文件引用信息。 */
export interface ImageFileResponse {
  /** 服务端文件对象 ID。 */
  id: number;
  /** 原始文件名。 */
  name: string;
  /** 服务端确认的 MIME 类型。 */
  mime_type: string;
  /** 文件大小，单位字节。 */
  size_bytes: number;
  /** 受控读取地址。 */
  url: string;
}

/** 上传单张图片；XHR 仅用于提供真实上传进度。 */
export function uploadImage(
  file: File,
  signal: AbortSignal,
  onProgress: (progress: ApiUploadProgress) => void,
) {
  const formData = new FormData();
  formData.append("file", file, file.name);
  return apiClient.upload<ImageFileResponse>("/api/files/images", {
    formData,
    signal,
    onProgress,
  });
}

/** 读取鉴权保护的图片 Blob，供页面生成临时缩略图 URL。 */
export function readImage(fileId: number, signal?: AbortSignal) {
  return apiClient.request<Blob>(`/api/files/${fileId}`, { signal, responseType: "blob" });
}

/** 删除当前用户拥有且尚未绑定的临时图片。 */
export function deleteImage(fileId: number) {
  return apiClient.request<void>(`/api/files/${fileId}`, { method: "DELETE" });
}

/** 前端不信任扩展名，选择后读取文件头并与浏览器 MIME 声明交叉校验。 */
export async function validateImageFile(file: File): Promise<string | null> {
  if (!allowedImageTypes.includes(file.type as (typeof allowedImageTypes)[number])) {
    return "仅支持 PNG、JPEG 或 WebP 图片";
  }
  if (file.size > maxImageBytes) return "图片大小不能超过 15MB";
  const header = new Uint8Array(await file.slice(0, 12).arrayBuffer());
  const detected = detectMime(header);
  if (detected !== file.type) return "文件内容与图片类型不匹配";
  return null;
}

function detectMime(bytes: Uint8Array): string | null {
  if (
    bytes.length >= 8 &&
    [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].every((value, index) => bytes[index] === value)
  )
    return "image/png";
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff)
    return "image/jpeg";
  if (
    bytes.length >= 12 &&
    String.fromCharCode(...bytes.slice(0, 4)) === "RIFF" &&
    String.fromCharCode(...bytes.slice(8, 12)) === "WEBP"
  )
    return "image/webp";
  return null;
}
