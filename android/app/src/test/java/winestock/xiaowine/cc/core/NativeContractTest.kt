package winestock.xiaowine.cc.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import winestock.xiaowine.cc.shell.ShellErrorCodes

class NativeContractTest {
    @Test
    fun validationResponseParsesNormalizedConfigAndFieldErrors() {
        val result =
            NativeContract.parseValidation(
                """
                {
                  "nativeProtocolVersion": 1,
                  "ok": true,
                  "result": {
                    "valid": true,
                    "fieldErrors": {},
                    "normalizedConfig": {
                      "mode": "connect-to-remote",
                      "bindHost": "127.0.0.1",
                      "port": 17890,
                      "remoteBaseUrl": "https://example.com/api"
                    }
                  }
                }
                """.trimIndent(),
            )

        assertTrue(result is NativeCallResult.Success)
        val validation = (result as NativeCallResult.Success).value
        assertTrue(validation.valid)
        assertTrue(validation.fieldErrors.isEmpty())
        assertEquals("https://example.com/api", validation.normalizedConfig?.remoteBaseUrl)
    }

    @Test
    fun incompatibleNativeProtocolReturnsStableError() {
        val result =
            NativeContract.parseInitialize(
                """{"nativeProtocolVersion":2,"ok":true,"result":{"initialized":true}}""",
            )

        assertFalse(result is NativeCallResult.Success)
        assertEquals(
            ShellErrorCodes.BRIDGE_VERSION_MISMATCH,
            (result as NativeCallResult.Failure).error.code,
        )
    }

    @Test
    fun runningStateRequiresMatchingNonzeroPorts() {
        val valid =
            NativeContract.parseServiceState(
                serviceStateJson("127.0.0.1:49152", "http://127.0.0.1:49152"),
            )
        val mismatched =
            NativeContract.parseServiceState(
                serviceStateJson("127.0.0.1:49152", "http://127.0.0.1:49153"),
            )
        val zero =
            NativeContract.parseServiceState(
                serviceStateJson("127.0.0.1:0", "http://127.0.0.1:0"),
            )

        assertTrue(valid is NativeCallResult.Success)
        assertEquals(
            ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
            (mismatched as NativeCallResult.Failure).error.code,
        )
        assertEquals(
            ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
            (zero as NativeCallResult.Failure).error.code,
        )
    }

    private fun serviceStateJson(boundAddress: String, apiBaseUrl: String): String =
        """
        {
          "nativeProtocolVersion": 1,
          "ok": true,
          "result": {
            "phase": "running",
            "boundAddress": "$boundAddress",
            "apiBaseUrl": "$apiBaseUrl"
          }
        }
        """.trimIndent()
}
