import com.android.build.api.artifact.SingleArtifact
import winestock.build.FrontendArchiveVerifyTask
import winestock.build.FrontendBuildTask
import winestock.build.FrontendStageTask
import winestock.build.FrontendVerifyTask
import winestock.build.RustNativeApkVerifyTask
import winestock.build.RustNativeBuildTask
import winestock.build.RustNativeVerifyTask
import winestock.build.VerifyNoLegacyFrontendAssetsTask
import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
}

// Release APK 必须使用正式证书；凭据只从未入库的本地文件或 CI 环境读取。
val signingPropertiesFile = rootProject.file("keystore.properties")
val signingProperties =
    Properties().apply {
        if (signingPropertiesFile.isFile) {
            signingPropertiesFile.inputStream().use(::load)
        }
    }

fun signingValue(propertyName: String, environmentName: String): String? =
    providers.gradleProperty(propertyName).orNull?.trim()?.takeIf { it.isNotEmpty() }
        ?: providers.environmentVariable(environmentName).orNull?.trim()?.takeIf { it.isNotEmpty() }
        ?: signingProperties.getProperty(propertyName)?.trim()?.takeIf { it.isNotEmpty() }

val releaseKeystoreFile = signingValue("winestock.android.keystoreFile", "WINSTOCK_ANDROID_KEYSTORE_FILE")
val releaseKeystorePassword =
    signingValue("winestock.android.keystorePassword", "WINSTOCK_ANDROID_KEYSTORE_PASSWORD")
val releaseKeyAlias = signingValue("winestock.android.keyAlias", "WINSTOCK_ANDROID_KEY_ALIAS")
val releaseKeyPassword = signingValue("winestock.android.keyPassword", "WINSTOCK_ANDROID_KEY_PASSWORD")
val releaseSigningValues =
    listOf(releaseKeystoreFile, releaseKeystorePassword, releaseKeyAlias, releaseKeyPassword)
val releaseSigningConfigured = releaseSigningValues.all { it != null }
val releaseSigningPartiallyConfigured = releaseSigningValues.any { it != null } && !releaseSigningConfigured
val releaseTaskRequested =
    gradle.startParameter.taskNames.any { taskName ->
        val leafTaskName = taskName.substringAfterLast(":")
        leafTaskName.equals("assemble", ignoreCase = true) ||
            leafTaskName.equals("build", ignoreCase = true) ||
            leafTaskName.contains("release", ignoreCase = true)
    }

if (releaseSigningPartiallyConfigured) {
    throw GradleException(
        "Android Release 签名配置不完整；需要同时提供 keystoreFile、keystorePassword、keyAlias 和 keyPassword。",
    )
}
if (releaseTaskRequested && !releaseSigningConfigured) {
    throw GradleException(
        "Android Release APK 必须签名。请复制 android/keystore.properties.example 为 " +
            "android/keystore.properties，或通过 WINSTOCK_ANDROID_* 环境变量提供签名配置。",
    )
}
val releaseKeystore = releaseKeystoreFile?.let { rootProject.file(it) }
if (releaseSigningConfigured && releaseKeystore?.isFile != true) {
    throw GradleException("Android Release keystore 不存在：${releaseKeystore?.absolutePath}")
}

/**
 * 读取 Cargo 工作区的发布版本。
 *
 * 根 Cargo.toml 是 Desktop、Server 与 Android 的唯一发布版本来源；这里仅识别
 * [workspace.package] 的稳定三段数字版本，格式变更时应显式更新此受限读取器。
 */
fun readWorkspaceReleaseVersion(cargoManifest: File): String {
    var inWorkspacePackage = false
    cargoManifest.forEachLine { sourceLine ->
        val line = sourceLine.trim()
        if (line == "[workspace.package]") {
            inWorkspacePackage = true
            return@forEachLine
        }
        if (inWorkspacePackage && line.startsWith("[")) {
            inWorkspacePackage = false
        }
        if (inWorkspacePackage) {
            Regex("""^version\\s*=\\s*\"([0-9]+\\.[0-9]+\\.[0-9]+)\"(?:\\s*#.*)?$""")
                .matchEntire(line)
                ?.groupValues
                ?.get(1)
                ?.let { return it }
        }
    }
    throw GradleException("根 Cargo.toml 的 [workspace.package] 必须声明三段数字 version")
}

/** 将共享语义版本映射为 Android Package Manager 要求的严格递增内部版本号。 */
fun androidVersionCode(releaseVersion: String): Int {
    val parts = releaseVersion.split('.').map(String::toInt)
    require(parts.all { it in 0..999 }) { "发布版本每一段必须介于 0 和 999：$releaseVersion" }
    return parts[0] * 1_000_000 + parts[1] * 1_000 + parts[2]
}

val workspaceReleaseVersion = readWorkspaceReleaseVersion(rootProject.file("../Cargo.toml"))
val workspaceAndroidVersionCode = androidVersionCode(workspaceReleaseVersion)

android {
    namespace = "winestock.xiaowine.cc"
    ndkVersion = "30.0.14904198"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "winestock.xiaowine.cc"
        minSdk = 28
        targetSdk = 36
        // Android 只从根 Cargo 工作区派生版本，避免与 Desktop/Server 发行版本漂移。
        versionCode = workspaceAndroidVersionCode
        versionName = workspaceReleaseVersion

        ndk {
            // 当前发布面只支持真实 ARM64 设备；不生成 32 位或模拟器 ABI。
            abiFilters += "arm64-v8a"
        }
    }

    signingConfigs {
        if (releaseSigningConfigured) {
            create("release") {
                storeFile = releaseKeystore
                storePassword = requireNotNull(releaseKeystorePassword)
                keyAlias = requireNotNull(releaseKeyAlias)
                keyPassword = requireNotNull(releaseKeyPassword)
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            if (releaseSigningConfigured) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        viewBinding = true
    }
    dependenciesInfo {
        includeInApk = false
    }
    packaging {
        resources {
            excludes += "DebugProbesKt.bin"
        }
    }
    lint {
        // 当前产品范围明确只支持 ARM64 Android 真实设备，不承诺 ChromeOS x86 ABI。
        disable += "ChromeOsAbiSupport"
    }
}

// Android 只消费本次 Gradle 构建生成并校验过的前端资源；frontend/dist 不属于 Android 输入。
val frontendProjectDirectory = rootProject.layout.projectDirectory.dir("../frontend")
val repositoryDirectory = rootProject.layout.projectDirectory.dir("..")
val isWindows = System.getProperty("os.name").contains("Windows", ignoreCase = true)
val localPnpmExecutable =
    providers.gradleProperty("winestock.pnpmExecutable")
        .orElse(if (isWindows) "pnpm.cmd" else "pnpm")
val forbiddenFrontendPaths =
    listOf(
        repositoryDirectory.asFile.absolutePath,
        rootProject.layout.projectDirectory.asFile.absolutePath,
        frontendProjectDirectory.asFile.absolutePath,
    ).distinct()

val frontendSourceFiles =
    fileTree(frontendProjectDirectory) {
        include(
            "index.html",
            "package.json",
            "pnpm-lock.yaml",
            "public/**",
            "src/**",
            "tsconfig*.json",
            "vite.config.ts",
        )
    }

// Rust Android 构建只观察 core/shared/native 的受控源码；target/build 等生成目录不属于输入。
val rustNativeSourceFiles =
    fileTree(repositoryDirectory) {
        include(
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/**",
            "android/native/Cargo.toml",
            "android/native/src/**",
            "core/Cargo.toml",
            "core/src/**",
            "shared/Cargo.toml",
            "shared/src/**",
        )
        exclude("**/target/**", "android/**/build/**")
    }

val verifyNoLegacyFrontendAssets =
    tasks.register<VerifyNoLegacyFrontendAssetsTask>("verifyNoLegacyFrontendAssets") {
        group = "verification"
        description = "禁止 app/src/main/assets/frontend 继续作为旧前端来源"
        legacyDirectory.set(layout.projectDirectory.dir("src/main/assets/frontend"))
    }

val buildAndroidFrontend =
    tasks.register<FrontendBuildTask>("buildAndroidFrontend") {
        group = "build"
        description = "使用本机 pnpm 从当前 Vue/Vite 源码生成 Android 专用前端产物"
        sourceFiles.from(frontendSourceFiles)
        frontendDirectory.set(frontendProjectDirectory)
        pnpmExecutable.set(localPnpmExecutable)
        buildScript.set("build:android")
        outputEnvironmentVariable.set("WINESTOCK_FRONTEND_OUT_DIR")
        outputDirectory.set(layout.buildDirectory.dir("intermediates/winestockFrontend/android/dist"))
    }

androidComponents {
    onVariants(selector().all()) { variant ->
        val variantName = variant.name
        val variantTaskSuffix = variantName.replaceFirstChar { character -> character.uppercase() }
        val strictVerification = variant.buildType == "release"

        if (variant.buildType == "release") {
            // 发布 APK 使用产品名和版本号；AGP 元数据与实际文件名由同一 VariantOutput 属性生成。
            variant.outputs.forEach { output ->
                output.outputFileName.set(
                    output.versionName.map { versionName -> "WineStock-$versionName-release.apk" },
                )
            }
        }

        val verifyFrontendAssets =
            tasks.register<FrontendVerifyTask>("verify${variantTaskSuffix}FrontendAssets") {
                group = "verification"
                description = "验证 $variantName variant 使用的 Vite 前端产物"
                inputDirectory.set(buildAndroidFrontend.flatMap { it.outputDirectory })
                strict.set(strictVerification)
                forbiddenAbsolutePaths.set(forbiddenFrontendPaths)
                verificationMarker.set(
                    layout.buildDirectory.file(
                        "intermediates/winestockFrontend/$variantName/verification.properties",
                    ),
                )
            }

        val stageFrontendAssets =
            tasks.register<FrontendStageTask>("stage${variantTaskSuffix}FrontendAssets") {
                group = "build"
                description = "把已验证前端暂存为 $variantName variant generated assets"
                dependsOn(verifyNoLegacyFrontendAssets)
                inputDirectory.set(buildAndroidFrontend.flatMap { it.outputDirectory })
                verificationMarker.set(verifyFrontendAssets.flatMap { it.verificationMarker })
                outputDirectory.set(
                    layout.buildDirectory.dir("generated/winestockFrontendAssets/$variantName"),
                )
            }

        variant.sources.assets?.addGeneratedSourceDirectory(
            stageFrontendAssets,
            FrontendStageTask::outputDirectory,
        )

        val buildRustNativeLibraries =
            tasks.register<RustNativeBuildTask>("build${variantTaskSuffix}RustNativeLibraries") {
                group = "build"
                description = "离线构建 $variantName variant 的 arm64-v8a Rust JNI 库"
                sourceFiles.from(rustNativeSourceFiles)
                repositoryDirectory.set(rootProject.layout.projectDirectory.dir(".."))
                ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
                cargoExecutable.set("cargo")
                cargoNdkVersion.set("4.1.2")
                cargoPackage.set("winestock-android")
                targetAbi.set("arm64-v8a")
                minApi.set(26)
                release.set(variant.buildType == "release")
                cargoFeatures.set(
                    if (variant.buildType == "release") {
                        emptyList()
                    } else {
                        listOf("debug-swagger-ui")
                    },
                )
                sqliteCompileFlags.set(
                    if (variant.buildType == "release") {
                        "-USQLITE_ENABLE_FTS3 -USQLITE_ENABLE_FTS3_PARENTHESIS -USQLITE_ENABLE_FTS5"
                    } else {
                        ""
                    },
                )
                cargoTargetDirectory.set(
                    layout.buildDirectory.dir("intermediates/winestockRust/$variantName/cargo-target"),
                )
                outputDirectory.set(
                    layout.buildDirectory.dir("generated/winestockRustJniLibs/$variantName"),
                )
            }

        val verifyRustNativeLibraries =
            tasks.register<RustNativeVerifyTask>("verify${variantTaskSuffix}RustNativeLibraries") {
                group = "verification"
                description = "验证 $variantName variant 的 ARM64 ELF、JNI 导出和动态依赖"
                inputDirectory.set(buildRustNativeLibraries.flatMap { it.outputDirectory })
                ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
                targetAbi.set("arm64-v8a")
                libraryFileName.set("libwinestock_android.so")
                buildProfile.set(if (variant.buildType == "release") "release" else "debug")
                expectedJniSymbols.set(
                    listOf(
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeInitialize",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeDefaultRuntimeConfig",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeValidateRuntimeConfig",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeStartLocalService",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeStopLocalService",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeRestartLocalService",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeGetRuntimeState",
                        "Java_winestock_xiaowine_cc_core_NativeCoreBridge_nativeShutdownEngine",
                    ),
                )
                allowedNeededLibraries.set(listOf("liblog.so", "libdl.so", "libm.so", "libc.so"))
                verificationMarker.set(
                    layout.buildDirectory.file(
                        "intermediates/winestockRust/$variantName/verification.properties",
                    ),
                )
            }

        variant.sources.jniLibs?.addGeneratedSourceDirectory(
            buildRustNativeLibraries,
            RustNativeBuildTask::outputDirectory,
        )

        // native merge 必须先通过 ELF 验收，不能只依赖 .so 文件存在。
        tasks.matching { task -> task.name == "merge${variantTaskSuffix}NativeLibs" }
            .configureEach { dependsOn(verifyRustNativeLibraries) }

        val stagedManifest =
            stageFrontendAssets.flatMap { task -> task.outputDirectory }
                .map { directory -> directory.file("frontend/asset-manifest.json") }

        val verifyApkPackage =
            tasks.register<FrontendArchiveVerifyTask>("verify${variantTaskSuffix}FrontendPackage") {
                group = "verification"
                description = "验证 $variantName APK 中实际打包的前端资源"
                archiveFiles.from(
                    variant.artifacts.get(SingleArtifact.APK).map { apkDirectory ->
                        apkDirectory.asFileTree.matching { include("**/*.apk") }
                    },
                )
                expectedManifest.set(stagedManifest)
                assetsPrefix.set("assets")
                allowArchiveWithoutFrontend.set(true)
                strict.set(strictVerification)
                forbiddenAbsolutePaths.set(forbiddenFrontendPaths)
                reportFile.set(
                    layout.buildDirectory.file(
                        "reports/winestockFrontend/$variantName/apk-verification.properties",
                    ),
                )
            }

        val verifyRustNativeApkPackage =
            tasks.register<RustNativeApkVerifyTask>("verify${variantTaskSuffix}RustNativeApkPackage") {
                group = "verification"
                description = "验证 $variantName APK 只包含 arm64-v8a WineStock JNI 库"
                dependsOn(verifyRustNativeLibraries)
                apkFiles.from(
                    variant.artifacts.get(SingleArtifact.APK).map { apkDirectory ->
                        apkDirectory.asFileTree.matching { include("**/*.apk") }
                    },
                )
                targetAbi.set("arm64-v8a")
                libraryFileName.set("libwinestock_android.so")
                buildProfile.set(if (variant.buildType == "release") "release" else "debug")
                reportFile.set(
                    layout.buildDirectory.file(
                        "reports/winestockRust/$variantName/apk-verification.properties",
                    ),
                )
            }

        tasks.matching { task -> task.name == "assemble$variantTaskSuffix" }
            .configureEach {
                dependsOn(verifyApkPackage)
                dependsOn(verifyRustNativeApkPackage)
            }
        tasks.matching { task -> task.name == "install$variantTaskSuffix" }
            .configureEach {
                dependsOn(verifyApkPackage)
                dependsOn(verifyRustNativeApkPackage)
            }
    }
}

dependencies {
    implementation(libs.androidx.activity.ktx)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.webkit)
    implementation(libs.androidx.core.splashscreen)
    testImplementation(libs.junit)
    testImplementation(libs.org.json)
}
