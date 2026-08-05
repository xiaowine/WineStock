// Android 构建逻辑只提供平台打包任务类型，不承载应用运行时代码。
plugins {
    `kotlin-dsl`
}

repositories {
    // buildSrc 的 Kotlin DSL 与插件依赖也优先使用国内镜像。
    maven {
        name = "AliyunGradlePlugin"
        url = uri("https://maven.aliyun.com/repository/gradle-plugin")
    }
    maven {
        name = "AliyunPublic"
        url = uri("https://maven.aliyun.com/repository/public")
    }
    maven {
        name = "HuaweiMaven"
        url = uri("https://repo.huaweicloud.com/repository/maven/")
    }
    gradlePluginPortal()
    mavenCentral()
}
