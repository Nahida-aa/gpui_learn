//! 桌面二进制薄壳：真正逻辑在 `lib.rs` 的 `run()`（条件编译入口）。
//! Android 不编译本 bin，而是产 cdylib `.so` 给 NativeActivity 加载。
#![cfg(not(target_family = "wasm"))]

fn main() {
    #[cfg(not(target_os = "android"))]
    _09_a11y::run();
}