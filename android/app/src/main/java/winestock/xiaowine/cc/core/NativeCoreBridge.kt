package winestock.xiaowine.cc.core

/** 具名 JNI 入口；只交换 native protocol v1 JSON，不暴露业务 API。 */
internal object NativeCoreBridge {
    @JvmStatic external fun nativeInitialize(): String?

    @JvmStatic external fun nativeDefaultRuntimeConfig(): String?

    @JvmStatic external fun nativeValidateRuntimeConfig(requestJson: String): String?

    @JvmStatic external fun nativeStartLocalService(requestJson: String): String?

    @JvmStatic external fun nativeStopLocalService(): String?

    @JvmStatic external fun nativeRestartLocalService(requestJson: String): String?

    @JvmStatic external fun nativeGetRuntimeState(): String?

//    @JvmStatic external fun nativeShutdownEngine(): String?
}
