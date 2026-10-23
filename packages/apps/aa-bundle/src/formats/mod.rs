//! 打包格式：一个模块一种输出格式，对应 CLI `--format` 的取值。
//!
//! - [`pkgbuild`] —— Arch Linux 的 PKGBUILD（makepkg 的输入）。支持源码打包
//!   （`build()` 里 `cargo build`）与二进制打包（`--prebuilt`，安装生成期
//!   钉死的已编好产物）两种模式。
//! - [`targz`] —— 自包含 tar.gz 应用包（zed.app 布局，跨发行版）。本就是
//!   预编译打包：不构建，只安装已存在的二进制。
//!
//! 加新格式 = 在这里加一个模块 + CLI 的 `Format` 枚举加一个变体。
//! 格式之间共享的一切（身份解析、.desktop 渲染、文件 IO）都在 crate 根的
//! 对应模块里，不往这里塞。

pub mod pkgbuild;
pub mod targz;
