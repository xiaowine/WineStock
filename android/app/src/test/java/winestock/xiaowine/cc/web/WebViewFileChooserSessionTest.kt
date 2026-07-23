package winestock.xiaowine.cc.web

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 验证 WebView 文件选择 pending 状态机与结果映射。
 * 契约对齐 MainActivity 的单个 ActivityResultLauncher：supersede 后再 launch 只产生一次结果，
 * 该结果必须结算**新** pending，不得被当作 stale 丢弃。
 */
class WebViewFileChooserSessionTest {

    @Test
    fun `begin stores pending and deliver settles once with uris`() {
        val session = WebViewFileChooserSession()
        val settlements = mutableListOf<Array<String>?>()

        assertTrue(session.begin { settlements += it })
        assertTrue(session.hasPending())
        assertTrue(
            session.deliver(arrayOf("content://media/1", "content://docs/2")),
        )
        assertFalse(session.hasPending())
        assertEquals(1, settlements.size)
        assertArrayEquals(
            arrayOf("content://media/1", "content://docs/2"),
            settlements.single(),
        )
        // 重复 deliver 不得二次回调
        assertFalse(session.deliver(arrayOf("content://other")))
        assertEquals(1, settlements.size)
    }

    @Test
    fun `cancel and empty deliver both settle with null`() {
        val session = WebViewFileChooserSession()
        val settlements = mutableListOf<Array<String>?>()

        assertTrue(session.begin { settlements += it })
        assertTrue(session.cancel())
        assertEquals(1, settlements.size)
        assertNull(settlements[0])

        assertTrue(session.begin { settlements += it })
        assertTrue(session.deliver(emptyArray()))
        assertEquals(2, settlements.size)
        assertNull(settlements[1])

        assertTrue(session.begin { settlements += it })
        assertTrue(session.deliver(arrayOf("  ", "")))
        assertEquals(3, settlements.size)
        assertNull(settlements[2])
    }

    @Test
    fun `new begin supersedes previous with null and next single deliver settles new callback`() {
        val session = WebViewFileChooserSession()
        val first = mutableListOf<Array<String>?>()
        val second = mutableListOf<Array<String>?>()

        assertTrue(session.begin { first += it })
        // 模拟第二次 onShowFileChooser：旧 callback 立即 null，新 callback 等待唯一 launcher 结果
        assertTrue(session.begin { second += it })

        assertEquals(1, first.size)
        assertNull(first.single())
        assertTrue(second.isEmpty())
        assertTrue(session.hasPending())

        // 单 launcher 只回传一次结果 → 必须交给新 pending，否则 WebView 文件选择会挂起
        assertTrue(session.deliver(arrayOf("content://picked-second")))
        assertFalse(session.hasPending())
        assertEquals(listOf(null), first)
        assertEquals(1, second.size)
        assertArrayEquals(arrayOf("content://picked-second"), second.single())

        assertFalse(session.deliver(arrayOf("content://extra")))
        assertEquals(1, second.size)
    }

    @Test
    fun `cancel then begin again accepts the next activity result`() {
        val session = WebViewFileChooserSession()
        val settlements = mutableListOf<Array<String>?>()

        // 启动失败路径：begin 后 cancel（无 Activity Result）
        assertTrue(session.begin { settlements += it })
        assertTrue(session.cancel())
        assertNull(settlements.single())
        assertFalse(session.hasPending())

        assertTrue(session.begin { settlements += it })
        assertTrue(session.deliver(arrayOf("content://ok")))
        assertEquals(2, settlements.size)
        assertArrayEquals(arrayOf("content://ok"), settlements[1])
    }

    @Test
    fun `destroy settles pending and rejects later deliver and begin`() {
        val session = WebViewFileChooserSession()
        val settlements = mutableListOf<Array<String>?>()

        assertTrue(session.begin { settlements += it })
        session.destroy()
        assertFalse(session.hasPending())
        assertEquals(1, settlements.size)
        assertNull(settlements.single())

        assertFalse(session.deliver(arrayOf("content://late")))
        assertEquals(1, settlements.size)

        val afterDestroy = mutableListOf<Array<String>?>()
        assertFalse(session.begin { afterDestroy += it })
        assertEquals(1, afterDestroy.size)
        assertNull(afterDestroy.single())
        // 幂等 destroy
        session.destroy()
        assertEquals(1, settlements.size)
    }

    @Test
    fun `mapChooserResult cancel empty single and multi clip`() {
        assertNull(
            WebViewFileChooserSession.mapChooserResult(
                resultOk = false,
                dataUri = "content://ignored",
                clipUris = listOf("content://a"),
            ),
        )
        assertNull(
            WebViewFileChooserSession.mapChooserResult(
                resultOk = true,
                dataUri = null,
                clipUris = emptyList(),
            ),
        )
        assertNull(
            WebViewFileChooserSession.mapChooserResult(
                resultOk = true,
                dataUri = "  ",
                clipUris = listOf("", "  "),
            ),
        )
        assertArrayEquals(
            arrayOf("content://single"),
            WebViewFileChooserSession.mapChooserResult(
                resultOk = true,
                dataUri = "content://single",
                clipUris = emptyList(),
            ),
        )
        // ClipData 优先于 data URI
        assertArrayEquals(
            arrayOf("content://a", "content://b"),
            WebViewFileChooserSession.mapChooserResult(
                resultOk = true,
                dataUri = "content://single",
                clipUris = listOf("content://a", "  ", "content://b"),
            ),
        )
    }
}
