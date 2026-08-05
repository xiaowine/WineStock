package winestock.xiaowine.cc.shell

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import org.json.JSONObject
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.security.MessageDigest

/**
 * Android Shell 的更新清单、APK 下载和系统安装器协调器。
 *
 * 本类只访问固定的更新清单，不把网络请求交给 WebView；安装文件只进入应用私有缓存，
 * 再通过受控 FileProvider URI 交给系统 Package Installer。它不修改 core 数据、不拥有前端 UI。
 */
class AppUpdateManager(
    context: Context,
    private val currentVersion: String,
) {
    private val appContext = context.applicationContext

    fun checkForUpdate(): AppUpdateCheckResult {
        val manifest = fetchManifest()
        val comparison = AppUpdateVersion.compare(manifest.version, currentVersion)
        if (comparison <= 0) {
            return AppUpdateCheckResult(currentVersion = currentVersion)
        }
        return AppUpdateCheckResult(
            currentVersion = currentVersion,
            latestVersion = manifest.version,
            notes = manifest.notes.takeIf { it.isNotBlank() },
        )
    }

    fun installUpdate(expectedVersion: String) {
        val manifest = fetchManifest()
        if (manifest.version != expectedVersion || AppUpdateVersion.compare(manifest.version, currentVersion) <= 0) {
            throw UpdateException("update_not_available", "请求安装的版本已经不可用，请重新检查更新")
        }

        ensureInstallPermission()
        val apkFile = downloadApk(manifest)
        val uri = try {
            FileProvider.getUriForFile(
                appContext,
                "${appContext.packageName}.fileprovider",
                apkFile,
            )
        } catch (_: Exception) {
            throw UpdateException("update_install_failed", "无法准备 Android 安装文件")
        }
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, APK_MIME_TYPE)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        try {
            appContext.startActivity(intent)
        } catch (_: Exception) {
            throw UpdateException("update_install_failed", "无法启动系统安装器")
        }
    }

    private fun ensureInstallPermission() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O || appContext.packageManager.canRequestPackageInstalls()) {
            return
        }
        val settingsIntent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES).apply {
            data = Uri.parse("package:${appContext.packageName}")
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        try {
            appContext.startActivity(settingsIntent)
        } catch (_: Exception) {
            // 部分定制系统没有单应用设置页；仍返回同一稳定码，让前端提供人工恢复提示。
        }
        throw UpdateException(
            "update_install_permission_required",
            "请在系统设置中允许 WineStock 安装未知来源应用后重试",
        )
    }

    private fun fetchManifest(): UpdateManifest {
        val connection = openConnection(MANIFEST_URL, MANIFEST_TIMEOUT_MS)
        try {
            val responseCode = connection.responseCode
            if (responseCode !in 200..299) {
                throw UpdateException("update_check_unavailable", "更新服务暂时不可用")
            }
            val contentType = connection.contentType?.lowercase().orEmpty()
            if (!contentType.startsWith("application/json") && !contentType.contains("+json")) {
                throw UpdateException("update_manifest_invalid", "更新服务返回了无效内容类型")
            }
            val body = connection.inputStream.use { it.readLimited(MAX_MANIFEST_BYTES) }
            val json = try {
                JSONObject(body.toString(Charsets.UTF_8))
            } catch (_: Exception) {
                throw UpdateException("update_manifest_invalid", "更新清单格式无效")
            }
            val manifest = try {
                val android = json.getJSONObject("android")
                UpdateManifest(
                    version = json.getString("version").trim(),
                    baseUrl = json.getString("baseUrl").trim(),
                    file = android.getString("file").trim(),
                    sha256 = android.getString("sha256").trim().lowercase(),
                    notes = json.optString("notes", ""),
                )
            } catch (_: Exception) {
                throw UpdateException("update_manifest_invalid", "更新清单缺少必要字段")
            }
            validateManifest(manifest)
            return manifest
        } catch (error: UpdateException) {
            throw error
        } catch (_: Exception) {
            throw UpdateException("update_check_unavailable", "暂时无法连接更新服务")
        } finally {
            connection.disconnect()
        }
    }

    private fun downloadApk(manifest: UpdateManifest): File {
        val connection = openConnection(releaseAssetUrl(manifest), DOWNLOAD_TIMEOUT_MS)
        val directory = File(appContext.cacheDir, UPDATE_CACHE_DIRECTORY)
        val destination = File(directory, "winestock-update-${manifest.version}.apk")
        val temporary = File(directory, "${destination.name}.part")
        try {
            if (connection.responseCode !in 200..299) {
                throw UpdateException("update_download_failed", "更新 APK 下载失败")
            }
            val contentLength = connection.contentLengthLong
            if (contentLength > MAX_APK_BYTES) {
                throw UpdateException("update_download_failed", "更新 APK 超过允许大小")
            }
            directory.mkdirs()
            var total = 0L
            FileOutputStream(temporary).use { output ->
                connection.inputStream.use { input ->
                    val buffer = ByteArray(DOWNLOAD_BUFFER_SIZE)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        total += read
                        if (total > MAX_APK_BYTES) {
                            throw UpdateException("update_download_failed", "更新 APK 超过允许大小")
                        }
                        output.write(buffer, 0, read)
                    }
                }
            }
            if (sha256(temporary) != manifest.sha256) {
                throw UpdateException("update_integrity_failed", "更新 APK 校验失败")
            }
            if (destination.exists() && !destination.delete()) {
                throw UpdateException("update_download_failed", "无法替换缓存中的更新 APK")
            }
            if (!temporary.renameTo(destination)) {
                throw UpdateException("update_download_failed", "无法保存更新 APK")
            }
            return destination
        } catch (error: UpdateException) {
            temporary.delete()
            throw error
        } catch (_: Exception) {
            temporary.delete()
            throw UpdateException("update_download_failed", "更新 APK 下载失败")
        } finally {
            connection.disconnect()
        }
    }

    private fun validateManifest(manifest: UpdateManifest) {
        if (!AppUpdateVersion.isValid(manifest.version)) {
            throw UpdateException("update_manifest_invalid", "更新版本格式无效")
        }
        val assetUrl = releaseAssetUrl(manifest)
        if (!assetUrl.endsWith(".apk", ignoreCase = true)) {
            throw UpdateException("update_manifest_invalid", "更新 APK 地址无效")
        }
        if (manifest.sha256.length != SHA256_HEX_LENGTH || manifest.sha256.any { !it.isDigit() && it !in 'a'..'f' }) {
            throw UpdateException("update_manifest_invalid", "更新摘要格式无效")
        }
    }

    private fun openConnection(url: String, timeoutMs: Int): HttpURLConnection {
        val connection = try {
            URL(url).openConnection() as HttpURLConnection
        } catch (_: Exception) {
            throw UpdateException("update_check_unavailable", "更新地址无效")
        }
        connection.connectTimeout = timeoutMs
        connection.readTimeout = timeoutMs
        connection.requestMethod = "GET"
        connection.instanceFollowRedirects = true
        connection.setRequestProperty("Accept", "application/json")
        connection.setRequestProperty("User-Agent", "WineStock Android/$currentVersion")
        return connection
    }

    /** 由受控基础地址和相对文件名生成 APK 下载地址，禁止清单改变主机或逃逸目录。 */
    private fun releaseAssetUrl(manifest: UpdateManifest): String {
        val base = try {
            URI(manifest.baseUrl)
        } catch (_: Exception) {
            throw UpdateException("update_manifest_invalid", "更新基础地址无效")
        }
        if (base.scheme != "https" || base.host.isNullOrBlank() || base.userInfo != null || base.query != null || base.fragment != null) {
            throw UpdateException("update_manifest_invalid", "更新基础地址必须使用不含凭据的 HTTPS")
        }
        val file = manifest.file
        if (file.isBlank() || file.startsWith('/') || file.contains('?') || file.contains('#') || file.split('/').any { it == "." || it == ".." }) {
            throw UpdateException("update_manifest_invalid", "更新文件名无效")
        }
        return try {
            URI("${manifest.baseUrl.trimEnd('/')}/").resolve(file).toString()
        } catch (_: Exception) {
            throw UpdateException("update_manifest_invalid", "更新文件地址无效")
        }
    }

    private fun sha256(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        FileInputStream(file).use { input ->
            val buffer = ByteArray(DOWNLOAD_BUFFER_SIZE)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { byte -> "%02x".format(byte.toInt() and 0xff) }
    }

    private data class UpdateManifest(
        val version: String,
        val baseUrl: String,
        val file: String,
        val sha256: String,
        val notes: String,
    )

    companion object {
        const val MANIFEST_URL = "https://api.ikuns.top/WineRealm/file/winestock/winestock.json"
        private const val MANIFEST_TIMEOUT_MS = 10_000
        private const val DOWNLOAD_TIMEOUT_MS = 120_000
        private const val MAX_MANIFEST_BYTES = 256 * 1024L
        private const val MAX_APK_BYTES = 512 * 1024 * 1024L
        private const val DOWNLOAD_BUFFER_SIZE = 16 * 1024
        private const val UPDATE_CACHE_DIRECTORY = "winestock-updates"
        private const val APK_MIME_TYPE = "application/vnd.android.package-archive"
        private const val SHA256_HEX_LENGTH = 64
    }
}

data class AppUpdateCheckResult(
    val currentVersion: String,
    val latestVersion: String? = null,
    val notes: String? = null,
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("currentVersion", currentVersion)
        latestVersion?.let { put("latestVersion", it) }
        notes?.let { put("notes", it) }
    }
}

class UpdateException(
    val code: String,
    override val message: String,
) : Exception(message)

private fun java.io.InputStream.readLimited(maxBytes: Long): ByteArray {
    val output = java.io.ByteArrayOutputStream()
    val buffer = ByteArray(16 * 1024)
    var total = 0L
    while (true) {
        val read = read(buffer)
        if (read < 0) break
        total += read
        if (total > maxBytes) throw UpdateException("update_manifest_invalid", "更新清单超过允许大小")
        output.write(buffer, 0, read)
    }
    return output.toByteArray()
}
