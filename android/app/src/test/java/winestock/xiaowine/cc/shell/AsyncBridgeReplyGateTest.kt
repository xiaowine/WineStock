package winestock.xiaowine.cc.shell

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AsyncBridgeReplyGateTest {
    @Test
    fun pageChangeRejectsLateReply() {
        val gate = AsyncBridgeReplyGate()
        val oldPage = gate.beginPage()
        val currentPage = gate.beginPage()

        assertFalse(gate.accepts(oldPage))
        assertTrue(gate.accepts(currentPage))
    }

    @Test
    fun destroyRejectsCurrentReply() {
        val gate = AsyncBridgeReplyGate()
        val currentPage = gate.beginPage()

        gate.destroy()

        assertFalse(gate.accepts(currentPage))
    }
}
