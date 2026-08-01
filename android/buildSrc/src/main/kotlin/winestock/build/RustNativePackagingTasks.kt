// 本文件拥有 Android Rust/JNI 的离线构建、ELF 校验与 APK 包级验收，不承载运行时逻辑。
package winestock.build

import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.zip.ZipFile
import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.LocalState
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import org.gradle.work.DisableCachingByDefault

/** 使用已准备好的 cargo-ndk/NDK 离线生成单一 ABI 的 JNI shared library。 */
@DisableCachingByDefault(because = "Rust/NDK 跨机器字节确定性验证完成前只启用本地 up-to-date 检查")
abstract class RustNativeBuildTask @Inject constructor(
    private val execOperations: ExecOperations,
    private val fileSystemOperations: FileSystemOperations,
) : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sourceFiles: ConfigurableFileCollection

    @get:Internal
    abstract val repositoryDirectory: DirectoryProperty

    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val ndkDirectory: DirectoryProperty

    @get:Input
    abstract val cargoExecutable: Property<String>

    @get:Input
    abstract val cargoNdkVersion: Property<String>

    @get:Input
    abstract val cargoPackage: Property<String>

    @get:Input
    abstract val targetAbi: Property<String>

    @get:Input
    abstract val minApi: Property<Int>

    @get:Input
    abstract val release: Property<Boolean>

    @get:Input
    abstract val cargoFeatures: ListProperty<String>

    @get:Input
    abstract val sqliteCompileFlags: Property<String>

    /** cargo 的中间 target 目录，不进入 Android 打包输入。 */
    @get:LocalState
    abstract val cargoTargetDirectory: DirectoryProperty

    /** AGP variant 消费的 generated jniLibs 根目录。 */
    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @TaskAction
    fun buildNativeLibrary() {
        val repositoryRoot = repositoryDirectory.get().asFile
        requireFile(repositoryRoot.resolve("Cargo.toml"), "根 Cargo.toml")
        requireFile(repositoryRoot.resolve("Cargo.lock"), "Cargo.lock")

        val versionOutput = runCargo(listOf("ndk", "--version"), repositoryRoot)
        val expectedVersion = cargoNdkVersion.get()
        if (!versionOutput.contains(expectedVersion)) {
            throw GradleException(
                "cargo-ndk 版本不匹配：需要 $expectedVersion，实际输出为 ${versionOutput.trim()}",
            )
        }

        val outputRoot = outputDirectory.get().asFile
        fileSystemOperations.delete { delete(outputRoot) }
        outputRoot.mkdirs()
        cargoTargetDirectory.get().asFile.mkdirs()

        val arguments =
            mutableListOf(
                "ndk",
                "-t",
                targetAbi.get(),
                "-P",
                minApi.get().toString(),
                "-o",
                outputRoot.absolutePath,
                "build",
                "-p",
                cargoPackage.get(),
                "--locked",
                "--offline",
            )
        if (release.get()) {
            arguments += "--release"
        }
        if (cargoFeatures.get().isNotEmpty()) {
            arguments += "--features"
            arguments += cargoFeatures.get().joinToString(",")
        }
        runCargo(arguments, repositoryRoot)

        val library = outputRoot.resolve("${targetAbi.get()}/libwinestock_android.so")
        if (!library.isFile || library.length() <= 0L) {
            throw GradleException("cargo-ndk 未生成 ${library.path}")
        }
    }

    private fun runCargo(arguments: List<String>, workingDirectory: File): String {
        val standardOutputBuffer = ByteArrayOutputStream()
        val errorOutputBuffer = ByteArrayOutputStream()
        try {
            execOperations.exec {
                workingDir(workingDirectory)
                commandLine(listOf(cargoExecutable.get()) + arguments)
                environment("ANDROID_NDK_HOME", ndkDirectory.get().asFile.absolutePath)
                environment("CARGO_TARGET_DIR", cargoTargetDirectory.get().asFile.absolutePath)
                val sqliteFlags = sqliteCompileFlags.get().trim()
                if (sqliteFlags.isNotEmpty()) {
                    environment("LIBSQLITE3_FLAGS", sqliteFlags)
                }
                standardOutput = standardOutputBuffer
                errorOutput = errorOutputBuffer
            }.assertNormalExitValue()
        } catch (error: Exception) {
            val detail = errorOutputBuffer.toString(StandardCharsets.UTF_8).trim()
            val suffix = if (detail.isBlank()) "" else "\n$detail"
            throw GradleException(
                "Rust Android 构建失败；普通 Gradle 构建不会联网安装工具或依赖。$suffix",
                error,
            )
        }
        return standardOutputBuffer.toString(StandardCharsets.UTF_8) +
            errorOutputBuffer.toString(StandardCharsets.UTF_8)
    }

    private fun requireFile(file: File, label: String) {
        if (!file.isFile) throw GradleException("缺少 $label：${file.path}")
    }
}

/** 使用 NDK LLVM 工具验证 generated jniLibs 的 ABI、导出符号和动态依赖。 */
@DisableCachingByDefault(because = "原生库验收开销较小，保留本地 up-to-date 语义")
abstract class RustNativeVerifyTask @Inject constructor(
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val inputDirectory: DirectoryProperty

    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val ndkDirectory: DirectoryProperty

    @get:Input
    abstract val targetAbi: Property<String>

    @get:Input
    abstract val libraryFileName: Property<String>

    @get:Input
    abstract val buildProfile: Property<String>

    @get:Input
    abstract val expectedJniSymbols: ListProperty<String>

    @get:Input
    abstract val allowedNeededLibraries: ListProperty<String>

    @get:OutputFile
    abstract val verificationMarker: RegularFileProperty

    @TaskAction
    fun verifyNativeLibrary() {
        val root = inputDirectory.get().asFile
        val abi = targetAbi.get()
        val unexpectedAbiDirectories =
            root.listFiles()
                .orEmpty()
                .filter(File::isDirectory)
                .map(File::getName)
                .filter { it != abi }
        if (unexpectedAbiDirectories.isNotEmpty()) {
            throw GradleException("generated jniLibs 包含意外 ABI：${unexpectedAbiDirectories.joinToString()}")
        }

        val library = root.resolve("$abi/${libraryFileName.get()}")
        if (!library.isFile || library.length() <= 0L) {
            throw GradleException("generated jniLibs 缺少目标原生库：${library.path}")
        }
        verifyElfHeader(library)

        val readElf = findNdkTool("llvm-readelf")
        val header = runTool(readElf, listOf("-h", library.absolutePath))
        if (!header.contains("Class:") || !header.contains("ELF64") || !header.contains("Machine:") || !header.contains("AArch64")) {
            throw GradleException("${library.name} 不是 arm64-v8a 对应的 ELF64 AArch64 shared library")
        }

        val dynamic = runTool(readElf, listOf("-d", library.absolutePath))
        val neededRegex = Regex("Shared library: \\[([^]]+)]")
        val needed = neededRegex.findAll(dynamic).map { it.groupValues[1] }.toSortedSet()
        val unexpectedNeeded = needed - allowedNeededLibraries.get().toSet()
        if (unexpectedNeeded.isNotEmpty()) {
            throw GradleException("${library.name} 依赖未允许的动态库：${unexpectedNeeded.joinToString()}")
        }

        val symbols = runTool(findNdkTool("llvm-nm"), listOf("-D", "--defined-only", library.absolutePath))
        val missingSymbols = expectedJniSymbols.get().filterNot(symbols::contains)
        if (missingSymbols.isNotEmpty()) {
            throw GradleException("${library.name} 缺少 JNI 导出：${missingSymbols.joinToString()}")
        }

        val marker = verificationMarker.get().asFile
        marker.parentFile.mkdirs()
        marker.writeText(
            buildString {
                appendLine("schemaVersion=1")
                appendLine("profile=${buildProfile.get()}")
                appendLine("abi=$abi")
                appendLine("library=${library.name}")
                appendLine("sha256=${sha256(library)}")
                appendLine("bytes=${library.length()}")
                appendLine("needed=${needed.joinToString(",")}")
            },
        )
    }

    private fun verifyElfHeader(file: File) {
        val header = file.inputStream().use { input -> input.readNBytes(20) }
        if (
            header.size < 20 ||
            header[0] != 0x7f.toByte() ||
            header[1] != 'E'.code.toByte() ||
            header[2] != 'L'.code.toByte() ||
            header[3] != 'F'.code.toByte() ||
            header[4] != 2.toByte() ||
            header[5] != 1.toByte() ||
            header[18] != 0xb7.toByte() ||
            header[19] != 0.toByte()
        ) {
            throw GradleException("${file.name} ELF header 与 arm64-v8a 不一致")
        }
    }

    private fun findNdkTool(name: String): File {
        val prebuiltRoot = ndkDirectory.get().asFile.resolve("toolchains/llvm/prebuilt")
        val hostDirectory = prebuiltRoot.listFiles().orEmpty().singleOrNull(File::isDirectory)
            ?: throw GradleException("无法确定 NDK LLVM host 工具目录：${prebuiltRoot.path}")
        val suffix = if (System.getProperty("os.name").contains("Windows", ignoreCase = true)) ".exe" else ""
        val tool = hostDirectory.resolve("bin/$name$suffix")
        if (!tool.isFile) throw GradleException("NDK 缺少工具：${tool.path}")
        return tool
    }

    private fun runTool(tool: File, arguments: List<String>): String {
        val outputBuffer = ByteArrayOutputStream()
        execOperations.exec {
            commandLine(listOf(tool.absolutePath) + arguments)
            setStandardOutput(outputBuffer)
            setErrorOutput(outputBuffer)
        }.assertNormalExitValue()
        return outputBuffer.toString(StandardCharsets.UTF_8)
    }
}

/** 验证最终 APK 只打包 arm64-v8a，并包含目标 WineStock JNI 库。 */
@DisableCachingByDefault(because = "最终 APK 验收必须读取当前归档")
abstract class RustNativeApkVerifyTask : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.NAME_ONLY)
    abstract val apkFiles: ConfigurableFileCollection

    @get:Input
    abstract val targetAbi: Property<String>

    @get:Input
    abstract val libraryFileName: Property<String>

    @get:Input
    abstract val buildProfile: Property<String>

    @get:OutputFile
    abstract val reportFile: RegularFileProperty

    @TaskAction
    fun verifyApks() {
        val apks = apkFiles.files.filter(File::isFile).sortedBy(File::getName)
        if (apks.isEmpty()) throw GradleException("没有找到待验证的 APK")

        val abi = targetAbi.get()
        val targetEntry = "lib/$abi/${libraryFileName.get()}"
        val verified = mutableListOf<String>()
        apks.forEach { apk ->
            ZipFile(apk).use { zip ->
                val nativeEntries =
                    zip.entries().asSequence()
                        .filter { !it.isDirectory && it.name.startsWith("lib/") && it.name.endsWith(".so") }
                        .toList()
                val unexpectedAbis =
                    nativeEntries.mapNotNull { entry -> entry.name.split('/').getOrNull(1) }
                        .filter { it != abi }
                        .toSortedSet()
                if (unexpectedAbis.isNotEmpty()) {
                    throw GradleException("APK ${apk.name} 包含意外 ABI：${unexpectedAbis.joinToString()}")
                }
                val target = zip.getEntry(targetEntry)
                    ?: throw GradleException("APK ${apk.name} 缺少 $targetEntry")
                if (target.size <= 0L) throw GradleException("APK ${apk.name} 中 $targetEntry 为空")
                val header = zip.getInputStream(target).use { it.readNBytes(20) }
                if (header.size < 20 || header[18] != 0xb7.toByte() || header[19] != 0.toByte()) {
                    throw GradleException("APK ${apk.name} 中 $targetEntry 不是 AArch64 ELF")
                }
                verified += "apk=${apk.name};entry=$targetEntry;bytes=${target.size}"
            }
        }

        val report = reportFile.get().asFile
        report.parentFile.mkdirs()
        report.writeText(
            buildString {
                appendLine("schemaVersion=1")
                appendLine("artifactType=apk")
                appendLine("profile=${buildProfile.get()}")
                appendLine("abi=$abi")
                verified.forEach(::appendLine)
            },
        )
    }
}

private fun sha256(file: File): String {
    val digest = MessageDigest.getInstance("SHA-256")
    file.inputStream().buffered().use { input ->
        val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
        while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            digest.update(buffer, 0, read)
        }
    }
    return digest.digest().joinToString("") { byte -> "%02x".format(byte) }
}
