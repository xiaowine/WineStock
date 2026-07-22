// 本文件拥有 Android 前端构建、校验、暂存和归档验收任务，不承载应用运行时逻辑。
package winestock.build

import java.io.File
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
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import org.gradle.work.DisableCachingByDefault

/** 从当前前端源码生成 Android 专用 Vite 产物。 */
@DisableCachingByDefault(because = "跨平台字节确定性验证完成前只启用 Gradle up-to-date 检查")
abstract class FrontendBuildTask @Inject constructor(
    private val execOperations: ExecOperations,
) : DefaultTask() {
    /** 影响 Android 前端产物的受控文件集合。 */
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sourceFiles: ConfigurableFileCollection

    /** 前端工程目录，只作为 pnpm 工作目录。 */
    @get:Internal
    abstract val frontendDirectory: DirectoryProperty

    /** pnpm 可执行文件。 */
    @get:Input
    abstract val pnpmExecutable: Property<String>

    /** package.json 中的 Android 构建脚本名。 */
    @get:Input
    abstract val buildScript: Property<String>

    /** 向 Vite 传递输出目录的非客户端环境变量名。 */
    @get:Input
    abstract val outputEnvironmentVariable: Property<String>

    /** Vite 生成但尚未暂存进 Android assets 的目录。 */
    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    /** 在隔离环境中执行类型检查和 Vite Android production build。 */
    @TaskAction
    fun buildFrontend() {
        val frontendRoot = frontendDirectory.get().asFile
        if (!frontendRoot.resolve("package.json").isFile || !frontendRoot.resolve("pnpm-lock.yaml").isFile) {
            throw GradleException("frontend/package.json 与 pnpm-lock.yaml 必须存在")
        }
        if (!frontendRoot.resolve("node_modules/.modules.yaml").isFile) {
            throw GradleException(
                "前端依赖尚未准备，请执行：pnpm --dir frontend install --frozen-lockfile",
            )
        }
        val outputEnvironmentName = outputEnvironmentVariable.get()
        if (outputEnvironmentName.startsWith("VITE_", ignoreCase = true)) {
            throw GradleException("Android 构建控制变量不得使用 VITE_ 前缀")
        }
        val outputRoot = outputDirectory.get().asFile
        val sanitizedEnvironment =
            System.getenv()
                .filterKeys { key -> !key.startsWith("VITE_", ignoreCase = true) }
                .toMutableMap()
                .apply {
                    put("NODE_ENV", "production")
                    put(outputEnvironmentName, outputRoot.absolutePath)
                }

        try {
            execOperations.exec {
                workingDir(frontendRoot)
                commandLine(pnpmExecutable.get(), "run", buildScript.get())
                setEnvironment(sanitizedEnvironment)
            }
        } catch (error: Exception) {
            throw GradleException("Android 前端类型检查或 Vite 构建失败", error)
        }

        if (!outputRoot.resolve("index.html").isFile || !outputRoot.resolve("asset-manifest.json").isFile) {
            throw GradleException("Vite Android 构建未生成 index.html 或 asset-manifest.json")
        }
    }
}

/** 验证 Vite 目录，并输出不含时间戳的稳定 marker。 */
@DisableCachingByDefault(because = "校验任务开销很小，先保留本地 up-to-date 语义")
abstract class FrontendVerifyTask : DefaultTask() {
    /** 待验证的 Vite 输出目录。 */
    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val inputDirectory: DirectoryProperty

    /** 是否启用发布级 source map 等严格约束。 */
    @get:Input
    abstract val strict: Property<Boolean>

    /** 不允许泄漏到 bundle 的本机绝对路径。 */
    @get:Input
    abstract val forbiddenAbsolutePaths: ListProperty<String>

    /** 包含 manifest 摘要、文件数和体积的稳定验证结果。 */
    @get:OutputFile
    abstract val verificationMarker: RegularFileProperty

    /** 校验入口、manifest、引用、开发服务器残留和绝对路径。 */
    @TaskAction
    fun verifyFrontend() {
        val summary =
            try {
                FrontendAssetValidation.validateDirectory(
                    root = inputDirectory.get().asFile.toPath(),
                    strict = strict.get(),
                    forbiddenAbsolutePaths = forbiddenAbsolutePaths.get(),
                )
            } catch (error: Exception) {
                throw GradleException("Android 前端产物校验失败：${error.message}", error)
            }
        val marker = verificationMarker.get().asFile
        marker.parentFile.mkdirs()
        marker.writeText(
            buildString {
                appendLine("schemaVersion=1")
                appendLine("manifestSha256=${summary.manifestSha256}")
                appendLine("fileCount=${summary.fileCount}")
                appendLine("totalBytes=${summary.totalBytes}")
            },
        )
    }
}

/** 把已验证的 Vite 目录同步到 Android variant generated assets。 */
@DisableCachingByDefault(because = "Sync 暂存任务依赖 Gradle up-to-date 检查即可")
abstract class FrontendStageTask @Inject constructor(
    private val fileSystemOperations: FileSystemOperations,
) : DefaultTask() {
    /** 已构建的 Vite 输出目录。 */
    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val inputDirectory: DirectoryProperty

    /** 上游完整性校验 marker，用于建立严格生产者依赖。 */
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val verificationMarker: RegularFileProperty

    /** Android Gradle Plugin 消费的 generated assets 根目录。 */
    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    /** 清理旧 hash 文件并把产物暂存到 generated assets 的 frontend 子目录。 */
    @TaskAction
    fun stageFrontend() {
        fileSystemOperations.sync {
            from(inputDirectory) {
                into("frontend")
            }
            into(outputDirectory)
        }
        if (!outputDirectory.get().file("frontend/index.html").asFile.isFile) {
            throw GradleException("Android generated assets 缺少 frontend/index.html")
        }
    }
}

/** 禁止已废弃的源码树前端目录重新成为隐式打包来源。 */
@DisableCachingByDefault(because = "守卫任务必须观察当前源码树")
abstract class VerifyNoLegacyFrontendAssetsTask : DefaultTask() {
    /** 已废弃的 app/src/main/assets/frontend 目录。 */
    @get:Internal
    abstract val legacyDirectory: DirectoryProperty

    /** 发现 legacy 目录时立即失败。 */
    @TaskAction
    fun verifyLegacyDirectoryAbsent() {
        val directory = legacyDirectory.get().asFile
        if (directory.exists()) {
            throw GradleException(
                "检测到已废弃的 ${directory.path}；请删除它，Android 只能消费 build/generated 中的前端资源",
            )
        }
    }
}

/** 验证最终 APK/AAB 中实际打包的前端资源和 Shell Bridge。 */
@DisableCachingByDefault(because = "最终归档验收应直接检查当前 Android 产物")
abstract class FrontendArchiveVerifyTask : DefaultTask() {
    /** AGP 生成的一个或多个 APK/AAB 文件。 */
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.NAME_ONLY)
    abstract val archiveFiles: ConfigurableFileCollection

    /** 暂存阶段的权威 manifest，用于和包内摘要比对。 */
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val expectedManifest: RegularFileProperty

    /** 归档内 Android assets 根前缀；APK 为 assets，AAB 为 base/assets。 */
    @get:Input
    abstract val assetsPrefix: Property<String>

    /** 配置 split APK 是否允许不承载 base 前端资源。 */
    @get:Input
    abstract val allowArchiveWithoutFrontend: Property<Boolean>

    /** 是否启用发布级严格校验。 */
    @get:Input
    abstract val strict: Property<Boolean>

    /** 不允许泄漏进最终归档的本机绝对路径。 */
    @get:Input
    abstract val forbiddenAbsolutePaths: ListProperty<String>

    /** 最终归档验证报告。 */
    @get:OutputFile
    abstract val reportFile: RegularFileProperty

    /** 校验包内资源存在性、manifest 摘要和全部引用。 */
    @TaskAction
    fun verifyArchives() {
        val archives = archiveFiles.files.filter(File::isFile).sortedBy(File::getName)
        if (archives.isEmpty()) {
            throw GradleException("没有找到待验证的 Android 归档")
        }
        val expectedManifestHash = FrontendAssetValidation.sha256(expectedManifest.get().asFile)
        val verified = mutableListOf<Pair<File, FrontendAssetSummary>>()
        archives.forEach { archive ->
            val summary =
                try {
                    FrontendAssetValidation.validateArchive(
                        archive = archive,
                        assetsPrefix = assetsPrefix.get(),
                        strict = strict.get(),
                        forbiddenAbsolutePaths = forbiddenAbsolutePaths.get(),
                    )
                } catch (error: Exception) {
                    throw GradleException("Android 归档 ${archive.name} 前端校验失败：${error.message}", error)
                }
            if (summary == null) {
                if (!allowArchiveWithoutFrontend.get()) {
                    throw GradleException("Android 归档 ${archive.name} 缺少前端入口")
                }
                return@forEach
            }
            if (summary.manifestSha256 != expectedManifestHash) {
                throw GradleException(
                    "Android 归档 ${archive.name} 的前端 manifest 与本次暂存产物不一致",
                )
            }
            verified += archive to summary
        }
        if (verified.isEmpty()) {
            throw GradleException("所有 Android 归档都缺少 base 前端资源")
        }

        val report = reportFile.get().asFile
        report.parentFile.mkdirs()
        report.writeText(
            buildString {
                appendLine("schemaVersion=1")
                appendLine("manifestSha256=$expectedManifestHash")
                verified.forEach { (archive, summary) ->
                    appendLine(
                        "archive=${archive.name};fileCount=${summary.fileCount};totalBytes=${summary.totalBytes}",
                    )
                }
            },
        )
    }
}
