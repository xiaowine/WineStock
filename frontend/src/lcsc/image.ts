// 本文件拥有立创商品图片的浏览器直连读取与内容校验；不调用 WineStock 业务 API，也不上传文件。

const LCSC_IMAGE_HOST = "alimg.szlcsc.com";
const LCSC_IMAGE_PATH_PREFIXES = [
  "/upload/public/product/",
  "/upload/public/brand/product/certificate/",
];
const MAX_IMAGE_BYTES = 15 * 1024 * 1024;
const SUPPORTED_IMAGE_TYPES = new Set(["image/jpeg", "image/png", "image/webp"]);

/** 读取 Core 已白名单校验的立创图片，并在进入图片草稿前复核响应和文件内容。 */
export async function readLcscItemImage(
  imageUrl: string | null,
  signal?: AbortSignal,
): Promise<Blob> {
  const url = controlledLcscImageUrl(imageUrl);
  const response = await fetch(url, {
    method: "GET",
    credentials: "omit",
    redirect: "error",
    referrerPolicy: "no-referrer",
    signal,
  });
  if (!response.ok) throw new Error("立创商品图片请求失败");

  const declaredLength = response.headers.get("content-length");
  if (declaredLength !== null) {
    const size = Number(declaredLength);
    if (!Number.isSafeInteger(size) || size < 0 || size > MAX_IMAGE_BYTES) {
      throw new Error("立创商品图片大小无效");
    }
  }
  const mimeType = response.headers.get("content-type")?.split(";", 1)[0]?.trim() ?? "";
  if (!SUPPORTED_IMAGE_TYPES.has(mimeType)) throw new Error("立创商品图片格式不受支持");

  const blob = await response.blob();
  if (blob.size <= 0 || blob.size > MAX_IMAGE_BYTES) throw new Error("立创商品图片大小无效");
  if (blob.type !== mimeType) throw new Error("立创商品图片格式不一致");
  if (!(await hasMatchingImageSignature(blob, mimeType))) {
    throw new Error("立创商品图片内容无效");
  }
  return blob;
}

function controlledLcscImageUrl(source: string | null): string {
  if (!source) throw new Error("立创商品没有可用图片");
  const url = new URL(source);
  if (
    url.protocol !== "https:" ||
    url.hostname !== LCSC_IMAGE_HOST ||
    !LCSC_IMAGE_PATH_PREFIXES.some((prefix) => url.pathname.startsWith(prefix)) ||
    url.username ||
    url.password ||
    url.search ||
    url.hash
  ) {
    throw new Error("立创商品图片地址无效");
  }
  return url.href;
}

async function hasMatchingImageSignature(blob: Blob, mimeType: string): Promise<boolean> {
  const bytes = new Uint8Array(await blob.slice(0, 12).arrayBuffer());
  if (mimeType === "image/png") {
    return [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].every(
      (value, index) => bytes[index] === value,
    );
  }
  if (mimeType === "image/jpeg") {
    return bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff;
  }
  return (
    mimeType === "image/webp" && textAt(bytes, 0, 4) === "RIFF" && textAt(bytes, 8, 4) === "WEBP"
  );
}

function textAt(bytes: Uint8Array, offset: number, length: number): string {
  return String.fromCharCode(...bytes.slice(offset, offset + length));
}
