package winestock.xiaowine.cc.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import winestock.xiaowine.cc.shell.ShellErrorCodes

class NativeLibraryLoaderTest {
    @Test
    fun successfulLoadIsMemoized() {
        var calls = 0
        val loader = NativeLibraryLoader { calls += 1 }

        assertNull(loader.ensureLoaded())
        assertNull(loader.ensureLoaded())
        assertTrue(loader.isLoaded())
        assertEquals(1, calls)
    }

    @Test
    fun failedLoadReturnsStableMemoizedError() {
        var calls = 0
        val loader =
            NativeLibraryLoader {
                calls += 1
                throw UnsatisfiedLinkError("missing")
            }

        val first = loader.ensureLoaded()
        val second = loader.ensureLoaded()

        assertEquals(ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE, first?.code)
        assertEquals(first, second)
        assertEquals(1, calls)
    }
}
