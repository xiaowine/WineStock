package winestock.xiaowine.cc.core

import android.content.Context
import winestock.xiaowine.cc.shell.ShellErrorCodes
import winestock.xiaowine.cc.shell.ShellRuntimeError

/** 把 core 存储固定在 no-backup app-private 目录，并在后台启动前预创建目录。 */
object AndroidStoragePaths {
    fun prepare(context: Context): NativeCallResult<NativeStoragePaths> {
        val dataDirectory = context.noBackupFilesDir.resolve("winestock/data")
        val filesDirectory = dataDirectory.resolve("files")
        return try {
            if ((!dataDirectory.isDirectory && !dataDirectory.mkdirs()) ||
                (!filesDirectory.isDirectory && !filesDirectory.mkdirs())
            ) {
                return NativeCallResult.Failure(storageError())
            }
            NativeCallResult.Success(
                NativeStoragePaths(
                    databasePath = dataDirectory.resolve("winestock.sqlite").absolutePath,
                    filesDir = filesDirectory.absolutePath,
                ),
            )
        } catch (_: SecurityException) {
            NativeCallResult.Failure(storageError())
        }
    }

    private fun storageError() =
        ShellRuntimeError(
            ShellErrorCodes.STORAGE_UNAVAILABLE,
            "无法准备 Android 本地存储目录",
        )
}
