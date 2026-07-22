// Android 构建逻辑只提供平台打包任务类型，不承载应用运行时代码。
plugins {
    `kotlin-dsl`
}

repositories {
    gradlePluginPortal()
    mavenCentral()
}
