// 本文件拥有 Android 打包前端的目录与归档完整性校验，不修改任何前端或平台运行时资源。
package winestock.build

import groovy.json.JsonSlurper
import java.io.File
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.security.MessageDigest
import java.util.zip.ZipFile

/** 已验证前端资源集合的稳定摘要。 */
internal data class FrontendAssetSummary(
    /** Vite manifest 的 SHA-256。 */
    val manifestSha256: String,
    /** 前端资源文件数量。 */
    val fileCount: Int,
    /** 前端资源未压缩总字节数。 */
    val totalBytes: Long,
)

/** 对 Vite 目录和 Android 归档中的前端资源执行同一套完整性规则。 */
internal object FrontendAssetValidation {
    private val htmlReferencePattern =
        Regex("""(?i)\b(?:src|href)\s*=\s*[\"']([^\"']+)[\"']""")
    private val uriSchemePattern = Regex("""^[A-Za-z][A-Za-z0-9+.-]*:""")
    private val windowsAbsolutePathPattern = Regex("""^[A-Za-z]:/""")
    private val textExtensions = setOf("css", "html", "js", "json", "mjs", "svg", "txt")
    private val developmentMarkers =
        listOf(
            "/@vite/client",
            "@vite/client",
            "__vite_ping",
            "vite-hmr",
            "http://localhost:5173",
            "http://127.0.0.1:5173",
            "ws://localhost:5173",
            "ws://127.0.0.1:5173",
        )

    /** 校验文件系统中的 Vite 输出目录。 */
    fun validateDirectory(
        root: Path,
        strict: Boolean,
        forbiddenAbsolutePaths: Collection<String>,
    ): FrontendAssetSummary {
        if (!Files.isDirectory(root)) {
            throw IllegalStateException("前端输出目录不存在：$root")
        }
        return validateAssetSet(
            source = DirectoryAssetSource(root),
            strict = strict,
            forbiddenAbsolutePaths = forbiddenAbsolutePaths,
        )
    }

    /**
     * 校验 APK/AAB 中的前端资源。
     *
     * 当归档不包含前端入口时返回 null，调用方可据此跳过不承载 base assets 的配置 split APK。
     */
    fun validateArchive(
        archive: File,
        assetsPrefix: String,
        strict: Boolean,
        forbiddenAbsolutePaths: Collection<String>,
    ): FrontendAssetSummary? {
        val normalizedAssetsPrefix = normalizePrefix(assetsPrefix)
        val frontendPrefix = "${normalizedAssetsPrefix}frontend/"
        ZipFile(archive).use { zip ->
            if (zip.getEntry("${frontendPrefix}index.html") == null) {
                return null
            }
            if (zip.getEntry("${normalizedAssetsPrefix}shell/bridge.js") == null) {
                throw IllegalStateException("${archive.name} 缺少 assets/shell/bridge.js")
            }
            return validateAssetSet(
                source = ZipAssetSource(zip, frontendPrefix),
                strict = strict,
                forbiddenAbsolutePaths = forbiddenAbsolutePaths,
            )
        }
    }

    /** 计算文件的稳定 SHA-256 十六进制摘要。 */
    fun sha256(file: File): String = sha256(file.readBytes())

    private fun validateAssetSet(
        source: AssetSource,
        strict: Boolean,
        forbiddenAbsolutePaths: Collection<String>,
    ): FrontendAssetSummary {
        val paths = source.paths().toSortedSet()
        val requiredFiles =
            listOf(
                "index.html",
                "asset-manifest.json",
                "favicon.svg",
                "icons.svg",
            )
        requiredFiles.forEach { path -> requireAsset(source, path) }
        if (paths.none { it.startsWith("assets/") }) {
            throw IllegalStateException("前端输出缺少 assets 目录中的构建资源")
        }

        val manifestBytes = source.readBytes("asset-manifest.json")
        val parsedManifest = JsonSlurper().parseText(manifestBytes.toString(StandardCharsets.UTF_8))
        val manifest = parsedManifest as? Map<*, *>
            ?: throw IllegalStateException("asset-manifest.json 顶层必须是 JSON 对象")
        if (manifest.isEmpty()) {
            throw IllegalStateException("asset-manifest.json 不得为空")
        }

        var hasEntry = false
        manifest.forEach { (rawKey, rawValue) ->
            val key = rawKey as? String
                ?: throw IllegalStateException("asset-manifest.json 包含非字符串键")
            val entry = rawValue as? Map<*, *>
                ?: throw IllegalStateException("manifest 条目 $key 必须是对象")
            if (entry["isEntry"] == true) {
                hasEntry = true
            }
            val outputFile = entry["file"] as? String
                ?: throw IllegalStateException("manifest 条目 $key 缺少 file")
            requireAsset(source, validateRelativePath(outputFile, "manifest file"))
            listOf("css", "assets").forEach { field ->
                stringList(entry[field], "$key.$field").forEach { referenced ->
                    requireAsset(source, validateRelativePath(referenced, "manifest $field"))
                }
            }
            listOf("imports", "dynamicImports").forEach { field ->
                stringList(entry[field], "$key.$field").forEach { importedKey ->
                    if (!manifest.containsKey(importedKey)) {
                        throw IllegalStateException("manifest 条目 $key 的 $field 引用了不存在的键：$importedKey")
                    }
                }
            }
        }
        if (!hasEntry) {
            throw IllegalStateException("asset-manifest.json 未声明任何 Vite entry")
        }

        validateIndexReferences(source)
        if (strict) {
            val sourceMaps = paths.filter { it.endsWith(".map", ignoreCase = true) }
            if (sourceMaps.isNotEmpty()) {
                throw IllegalStateException("发布前端不得包含 source map：${sourceMaps.joinToString()}")
            }
        }
        validateTextContent(source, paths, forbiddenAbsolutePaths)

        val totalBytes = paths.sumOf(source::size)
        if (totalBytes <= 0L) {
            throw IllegalStateException("前端输出总体积必须大于零")
        }
        return FrontendAssetSummary(
            manifestSha256 = sha256(manifestBytes),
            fileCount = paths.size,
            totalBytes = totalBytes,
        )
    }

    private fun validateIndexReferences(source: AssetSource) {
        val index = source.readBytes("index.html").toString(StandardCharsets.UTF_8)
        htmlReferencePattern.findAll(index).forEach { match ->
            val rawReference = match.groupValues[1].trim()
            if (rawReference.isEmpty() || rawReference.startsWith('#') || rawReference.startsWith("//")) {
                return@forEach
            }
            if (uriSchemePattern.containsMatchIn(rawReference)) {
                return@forEach
            }
            val withoutFragment = rawReference.substringBefore('#').substringBefore('?')
            if (withoutFragment.isEmpty() || withoutFragment == "/") {
                return@forEach
            }
            val relativePath = validateRelativePath(withoutFragment.trimStart('/'), "index.html 引用")
            requireAsset(source, relativePath)
        }
    }

    private fun validateTextContent(
        source: AssetSource,
        paths: Collection<String>,
        forbiddenAbsolutePaths: Collection<String>,
    ) {
        val forbiddenFragments =
            forbiddenAbsolutePaths
                .filter { it.isNotBlank() }
                .flatMap { path ->
                    val forward = path.replace('\\', '/')
                    listOf(path, forward, path.replace("\\", "\\\\"))
                }
                .distinct()

        paths.filter { path -> path.substringAfterLast('.', "").lowercase() in textExtensions }
            .forEach { path ->
                val content = source.readBytes(path).toString(StandardCharsets.UTF_8)
                developmentMarkers.firstOrNull(content::contains)?.let { marker ->
                    throw IllegalStateException("前端资源 $path 包含开发服务器标记：$marker")
                }
                forbiddenFragments.firstOrNull { fragment -> content.contains(fragment, ignoreCase = true) }
                    ?.let { fragment ->
                        throw IllegalStateException("前端资源 $path 泄漏了构建机绝对路径：$fragment")
                    }
            }
    }

    private fun stringList(value: Any?, field: String): List<String> {
        if (value == null) {
            return emptyList()
        }
        val values = value as? Collection<*>
            ?: throw IllegalStateException("manifest 字段 $field 必须是数组")
        return values.map { item ->
            item as? String ?: throw IllegalStateException("manifest 字段 $field 只能包含字符串")
        }
    }

    private fun requireAsset(source: AssetSource, path: String) {
        if (!source.exists(path)) {
            throw IllegalStateException("前端输出引用了不存在的资源：$path")
        }
        if (source.size(path) <= 0L) {
            throw IllegalStateException("前端资源不得为空：$path")
        }
    }

    private fun validateRelativePath(rawPath: String, owner: String): String {
        val slashPath = rawPath.replace('\\', '/')
        if (slashPath.startsWith('/') || windowsAbsolutePathPattern.containsMatchIn(slashPath)) {
            throw IllegalStateException("$owner 使用了绝对文件路径：$rawPath")
        }
        if (slashPath.split('/').any { it == ".." }) {
            throw IllegalStateException("$owner 试图逃出前端输出目录：$rawPath")
        }
        val normalized = Paths.get(slashPath).normalize().toString().replace('\\', '/')
        if (normalized.isBlank() || normalized == "." || normalized.startsWith("../")) {
            throw IllegalStateException("$owner 包含无效路径：$rawPath")
        }
        return normalized.removePrefix("./")
    }

    private fun normalizePrefix(prefix: String): String {
        val normalized = prefix.replace('\\', '/').trim('/')
        return if (normalized.isEmpty()) "" else "$normalized/"
    }

    private fun sha256(bytes: ByteArray): String =
        MessageDigest.getInstance("SHA-256")
            .digest(bytes)
            .joinToString(separator = "") { byte -> "%02x".format(byte) }

    private interface AssetSource {
        fun paths(): Collection<String>

        fun exists(path: String): Boolean

        fun readBytes(path: String): ByteArray

        fun size(path: String): Long
    }

    private class DirectoryAssetSource(private val root: Path) : AssetSource {
        override fun paths(): Collection<String> =
            Files.walk(root).use { stream ->
                stream.filter(Files::isRegularFile)
                    .map(root::relativize)
                    .map { it.toString().replace('\\', '/') }
                    .toList()
            }

        override fun exists(path: String): Boolean = Files.isRegularFile(resolve(path))

        override fun readBytes(path: String): ByteArray = Files.readAllBytes(resolve(path))

        override fun size(path: String): Long = Files.size(resolve(path))

        private fun resolve(path: String): Path {
            val resolved = root.resolve(path).normalize()
            if (!resolved.startsWith(root)) {
                throw IllegalStateException("前端资源路径逃出输出目录：$path")
            }
            return resolved
        }
    }

    private class ZipAssetSource(
        private val zip: ZipFile,
        private val prefix: String,
    ) : AssetSource {
        override fun paths(): Collection<String> =
            zip.entries().asSequence()
                .filter { entry -> !entry.isDirectory && entry.name.startsWith(prefix) }
                .map { entry -> entry.name.removePrefix(prefix) }
                .toList()

        override fun exists(path: String): Boolean = zip.getEntry("$prefix$path")?.isDirectory == false

        override fun readBytes(path: String): ByteArray {
            val entry = zip.getEntry("$prefix$path")
                ?: throw IllegalStateException("归档缺少前端资源：$path")
            return zip.getInputStream(entry).use { it.readBytes() }
        }

        override fun size(path: String): Long =
            zip.getEntry("$prefix$path")?.size
                ?: throw IllegalStateException("归档缺少前端资源：$path")
    }
}
