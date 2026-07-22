package winestock.xiaowine.cc.web

import android.net.Uri
import android.view.View
import android.webkit.WebView
import androidx.core.graphics.Insets
import androidx.core.net.toUri
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONObject
import java.util.Locale
import kotlin.math.abs
import kotlin.math.round

/**
 * Android WebView 安全区发布器。
 *
 * 职责：采集 WindowInsets 的系统栏与挖孔区域，把物理像素转换为 CSS 像素，
 * 并只向受信任的前端文档发布 --shell-safe-area-inset-* 变量；不消费 inset，
 * 也不拥有页面布局或业务协议。
 */
internal class WebViewportInsetsPublisher(
    private val insetTarget: View,
    private val webView: WebView,
    private val trustedOrigin: String,
) {
    private var disposed = false
    private var hasReceivedInsets = false
    private var latestPhysicalInsets = Insets.of(0, 0, 0, 0)
    private var lastPublished: CssInsets? = null

    /** 安装监听并请求首轮 WindowInsets 分发。 */
    fun install() {
        if (disposed) return
        ViewCompat.setOnApplyWindowInsetsListener(insetTarget) { _, insets ->
            if (!disposed) {
                latestPhysicalInsets =
                    insets.getInsets(
                        WindowInsetsCompat.Type.systemBars() or
                            WindowInsetsCompat.Type.displayCutout(),
                    )
                hasReceivedInsets = true
                publishIfPossible(force = false)
            }
            // 让其它 View 继续收到原始 inset；本类不执行全局 padding 或消费。
            insets
        }
        ViewCompat.requestApplyInsets(insetTarget)
    }

    /** 页面提交或加载完成后重发缓存值，覆盖页面导航造成的 CSS 变量丢失。 */
    fun onPageVisible(url: String?) {
        if (disposed || !hasReceivedInsets) return
        if (isTrustedPage(url ?: webView.url)) {
            publishIfPossible(force = true)
        }
    }

    /** 在显示密度或窗口配置变化后按当前 density 重新计算并发布。 */
    fun refresh() {
        if (disposed || !hasReceivedInsets) return
        publishIfPossible(force = true)
    }

    /** Activity 销毁时解除监听，阻止异步脚本继续写入失效页面。 */
    fun dispose() {
        if (disposed) return
        disposed = true
        ViewCompat.setOnApplyWindowInsetsListener(insetTarget, null)
    }

    private fun publishIfPossible(force: Boolean) {
        if (disposed || !isTrustedPage(webView.url)) return

        val cssInsets = latestPhysicalInsets.toCssInsets()
        if (!force && cssInsets == lastPublished) return

        val script = buildPublishScript(cssInsets)
        try {
            webView.evaluateJavascript(script, null)
            lastPublished = cssInsets
        } catch (_: RuntimeException) {
            // WebView 可能正处于销毁或导航切换，下一次 inset/page 回调会重试。
        }
    }

    private fun buildPublishScript(insets: CssInsets): String {
        // 数值由本类格式化，JSON quote 仅用于保证脚本字符串边界，不接受页面输入。
        return """
            (() => {
              const root = document.documentElement;
              if (!root) return;
              root.style.setProperty("--shell-safe-area-inset-top", ${JSONObject.quote(insets.top)});
              root.style.setProperty("--shell-safe-area-inset-right", ${JSONObject.quote(insets.right)});
              root.style.setProperty("--shell-safe-area-inset-bottom", ${JSONObject.quote(insets.bottom)});
              root.style.setProperty("--shell-safe-area-inset-left", ${JSONObject.quote(insets.left)});
            })();
        """.trimIndent()
    }

    private fun isTrustedPage(url: String?): Boolean {
        val page = url?.let { runCatching { it.toUri() }.getOrNull() } ?: return false
        val trusted = runCatching { trustedOrigin.toUri() }.getOrNull() ?: return false
        if (!page.scheme.equals(trusted.scheme, ignoreCase = true)) return false
        if (!page.host.equals(trusted.host, ignoreCase = true)) return false
        return effectivePort(page) == effectivePort(trusted)
    }

    private fun effectivePort(uri: Uri): Int =
        if (uri.port != -1) {
            uri.port
        } else if (uri.scheme.equals("https", ignoreCase = true)) {
            443
        } else if (uri.scheme.equals("http", ignoreCase = true)) {
            80
        } else {
            -1
        }

    private fun Insets.toCssInsets(): CssInsets {
        val density = insetTarget.resources.displayMetrics.density.takeIf { it > 0f } ?: 1f
        return CssInsets(
            top = toCssPx(top, density),
            right = toCssPx(right, density),
            bottom = toCssPx(bottom, density),
            left = toCssPx(left, density),
        )
    }

    private fun toCssPx(physicalPx: Int, density: Float): String {
        val cssPx = physicalPx.toFloat() / density
        val rounded = round(cssPx * 100f) / 100f
        val normalized = if (abs(rounded) < 0.01f) 0f else rounded
        return String.format(Locale.US, "%.2fpx", normalized).removeTrailingZeros()
    }

    private fun String.removeTrailingZeros(): String {
        if (!endsWith("px")) return this
        val number = removeSuffix("px").trimEnd('0').trimEnd('.')
        return "${if (number.isEmpty() || number == "-0") "0" else number}px"
    }

    private data class CssInsets(
        val top: String,
        val right: String,
        val bottom: String,
        val left: String,
    )
}
