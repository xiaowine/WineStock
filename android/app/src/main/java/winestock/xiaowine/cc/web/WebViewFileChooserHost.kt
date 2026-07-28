package winestock.xiaowine.cc.web

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import androidx.activity.result.ActivityResult

/**
 * WebView HTML 文件选择的 Activity 侧宿主。
 *
 * 会话状态在 [WebViewFileChooserSession]；本类负责启动系统选择器 Intent、
 * 解析 Activity Result，并把 `content://` URI 交回 WebView。不申请存储权限。
 */
internal class WebViewFileChooserHost(
    private val session: WebViewFileChooserSession = WebViewFileChooserSession(),
    private val launchIntent: (Intent) -> Unit,
) {
    /** Activity Result 回调：映射 URI 后只结算一次 pending。 */
    fun onActivityResult(result: ActivityResult) {
        val uriStrings =
            WebViewFileChooserSession.mapChooserResult(
                resultOk = result.resultCode == Activity.RESULT_OK,
                dataUri = result.data?.data?.toString(),
                clipUris = extractClipUris(result.data),
            )
        session.deliver(uriStrings)
    }

    /**
     * 登记 pending ValueCallback 并启动系统文件选择器。
     * 返回 true 表示宿主已接管（含启动失败时的 null 结算）。
     */
    fun onShowFileChooser(
        filePathCallback: ValueCallback<Array<Uri>>,
        fileChooserParams: WebChromeClient.FileChooserParams?,
    ): Boolean {
        val accepted =
            session.begin { uriStrings ->
                val uris = uriStrings?.map(Uri::parse)?.toTypedArray()
                filePathCallback.onReceiveValue(uris)
            }
        if (!accepted) {
            // session 已 destroy：begin 已对 WebView 回调 null，勿再 launch。
            return true
        }

        val intent =
            try {
                fileChooserParams?.createIntent()
                    ?: Intent(Intent.ACTION_GET_CONTENT).apply {
                        addCategory(Intent.CATEGORY_OPENABLE)
                        type = "*/*"
                    }
            } catch (_: Exception) {
                session.cancel()
                return true
            }

        return try {
            launchIntent(intent)
            true
        } catch (_: ActivityNotFoundException) {
            session.cancel()
            true
        } catch (_: Exception) {
            session.cancel()
            true
        }
    }

    /** WebView renderer 退出时取消旧页面请求，但保留宿主以供新 WebView 继续使用。 */
    fun cancelPending() {
        session.cancel()
    }

    /** 必须以 null 结算未完成回调，避免 WebView 挂起。 */
    fun destroy() {
        session.destroy()
    }

    companion object {
        /** 从选择器 Intent 提取多选 ClipData URI；无 ClipData 时返回空列表。 */
        fun extractClipUris(data: Intent?): List<String> {
            val clip = data?.clipData ?: return emptyList()
            val uris = ArrayList<String>(clip.itemCount)
            for (index in 0 until clip.itemCount) {
                val uri = clip.getItemAt(index)?.uri?.toString()
                if (!uri.isNullOrBlank()) {
                    uris += uri
                }
            }
            return uris
        }
    }
}
