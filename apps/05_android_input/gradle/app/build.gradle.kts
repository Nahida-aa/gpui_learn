// 本例子的 Gradle 构建逻辑全部来自仓库共享模板（配置驱动，Tauri 风格）。
// 这里不写任何 Gradle 脚本——所有可变项都在同目录 gradle.properties 里。
apply(from = rootProject.file("../../../gradle/android-app.gradle"))
