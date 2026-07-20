package winestock.xiaowine.cc.shell

import android.content.Context
import org.json.JSONObject

/**
 * Android shell 的运行配置持久化。
 *
 * 使用版本化 SharedPreferences 记录 frontend 交来的运行配置，读取分为
 * missing / invalid / loaded 三态，与 frontend/src/shell/web.ts 的 loadPersistedConfig 一致。
 * 本文件只保存运行配置，不保存 access token、refresh token 或业务数据。
 */
class RuntimeConfigStore(context: Context) {

    private val prefs =
        context.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    /** 持久化读取结果三态。 */
    sealed interface Loaded {
        /** 从未保存过配置。 */
        data object Missing : Loaded

        /** 保存过但无法解析或结构非法，附带用于修复的回退配置。 */
        data class Invalid(val fallback: EditableRuntimeConfig) : Loaded

        /** 读到结构合法的配置（值是否有效仍需再校验）。 */
        data class Present(val config: EditableRuntimeConfig) : Loaded
    }

    /** 读取当前持久化配置。 */
    fun load(): Loaded {
        val serialized = prefs.getString(KEY_CONFIG, null) ?: return Loaded.Missing
        return try {
            val parsed = EditableRuntimeConfig.fromJsonOrNull(JSONObject(serialized))
            if (parsed != null) {
                Loaded.Present(parsed)
            } else {
                Loaded.Invalid(DEFAULT_RUNTIME_CONFIG)
            }
        } catch (_: Exception) {
            Loaded.Invalid(DEFAULT_RUNTIME_CONFIG)
        }
    }

    /** 保存配置；写入失败返回 false，由调用方映射为 config_unavailable。 */
    fun save(config: EditableRuntimeConfig): Boolean =
        try {
            prefs.edit().putString(KEY_CONFIG, config.toJson().toString()).commit()
        } catch (_: Exception) {
            false
        }

    private companion object {
        const val PREFS_NAME = "winestock.runtime"
        // 版本化 key，与 web.ts 的 winestock.runtime.config.v1 命名对齐。
        const val KEY_CONFIG = "config.v1"
    }
}
