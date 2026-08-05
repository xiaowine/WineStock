package winestock.xiaowine.cc.shell

/** Android Shell 更新版本比较；只比较三段数字，避免字典序误判。 */
internal object AppUpdateVersion {
    fun isValid(value: String): Boolean = parse(value) != null

    fun compare(left: String, right: String): Int {
        val leftParts = parse(left) ?: throw UpdateException("update_manifest_invalid", "更新版本格式无效")
        val rightParts = parse(right) ?: throw UpdateException("update_manifest_invalid", "当前版本格式无效")
        return leftParts.zip(rightParts).firstOrNull { it.first != it.second }?.let { it.first.compareTo(it.second) }
            ?: leftParts.size.compareTo(rightParts.size)
    }

    private fun parse(value: String): List<Long>? {
        val parts = value.trim().substringBefore('-').split('.')
        if (parts.isEmpty() || parts.size > 3) return null
        val normalized = MutableList(3) { 0L }
        parts.forEachIndexed { index, part ->
            normalized[index] = part.toLongOrNull() ?: return null
        }
        return normalized
    }
}
