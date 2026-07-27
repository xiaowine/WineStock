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
import kotlin.math.roundToInt

/**
 * Android WebView 视口 inset 发布与输入法避让。
 *
 * 职责：采集 WindowInsets 的系统栏与挖孔区域，把物理像素转换为 CSS 像素，
 * 并只向受信任的前端文档发布 --shell-safe-area-inset-* 变量；同时在内容容器
 * 消费 IME inset——edge-to-edge 下 `adjustResize` 失效，由本类给 WebView 容器
 * 加底部 padding 压缩 WebView 视口，让 Chromium 自行把聚焦输入框滚入可见区。
 * 已处理类型向 WebView 下发置零副本，避免新版 WebView 再次生成 CSS safe area
 * 或调整 visual viewport。键盘弹出期间安全区底边发布为 0。不拥有业务协议。
 */
internal class WebViewportInsetsPublisher(
    private val insetSource: View,
    private val imeInsetTarget: View,
    private val webView: WebView,
    private val trustedOrigin: String,
) {
    private var disposed = false
    private var hasReceivedInsets = false
    private var latestPhysicalInsets = Insets.of(0, 0, 0, 0)
    private var lastPublished: CssInsets? = null
    private var imeBottomPhysicalPx = 0
    private val handledWebViewInsetTypes =
        WindowInsetsCompat.Type.systemBars() or
            WindowInsetsCompat.Type.displayCutout() or
            WindowInsetsCompat.Type.ime()

    /** 安装监听并请求首轮 WindowInsets 分发。 */
    fun install() {
        if (disposed) return
        ViewCompat.setOnApplyWindowInsetsListener(insetSource) { _, insets ->
            if (!disposed) {
                latestPhysicalInsets = extractContentSafeInsets(insets)
                hasReceivedInsets = true
                updateImeAccommodation(insets)
                publishIfPossible(force = false)
            }
            zeroHandledInsets(insets)
        }
        ViewCompat.requestApplyInsets(insetSource)
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
        ViewCompat.setOnApplyWindowInsetsListener(insetSource, null)
    }

    /**
     * 保留 Insets 更新分发，但把 shell 已处理的类型置零后交给 WebView。
     * 不能返回 CONSUMED，否则输入法收起等后续零值通知可能被截断并留下 ghost padding。
     */
    private fun zeroHandledInsets(insets: WindowInsetsCompat): WindowInsetsCompat =
        WindowInsetsCompat.Builder(insets)
            .setInsets(handledWebViewInsetTypes, Insets.NONE)
            .build()

    /**
     * 键盘弹出/收起时压缩或还原 WebView 内容容器：edge-to-edge 关闭了框架的
     * `adjustResize`，必须由 shell 自行消费 IME inset，否则输入法直接盖住 WebView。
     * ProtectionLayout 根节点必须保持无 padding，避免底部 ColorProtection 被推到键盘上方。
     * 键盘弹出同时触发一次安全区重发（底边此时应为 0）。
     */
    private fun updateImeAccommodation(insets: WindowInsetsCompat) {
        val imeBottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
        if (imeBottom == imeBottomPhysicalPx) return
        imeBottomPhysicalPx = imeBottom
        imeInsetTarget.setPadding(
            imeInsetTarget.paddingLeft,
            imeInsetTarget.paddingTop,
            imeInsetTarget.paddingRight,
            imeBottom,
        )
        publishIfPossible(force = false)
    }

    private fun publishIfPossible(force: Boolean) {
        if (disposed || !isTrustedPage(webView.url)) return

        val cssInsets = effectiveSafeInsets().toCssInsets()
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

    /** 键盘弹出期间视口底边贴着输入法顶部，导航栏被输入法覆盖，安全区底边为 0。 */
    private fun effectiveSafeInsets(): Insets =
        if (imeBottomPhysicalPx > 0) {
            Insets.of(latestPhysicalInsets.left, latestPhysicalInsets.top, latestPhysicalInsets.right, 0)
        } else {
            latestPhysicalInsets
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

    /**
     * 合并状态栏/导航栏（含 ignoringVisibility）、挖孔、可点区与强制手势底边。
     * 底边若仍为 0，回退系统 `navigation_bar_height`，避免 edge-to-edge 下漏报导航栏高度。
     */
    private fun extractContentSafeInsets(insets: WindowInsetsCompat): Insets {
        val systemAndCutout =
            insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
            )
        val navIgnoring =
            insets.getInsetsIgnoringVisibility(WindowInsetsCompat.Type.navigationBars())
        val statusIgnoring =
            insets.getInsetsIgnoringVisibility(WindowInsetsCompat.Type.statusBars())
        val tappable = insets.getInsets(WindowInsetsCompat.Type.tappableElement())
        val mandatoryGestures = insets.getInsets(WindowInsetsCompat.Type.mandatorySystemGestures())

        val top = maxOf(systemAndCutout.top, statusIgnoring.top, tappable.top)
        val left = maxOf(systemAndCutout.left, tappable.left)
        val right = maxOf(systemAndCutout.right, tappable.right)
        var bottom =
            maxOf(
                systemAndCutout.bottom,
                navIgnoring.bottom,
                tappable.bottom,
                mandatoryGestures.bottom,
            )
        // 底边为 0 且侧边也无导航 inset 时，多半是 edge-to-edge 漏报底栏（非横屏侧栏导航）。
        val hasSideNavigation = navIgnoring.left > 0 || navIgnoring.right > 0
        if (bottom <= 0 && !hasSideNavigation) {
            bottom = navigationBarHeightFallbackPx()
        }
        return Insets.of(left, top, right, bottom)
    }

    private fun navigationBarHeightFallbackPx(): Int {
        val resources = insetSource.resources
        val resId = resources.getIdentifier("navigation_bar_height", "dimen", "android")
        if (resId > 0) {
            val value = resources.getDimensionPixelSize(resId)
            if (value > 0) return value
        }
        // 常见三键导航约 48dp；手势条更矮时若 inset 非 0 不会走到这里。
        return (48f * resources.displayMetrics.density).roundToInt()
    }

    private fun Insets.toCssInsets(): CssInsets {
        val density = insetSource.resources.displayMetrics.density.takeIf { it > 0f } ?: 1f
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
