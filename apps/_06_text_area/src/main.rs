// 桌面二进制入口：仅非 Android 编译/运行。`run()` 在 lib.rs 里定义（平台无关装配）。
// Android 上本 bin 不会被实际构建（见 Cargo.toml 的 [[bin]] target 限制），
// 这里再加一层 cfg 兜底，确保即使被编译也是空 main，不会引用桌面专属的 run()。
fn main() {
    #[cfg(not(target_os = "android"))]
    text_area_06::run();
}
