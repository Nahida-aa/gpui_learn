plugins {
    id("com.android.application") version "8.7.3" apply false
    // Kotlin 版本与 Tauri 工程实测一致；与 AGP 8.7.3 / Gradle 8.9 兼容。
    id("org.jetbrains.kotlin.android") version "1.9.25" apply false
}
