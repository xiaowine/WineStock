package winestock.xiaowine.cc.shell

import android.annotation.SuppressLint
import android.content.Context
import org.json.JSONObject

/**
 * Android shell 的运行配置持久化。
 *
 * 使用版本化 SharedPreferences 记录 frontend 交来的运行配置，读取分为
 * missing / invalid / loaded 三态，与 frontend/src/shell/transports/web.ts 的 loadPersistedConfig 一致。
 * 本文件只保存运行配置，不保存 access token、refresh token 或业务数据。
 */
interface RuntimeConfigRepository {
    /** 持久化读取结果三态。 */
    sealed interface Loaded {
        data object Missing : Loaded
        data object Invalid : Loaded
        data class Present(val config: EditableRuntimeConfig) : Loaded
    }

    fun load(): Loaded

    fun save(config: EditableRuntimeConfig): Boolean
}

class RuntimeConfigStore(context: Context) : RuntimeConfigRepository {

    private val prefs =
        context.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    /** 读取当前持久化配置。 */
    override fun load(): RuntimeConfigRepository.Loaded {
        val serialized = prefs.getString(KEY_CONFIG, null) ?: return RuntimeConfigRepository.Loaded.Missing
        return try {
            val parsed = EditableRuntimeConfig.fromJsonOrNull(JSONObject(serialized))
            if (parsed != null) {
                RuntimeConfigRepository.Loaded.Present(parsed)
            } else {
                RuntimeConfigRepository.Loaded.Invalid
            }
        } catch (_: Exception) {
            RuntimeConfigRepository.Loaded.Invalid
        }
    }

    /** 保存配置；写入失败返回 false，由调用方映射为 config_unavailable。 */
    @SuppressLint("ApplySharedPref", "UseKtx")
    override fun save(config: EditableRuntimeConfig): Boolean =
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
