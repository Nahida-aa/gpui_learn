plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// 本文件由 `gpui-cli android init` 生成（模板见 crates/gpui-cli）。
// 所有可变项来自：例子的 Cargo.toml（cargo 包名 / lib 名）与 gpui.conf.json
// （identifier / app_name）。不要手写——改配置后重跑 `gpui-cli android init`。

// ── 从生成时注入的常量 ──────────────────────────────────────────────────────
val rustLibName = "_09_a11y"          // cargo cdylib 的 [lib] name → lib<RUST_LIB_NAME>.so
val cargoPackage = "_09_a11y"         // workspace 里的 cargo 包名，用于 cargo ndk -p
val abiList = listOf("arm64-v8a", "x86_64")              // 目标 ABI 列表（默认 arm64-v8a + x86_64）

// cargo ndk 的 -o 目录会**自动追加 ABI 子目录**，所以给 jniLibs 根即可。
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs")
val cargoProfile = if (providers.gradleProperty("release").isPresent) "release" else "debug"

// ── Rust 交叉编译任务：cargo ndk 为每个 ABI 编出 .so ────────────────────────
val cargoBuild by tasks.registering(Exec::class) {
    workingDir = projectDir.parentFile.parentFile.parentFile.parentFile // 仓库根（含 Cargo.toml workspace）
    val args = mutableListOf(
        "cargo", "ndk",
        "-P", "26", // 必须 >= 24，否则 NDK 链接找不到 libnativewindow.so
        "-o", jniLibsDir.asFile.absolutePath,
        "build",
        "-p", cargoPackage,
    )
    abiList.forEach { args += listOf("-t", it) }
    if (cargoProfile == "release") args += "--release"
    commandLine(args)
    outputs.dir(jniLibsDir)
}

tasks.named("preBuild") { dependsOn(cargoBuild) }

android {
    namespace = providers.gradleProperty("appId").get()
    compileSdk = 35

    // Java 与 Kotlin 的 JVM 目标必须一致，否则 AGP 报 Inconsistent JVM-target。
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }

    defaultConfig {
        applicationId = providers.gradleProperty("appId").get()
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += abiList
        }

        // 填进 AndroidManifest 的 android.app.lib_name（NativeActivity 用它找 .so）。
        manifestPlaceholders["nativeLibraryName"] = rustLibName
        // 把 app_name 注入资源，AndroidManifest 用 @string/app_name 引用即可。
        resValue("string", "app_name", providers.gradleProperty("appName").get())
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
            // 复用 vendored gpui-android 提供的 GpuiActivity（Kotlin，NativeActivity 子类 +
            // 软键盘 InputConnection + 无障碍桥接）。不复制，直接引用源码目录。
            // kotlin-android 插件会把 java/kotlin 源集合并，.kt 放该目录即可被编译。
            // 路径由 `gpui-cli` 在生成时按例子实际位置算出（相对 gen/android/app/）。
            java.srcDir("../../../../../packages/gpui-android/android/src/main/java")
        }
    }

    packaging {
        jniLibs {
            abiList.forEach { keepDebugSymbols += listOf("*/$it/lib${rustLibName}.so") }
        }
    }
}
