# gpui-cli —— GpuiActivity Java → Kotlin 迁移 TODO

> 目标:把共享原生宿主 `packages/gpui-android/android/src/main/java/dev/gpui/mobile/GpuiActivity.java`
> 迁移为 Kotlin(`GpuiActivity.kt`),并让所有 app 的 `gen/android` 能直接编译 Kotlin。
> 迁移后 Java 版仍可从 git 历史查看(`git log` / `git show`),无需保留备份文件。

## 背景 / 为什么现在做

- 我们想仿照 Tauri 的模型:原生壳是可入库、可 review、偶发可手改的;而语言用 Kotlin(现代 Android 默认、
  Tauri 的 `MainActivity.kt` 即是 Kotlin)更贴近生态。
- 现状:`GpuiActivity.java` 是 vendor 自 gpui-toolkit 的 Java;本仓库刻意「零 kt」。这次迁移是
  「把共享宿主转成 Kotlin」,不含每-app 原生代码。

## 技术前提(已确认的坑)

- 生成的 `app/build.gradle.kts`(模板 `templates/android-app.gradle.kts`)目前 **只 apply
  `com.android.application`,没有 kotlin 插件** —— 不先加插件,`.kt` 根本无法编译。
- `java.srcDir("{ANDROID_JAVA_DIR}")` 直接引用共享目录;kotlin-android 插件默认把 kotlin 源码
  源集并到 java 源集目录,所以 `.kt` 放同目录即可被编译,`srcDir` 无需改路径。

## TODO

- [x] **1. 保存 Java 版**
  - [x] 确认 `GpuiActivity.java` 已入库(git ls-files 命中、工作区干净)——git 历史即备份
  - [ ] (可选)先 `git commit` 当前状态,留一个清晰的迁移分界点

- [x] **2. 让生成工程能编译 Kotlin**(改 `packages/gpui-cli/templates/android-app.gradle.kts`)
  - [x] 在 `plugins {}` 加 `id("org.jetbrains.kotlin.android")`(版本需与 AGP/SDK 兼容,对照 Tauri 项目的配置)
  - [x] 确认 `gradle.properties` 或 settings 里 Kotlin 版本来源(在模板里定死一个可用版本)
  - [x] 更新源码引用注释:`java.srcDir` 的语义说明从「GpuiActivity.java」改为「GpuiActivity.kt」

- [x] **3. 把 GpuiActivity 翻译成 Kotlin**(共享层,`packages/gpui-android`)
  - [x] 新建 `GpuiActivity.kt`,1:1 保留全部逻辑
  - [x] 注意 Java 隐式差异(Kotlin 编译期已修:`editable.length` 属性、`TAG` 常量、内部类
        `companion object` 折叠进外层、`ensureNativeLibLoaded` 里 `(ctx as GpuiActivity)` 强转)
  - [x] 删除 `GpuiActivity.java`(git 历史即备份)
  - [x] 同步更新 `packages/gpui-android` 内对 Activity 的文档/注释引用

- [x] **4. 重新生成并验证打包**
  - [x] 对 `apps/_09_a11y`、`apps/08_testing` 重跑 `just android_init` + `gradlew assembleDebug`
  - [x] 确认 AAPT/AGP 阶段无错、`.so` 正常打入
  - [ ] (可选)真机/Waydroid 装包验证软键盘与无障碍桥仍工作

- [x] **5. 文档一致性**
  - [x] 更新 `apps/05_android_input/docs/IME_INPUT_DEBUG.md`
  - [x] 更新 `apps/06_text_area/docs/ANDROID_INPUT_DESIGN.md`、`SOURCE_VALUE_BUG.md`、`selection_toolbar.md` 中的 `GpuiActivity.kt` 引用
  - [x] 检查 `docs/` 与各 app README 里对 Java 文件的称呼

- [ ] **6.(后续,独立议题)Tauri 式 per-app 可编辑入口**
  - [ ] 设计:给每个 app 生成一个入库、`init` 不覆盖的 Kotlin 入口(默认继承共享 `GpuiActivity`)
  - [ ] `init` 只在文件不存在时创建(区别于当前 `fs::write` 无条件覆盖)
  - [ ] 评估是否让 `GpuiActivity` 可继承/可覆写(open class),供个别 app 下放原生能力

## 验收标准

- [ ] `cargo check -p _09_a11y` 通过(桌面)
- [ ] 至少两个 app 的 `./gradlew assembleDebug` 成功且含 `.kt` 编译产物
- [ ] `git log --oneline` 里能找到迁移分界点,Java 版随时可 `git show` 取回
