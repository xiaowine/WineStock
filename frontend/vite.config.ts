// 本文件拥有共享前端的 Vite 构建配置，并为 Android 打包提供隔离且可校验的生产模式。
import { isAbsolute } from "node:path";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { compression } from "vite-plugin-compression2";
import packageJson from "./package.json" with { type: "json" };

const ANDROID_MODE = "android";
const ANDROID_OUTPUT_ENV = "WINESTOCK_FRONTEND_OUT_DIR";

// 前端发布阶段徽标文案来自 package.json `appStage`，发行时改字段即可；空串/缺省则不展示。
const sharedDefine = {
  __APP_STAGE_LABEL__: JSON.stringify(
    typeof packageJson.appStage === "string" ? packageJson.appStage.trim() : "",
  ),
};

export default defineConfig(({ mode }) => {
  if (mode !== ANDROID_MODE) {
    return {
      define: sharedDefine,
      plugins: [
        vue(),
        // 仅 web 构建产出 .gz/.br 伴生文件，供未来静态部署的服务器直接送预压缩内容；
        // Android 构建不加：APK 打包本身已对 assets 做 deflate，预压缩文件只会撑大包体。
        compression({
          include: [/\.(js|mjs|css|html|svg|json|wasm)$/],
          threshold: 1024,
          algorithms: ["gzip", "brotliCompress"],
        }),
      ],
    };
  }

  const androidOutputDirectory = process.env[ANDROID_OUTPUT_ENV]?.trim();
  if (!androidOutputDirectory || !isAbsolute(androidOutputDirectory)) {
    throw new Error(`${ANDROID_OUTPUT_ENV} 必须是由 Android Gradle 构建提供的绝对输出目录`);
  }

  return {
    base: "/",
    envDir: false,
    define: sharedDefine,
    plugins: [vue()],
    build: {
      outDir: androidOutputDirectory,
      emptyOutDir: true,
      manifest: "asset-manifest.json",
      sourcemap: false,
    },
  };
});
