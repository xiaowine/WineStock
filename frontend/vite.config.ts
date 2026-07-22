// 本文件拥有共享前端的 Vite 构建配置，并为 Android 打包提供隔离且可校验的生产模式。
import { isAbsolute } from "node:path";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const ANDROID_MODE = "android";
const ANDROID_OUTPUT_ENV = "WINESTOCK_FRONTEND_OUT_DIR";

export default defineConfig(({ mode }) => {
  if (mode !== ANDROID_MODE) {
    return {
      plugins: [vue()],
    };
  }

  const androidOutputDirectory = process.env[ANDROID_OUTPUT_ENV]?.trim();
  if (!androidOutputDirectory || !isAbsolute(androidOutputDirectory)) {
    throw new Error(`${ANDROID_OUTPUT_ENV} 必须是由 Android Gradle 构建提供的绝对输出目录`);
  }

  return {
    base: "/",
    envDir: false,
    plugins: [vue()],
    build: {
      outDir: androidOutputDirectory,
      emptyOutDir: true,
      manifest: "asset-manifest.json",
      sourcemap: false,
    },
  };
});
