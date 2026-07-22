import com.android.build.api.artifact.SingleArtifact
import winestock.build.FrontendArchiveVerifyTask
import winestock.build.FrontendBuildTask
import winestock.build.FrontendStageTask
import winestock.build.FrontendVerifyTask
import winestock.build.VerifyNoLegacyFrontendAssetsTask

plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "winestock.xiaowine.cc"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "winestock.xiaowine.cc"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            optimization {
                enable = false
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
        includeInBundle = false
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

        val verifyBundlePackage =
            tasks.register<FrontendArchiveVerifyTask>("verify${variantTaskSuffix}FrontendBundlePackage") {
                group = "verification"
                description = "验证 $variantName AAB 中实际打包的前端资源"
                archiveFiles.from(variant.artifacts.get(SingleArtifact.BUNDLE))
                expectedManifest.set(stagedManifest)
                assetsPrefix.set("base/assets")
                allowArchiveWithoutFrontend.set(false)
                strict.set(strictVerification)
                forbiddenAbsolutePaths.set(forbiddenFrontendPaths)
                reportFile.set(
                    layout.buildDirectory.file(
                        "reports/winestockFrontend/$variantName/bundle-verification.properties",
                    ),
                )
            }

        tasks.matching { task -> task.name == "assemble$variantTaskSuffix" }
            .configureEach { dependsOn(verifyApkPackage) }
        tasks.matching { task -> task.name == "bundle$variantTaskSuffix" }
            .configureEach { dependsOn(verifyBundlePackage) }
    }
}

dependencies {
    implementation(libs.androidx.activity.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.constraintlayout)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.webkit)
    implementation(libs.androidx.core.splashscreen)
    implementation(libs.material)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
}
