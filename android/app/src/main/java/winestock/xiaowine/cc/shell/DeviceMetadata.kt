package winestock.xiaowine.cc.shell

import android.content.Context
import android.os.Build

/** Shell Bridge 注入用的设备与应用版本标签；无业务含义。 */
internal object DeviceMetadata {
    fun resolveDeviceName(): String {
        val manufacturer = Build.MANUFACTURER?.replaceFirstChar { it.uppercase() }.orEmpty()
        val model = Build.MODEL.orEmpty()
        val label = listOf(manufacturer, model).filter { it.isNotBlank() }.joinToString(" ")
        return label.ifBlank { "WineStock Android" }
    }

    fun resolveAppVersion(context: Context): String =
        try {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "unknown"
        } catch (_: Exception) {
            "unknown"
        }
}
