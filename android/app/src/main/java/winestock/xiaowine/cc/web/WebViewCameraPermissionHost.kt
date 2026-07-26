package winestock.xiaowine.cc.web

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.webkit.PermissionRequest
import androidx.core.content.ContextCompat

/**
 * WebView `getUserMedia()` 摄像头授权的 Activity 侧宿主。
 *
 * 只对受信任前端 origin 的 VIDEO_CAPTURE 请求放行：原生 CAMERA 已授权时立即 grant，
 * 否则发起运行时权限请求并在结果返回后结算；其余来源与资源一律 deny。
 * 不拥有 WebChromeClient 或运行时权限 launcher 的注册。
 */
internal class WebViewCameraPermissionHost(
    private val context: Context,
    private val trustedOrigin: String,
    private val requestCameraPermission: () -> Unit,
) {
    private var pendingRequest: PermissionRequest? = null

    /** WebChromeClient.onPermissionRequest 入口；总是同步处理，不留未结算请求。 */
    fun onPermissionRequest(request: PermissionRequest) {
        val origin = request.origin?.toString()?.trimEnd('/') ?: ""
        val wantsCamera = request.resources.contains(PermissionRequest.RESOURCE_VIDEO_CAPTURE)
        if (origin != trustedOrigin || !wantsCamera) {
            request.deny()
            return
        }

        if (hasNativeCameraPermission()) {
            grantCamera(request)
            return
        }

        // 新请求覆盖旧请求时，旧请求必须显式 deny，避免 WebView 挂起等待。
        pendingRequest?.deny()
        pendingRequest = request
        requestCameraPermission()
    }

    /** WebView 侧取消（页面关闭等）；丢弃对应的 pending 请求。 */
    fun onPermissionRequestCanceled(request: PermissionRequest?) {
        if (pendingRequest === request) {
            pendingRequest = null
        }
    }

    /** 运行时权限结果回调：按结果结算 pending 的 WebView 请求。 */
    fun onNativePermissionResult(granted: Boolean) {
        val request = pendingRequest ?: return
        pendingRequest = null
        if (granted) grantCamera(request) else request.deny()
    }

    /** Activity 销毁时以 deny 结算未完成请求。 */
    fun destroy() {
        pendingRequest?.deny()
        pendingRequest = null
    }

    private fun hasNativeCameraPermission(): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_GRANTED

    /** 只授予摄像头资源，麦克风等其余请求项不随行放行。 */
    private fun grantCamera(request: PermissionRequest) {
        request.grant(arrayOf(PermissionRequest.RESOURCE_VIDEO_CAPTURE))
    }
}
