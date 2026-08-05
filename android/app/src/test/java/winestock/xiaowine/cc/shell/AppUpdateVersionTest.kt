package winestock.xiaowine.cc.shell

import org.junit.Assert.assertEquals
import org.junit.Test

/** 验证 Android 更新版本比较的数字排序和非法输入边界。 */
class AppUpdateVersionTest {
    @Test
    fun comparesNumericSemanticVersions() {
        assertEquals(1, AppUpdateVersion.compare("0.1.10", "0.1.9"))
        assertEquals(0, AppUpdateVersion.compare("0.1", "0.1.0"))
        assertEquals(-1, AppUpdateVersion.compare("0.0.9", "0.1.0"))
    }

    @Test(expected = UpdateException::class)
    fun rejectsInvalidVersion() {
        AppUpdateVersion.compare("development", "0.1.0")
    }
}
