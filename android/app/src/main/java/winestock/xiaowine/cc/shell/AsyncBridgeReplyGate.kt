package winestock.xiaowine.cc.shell

/** 页面换代后拒绝旧异步 JNI/配置调用的迟到回复。 */
class AsyncBridgeReplyGate {
    private var generation = 0L
    private var destroyed = false

    fun beginPage(): Long {
        if (destroyed) return generation
        generation += 1
        return generation
    }

    fun accepts(capturedGeneration: Long): Boolean =
        !destroyed && capturedGeneration == generation

    fun destroy() {
        destroyed = true
        generation += 1
    }
}
