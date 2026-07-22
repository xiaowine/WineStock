package winestock.xiaowine.cc.web

import android.content.Context
import android.webkit.WebResourceResponse
import androidx.webkit.WebViewAssetLoader
import java.io.IOException

/**
 * 把受信任 origin 的请求映射到打包在 assets/frontend 下的前端资源。
 *
 * 前端 index.html 使用 origin 根绝对路径（根、assets 目录、favicon.svg），因此这里在 assets 根之上
 * 统一加 frontend/ 前缀，并把根路径回退到 index.html。命中失败返回 null，交回 WebView 默认处理，
 * 避免把未知路径当成前端资源。
 *
 * 本文件属于 Android 平台的前端资源打包/加载职责，不服务业务 API，也不依赖 core。
 */
class FrontendPathHandler(context: Context) : WebViewAssetLoader.PathHandler {

    private val assetManager = context.applicationContext.assets

    override fun handle(path: String): WebResourceResponse? {
        // path 是 origin 根之后的部分（不含前导斜杠）；空或“/”回退到入口 HTML。
        val normalized = path.trimStart('/').ifEmpty { "index.html" }
        val assetPath = "$ASSET_ROOT/$normalized"
        return try {
            val stream = assetManager.open(assetPath)
            val mimeType = guessMimeType(normalized)
            val encoding = if (isTextMime(mimeType)) "utf-8" else null
            WebResourceResponse(mimeType, encoding, stream)
        } catch (_: IOException) {
            // 资源不存在。hash 路由不会请求真实文件，因此这里不做 SPA 回退，直接交回默认处理。
            null
        }
    }

    private fun guessMimeType(path: String): String {
        val lower = path.substringAfterLast('/').lowercase()
        val ext = lower.substringAfterLast('.', "")
        return when (ext) {
            "html", "htm" -> "text/html"
            "js", "mjs" -> "application/javascript"
            "css" -> "text/css"
            "json" -> "application/json"
            "svg" -> "image/svg+xml"
            "png" -> "image/png"
            "jpg", "jpeg" -> "image/jpeg"
            "webp" -> "image/webp"
            "gif" -> "image/gif"
            "ico" -> "image/x-icon"
            "woff2" -> "font/woff2"
            "woff" -> "font/woff"
            "ttf" -> "font/ttf"
            "map" -> "application/json"
            "txt" -> "text/plain"
            else -> "application/octet-stream"
        }
    }

    private fun isTextMime(mimeType: String): Boolean =
        mimeType.startsWith("text/") ||
            mimeType == "application/javascript" ||
            mimeType == "application/json" ||
            mimeType == "image/svg+xml"

    private companion object {
        /** 前端产物在 assets 中的相对根目录，由 Gradle variant generated assets 注册并打入包内。 */
        const val ASSET_ROOT = "frontend"
    }
}
