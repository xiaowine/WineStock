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

// 把共享前端的构建产物同步进 Android assets；前端源码不在本模块维护，
// 需先在 frontend/ 执行 `pnpm build` 生成 dist。生成目录不纳入版本库。
// 在配置期把 dist 是否存在算成布尔值并赋给 enabled，避免 onlyIf 的 lambda 捕获脚本对象，
// 以兼容 configuration cache。dist 不存在时任务禁用而非删除已打包资源。
val frontendDistDir: File = rootProject.projectDir.resolve("../frontend/dist")
val frontendDistExists: Boolean = frontendDistDir.isDirectory
val syncFrontendAssets =
    tasks.register<Sync>("syncFrontendAssets") {
        description = "把 frontend/dist 同步到 assets/frontend"
        enabled = frontendDistExists
        from(frontendDistDir)
        into(layout.projectDirectory.dir("src/main/assets/frontend"))
    }

tasks.named("preBuild") {
    dependsOn(syncFrontendAssets)
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