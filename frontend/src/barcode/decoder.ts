// 本文件拥有 zxing-wasm reader 的懒加载、wasm 自托管定位和统一二维码解码入口；它不拥有摄像头取流或扫码 UI。
import type { ReadResult, ReaderOptions } from "zxing-wasm/reader";
import wasmUrl from "zxing-wasm/reader/zxing_reader.wasm?url";

/**
 * 扫码场景固定只识别 QRCode：立创料袋是二维码，收窄格式可降低误识别并加快解码。
 * 相机帧走速度优先路径：连续帧天然有重试机会，单帧不必做旋转/反色/加强尝试。
 */
const CAMERA_FRAME_OPTIONS: ReaderOptions = {
  formats: ["QRCode"],
  tryHarder: false,
  tryRotate: false,
  tryInvert: false,
  maxNumberOfSymbols: 1,
};

/** 静态图片只解码一次，走精度优先路径。 */
const STILL_IMAGE_OPTIONS: ReaderOptions = {
  formats: ["QRCode"],
  tryHarder: true,
  maxNumberOfSymbols: 2,
};

/** 已识别二维码的文本与四角位置（原始帧坐标），供扫码 UI 绘制检测框。 */
export interface DecodedQrCode {
  text: string;
  position: ReadResult["position"];
}

let readerPromise: Promise<typeof import("zxing-wasm/reader")> | null = null;

/**
 * 首次调用时动态加载 zxing-wasm reader 并初始化自托管 wasm；模块与 wasm 均不进入主包。
 * 失败时清除缓存，允许下次打开扫码时重试。
 */
export function loadBarcodeReader(): Promise<typeof import("zxing-wasm/reader")> {
  if (!readerPromise) {
    readerPromise = import("zxing-wasm/reader")
      .then(async (module) => {
        await module.prepareZXingModule({
          overrides: {
            locateFile: (path: string, prefix: string) =>
              path.endsWith(".wasm") ? wasmUrl : prefix + path,
          },
          fireImmediately: true,
        });
        return module;
      })
      .catch((error: unknown) => {
        readerPromise = null;
        throw error;
      });
  }
  return readerPromise;
}

/** 解码单帧摄像头图像；调用方应预先把帧降采样以进一步提速。 */
export function decodeQrCameraFrame(frame: ImageData): Promise<DecodedQrCode[]> {
  return decode(frame, CAMERA_FRAME_OPTIONS);
}

/** 解码静态图片文件（拍照、选图、拖放、粘贴共用）。 */
export function decodeQrImage(image: Blob): Promise<DecodedQrCode[]> {
  return decode(image, STILL_IMAGE_OPTIONS);
}

async function decode(source: ImageData | Blob, options: ReaderOptions): Promise<DecodedQrCode[]> {
  const reader = await loadBarcodeReader();
  const results = await reader.readBarcodes(source, options);
  return results
    .filter((result) => result.isValid)
    .map((result) => ({ text: result.text, position: result.position }));
}
