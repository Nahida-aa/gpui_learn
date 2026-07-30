// 桌面二进制入口：仅非 Android 编译/运行。`run()` 在 lib.rs 里定义。
// Android 上加载的是 lib 产出的 cdylib `.so`（经 NativeActivity），
// 本 bin 不用于 Android，这里再用 cfg 兜底，确保即使被编译也是空 main。
fn main() {
    #[cfg(not(target_os = "android"))]
    uniform_list_07::run();
}
