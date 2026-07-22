package winestock.xiaowine.cc.shell

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

private typealias Started = NativeBackRequestBroker.BeginResult.Started

/** 验证 native back 单 pending 状态机，不依赖 Android 主线程或 WebView。 */
class NativeBackRequestBrokerTest {

    @Test
    fun `idle request creates unique id with page generation`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()

        val first = broker.beginRequest(canGoBack = true) {} as Started
        assertEquals("page-1:1", first.request.requestId)
        assertTrue(first.request.canGoBack)
        assertEquals(400L, scheduler.tasks.single().delayMs)

        assertTrue(broker.resolve(first.request.requestId, handled = true))
        val second = broker.beginRequest(canGoBack = false) {} as Started
        assertEquals("page-1:2", second.request.requestId)
        assertFalse(second.request.canGoBack)
    }

    @Test
    fun `handled response settles once without fallback value`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()
        val settlements = mutableListOf<Boolean>()
        val request = (broker.beginRequest(true, settlements::add) as Started).request

        assertTrue(broker.resolve(request.requestId, handled = true))
        assertEquals(listOf(true), settlements)
        assertFalse(broker.resolve(request.requestId, handled = false))
        scheduler.runAll()
        assertEquals(listOf(true), settlements)
    }

    @Test
    fun `unhandled response requests fallback exactly once`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()
        val settlements = mutableListOf<Boolean>()
        val request = (broker.beginRequest(true, settlements::add) as Started).request

        assertTrue(broker.resolve(request.requestId, handled = false))
        assertEquals(listOf(false), settlements)
        scheduler.runAll()
        assertEquals(listOf(false), settlements)
    }

    @Test
    fun `timeout requests fallback and rejects late response`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()
        val settlements = mutableListOf<Boolean>()
        val request = (broker.beginRequest(true, settlements::add) as Started).request

        scheduler.runAll()

        assertEquals(listOf(false), settlements)
        assertFalse(broker.resolve(request.requestId, handled = true))
    }

    @Test
    fun `second back while awaiting is consumed without queue`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()

        val first = broker.beginRequest(true) {}
        val second = broker.beginRequest(true) {}

        assertTrue(first is Started)
        assertTrue(second is NativeBackRequestBroker.BeginResult.AlreadyPending)
        assertEquals(1, scheduler.activeTaskCount())
    }

    @Test
    fun `late response cannot settle a newer request`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()
        val settlements = mutableListOf<Boolean>()
        val first = (broker.beginRequest(true, settlements::add) as Started).request
        scheduler.runAll()
        val second = (broker.beginRequest(true, settlements::add) as Started).request

        assertFalse(broker.resolve(first.requestId, handled = true))
        assertTrue(broker.resolve(second.requestId, handled = true))
        assertEquals(listOf(false, true), settlements)
    }

    @Test
    fun `new page invalidates old request without fallback`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()
        val settlements = mutableListOf<Boolean>()
        val oldRequest = (broker.beginRequest(true, settlements::add) as Started).request

        broker.beginPage()
        scheduler.runAll()
        val newRequest = (broker.beginRequest(true, settlements::add) as Started).request

        assertTrue(settlements.isEmpty())
        assertFalse(broker.resolve(oldRequest.requestId, handled = true))
        assertEquals("page-2:2", newRequest.requestId)
        assertTrue(broker.resolve(newRequest.requestId, handled = true))
        assertEquals(listOf(true), settlements)
    }

    @Test
    fun `cancel and destroy suppress timeout callbacks and future requests`() {
        val scheduler = FakeScheduler()
        val broker = broker(scheduler)
        broker.beginPage()
        val settlements = mutableListOf<Boolean>()
        val request = (broker.beginRequest(true, settlements::add) as Started).request

        assertTrue(broker.cancelRequest(request.requestId))
        scheduler.runAll()
        assertTrue(settlements.isEmpty())

        broker.beginRequest(true, settlements::add)
        broker.destroy()
        scheduler.runAll()
        assertTrue(settlements.isEmpty())
        assertTrue(
            broker.beginRequest(true, settlements::add) is
                NativeBackRequestBroker.BeginResult.Destroyed,
        )
    }

    private fun broker(scheduler: FakeScheduler) =
        NativeBackRequestBroker(responseTimeoutMs = 400L, scheduler = scheduler)

    private class FakeScheduler : NativeBackRequestBroker.TimeoutScheduler {
        data class Task(
            val delayMs: Long,
            val action: () -> Unit,
            var cancelled: Boolean = false,
        )

        val tasks = mutableListOf<Task>()

        override fun schedule(
            delayMs: Long,
            action: () -> Unit,
        ): NativeBackRequestBroker.TimeoutHandle {
            val task = Task(delayMs = delayMs, action = action)
            tasks += task
            return NativeBackRequestBroker.TimeoutHandle { task.cancelled = true }
        }

        fun activeTaskCount(): Int = tasks.count { !it.cancelled }

        fun runAll() {
            val current = tasks.toList()
            for (task in current) {
                if (!task.cancelled) task.action()
            }
        }
    }
}
