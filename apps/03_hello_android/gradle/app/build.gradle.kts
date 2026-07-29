plugins {
    id("com.android.application")
}

// ── Rust 交叉编译任务 ───────────────────────────────────────────────────────
//
// 在 Gradle 打包 apk 之前，用 `cargo ndk` 把我们的 cdylib 编成 Android 的
// `libhello_android.so`，并复制到 Gradle 的 jniLibs 目录。
// 前置条件（见 README）：已装 cargo-ndk、ANDROID_HOME/NDK_HOME、Java。
val rustTarget = "arm64-v8a"
val rustLibName = "hello_android" // 与 Cargo.toml 的 [lib] name 一致
// cargo ndk 的 -o 目录会**自动追加 ABI 子目录**，所以这里给 jniLibs 根即可，
// 最终产物落在 src/main/jniLibs/arm64-v8a/libhello_android.so（Gradle 期望的位置）。
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs")
val cargoProfile = if (providers.gradleProperty("release").isPresent) "release" else "debug"

val cargoBuild by tasks.registering(Exec::class) {
    workingDir = projectDir.parentFile.parentFile.parentFile // 仓库根（含 Cargo.toml workspace）
    // 先拼成 List<String>，再传给 commandLine，避免 vararg/DSL 解析歧义。
    val args = listOf(
        "cargo", "ndk",
        "-t", rustTarget,
        "-P", "26", // 必须 ≥ 24，否则 NDK 链接找不到 libnativewindow.so
        "-o", jniLibsDir.asFile.absolutePath,
        "build",
        "-p", "hello_android_03",
    )
    if (cargoProfile == "release") {
        commandLine(args + "--release")
    } else {
        commandLine(args)
    }
    // 让 Gradle 知道产物位置，便于增量判断。
    outputs.dir(jniLibsDir)
}

// 让 :app:preBuild 依赖于 Rust 编译，保证打 apk 前 .so 已就位。
tasks.named("preBuild") { dependsOn(cargoBuild) }

android {
    namespace = "dev.gpui.learn.hello_android"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.gpui.learn.hello_android"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf(rustTarget)
        }

        // 这个值会填进 AndroidManifest 的 android.app.lib_name（NativeActivity 用）。
        manifestPlaceholders["nativeLibraryName"] = rustLibName
    }

    buildTypes {
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
        release {
            isMinifyEnabled = false
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
            // 复用 vendored gpui-android 提供的 GpuiActivity.java（NativeActivity 子类 +
            // 软键盘 InputConnection + 无障碍桥接）。不要复制，直接引用源码目录。
            java.srcDir("../../../../crates/gpui-android/android/src/main/java")
        }
    }

    packaging {
        jniLibs {
            keepDebugSymbols += listOf("*/$rustTarget/lib${rustLibName}.so")
        }
    }
}
