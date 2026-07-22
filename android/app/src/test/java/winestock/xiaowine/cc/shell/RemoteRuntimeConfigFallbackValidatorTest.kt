package winestock.xiaowine.cc.shell

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RemoteRuntimeConfigFallbackValidatorTest {
    @Test
    fun normalizesIpv6WithoutAddingDuplicateBrackets() {
        assertEquals(
            "http://[::1]:17890/api",
            RemoteRuntimeConfigFallbackValidator.normalizeApiBaseUrl(
                "http://[::1]:17890/api/",
            ),
        )
    }

    @Test
    fun rejectsUnspecifiedAndQueryBearingApiAddresses() {
        assertNull(RemoteRuntimeConfigFallbackValidator.normalizeApiBaseUrl("http://0.0.0.0:17890"))
        assertNull(
            RemoteRuntimeConfigFallbackValidator.normalizeApiBaseUrl(
                "https://example.com/api?token=secret",
            ),
        )
    }
}
