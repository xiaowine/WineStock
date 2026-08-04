// 本文件拥有捐赠二维码的懒加载生成和 WASM 自托管定位；不拥有捐赠内容配置或图片展示。
import writerWasmAssetUrl from "zxing-wasm/writer/zxing_writer.wasm?url";

// Vite dev server may append `?import&url` to dependency assets; that response is a JS
// asset module, while ZXing needs the underlying WASM binary URL.
const writerWasmUrl = writerWasmAssetUrl.replace(/\?import&url$/, "");

let writerPromise: Promise<typeof import("zxing-wasm/writer")> | null = null;

export function loadDonationQrWriter(): Promise<typeof import("zxing-wasm/writer")> {
  if (!writerPromise) {
    writerPromise = import("zxing-wasm/writer")
      .then(async (module) => {
        await module.prepareZXingModule({
          overrides: {
            locateFile: (path: string, prefix: string) =>
              path.endsWith(".wasm") ? writerWasmUrl : prefix + path,
          },
          fireImmediately: true,
        });
        return module;
      })
      .catch((error: unknown) => {
        writerPromise = null;
        throw error;
      });
  }
  return writerPromise;
}

export async function generateDonationQr(content: string): Promise<Blob> {
  const writer = await loadDonationQrWriter();
  const output = await writer.writeBarcode(content, {
    format: "QRCode",
    scale: 4,
    addQuietZones: false,
    addHRT: false,
    options: "ecLevel=H",
  });
  if (output.error || !output.image) {
    throw new Error(output.error || "二维码生成失败");
  }
  return output.image;
}
