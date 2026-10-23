//! # aa-bundle —— 打包基础设施（库侧）
//!
//! 打包工具 `aa-bundle` 的**可复用**部分，二进制（`src/main.rs`）只是 CLI 壳。
//! 其他工具（如 `aa-gpui-cli`）依赖本库，不重复实现。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! | --- | --- |
//! | [`meta`] | bundle 元数据结构与解析（对齐 cargo-bundle 的 `BundleSettings`） |
//! | [`desktop`] | `.desktop` 条目渲染（类别映射 / 模板 / 安装名） |
//! | [`identity`] | 命名规则（段规范化 / identifier 推导 / 仓库名发现） |
//! | [`cargo`] | 构建环境查询（target 目录 / profile 目录 / 产物路径） |
//! | [`fs`] | 文件 IO 基建（写 / 拷 / 相对路径 / 可执行位） |
//! | [`formats`] | **打包格式**：一个模块一种输出（`pkgbuild` / `targz`） |
//!
//! ## 设计约束
//!
//! - **格式特有设置**（Arch 的 depends / 安装前缀等）一律放 bundle 元数据的
//!   子表（如 `[bundle.arch]`），**不做成 CLI flag**——flag 只留所有格式
//!   通用的（`-p` / `--bundle-meta` / `--bin-name` / `--icon-name` /
//!   `--desktop-template` / `--out`），否则加新格式时用户会看到一堆对当前
//!   格式无效的参数。
//! - 错误消息里提到 `--bundle-meta`，因为那是消费方 CLI 的约定参数名
//!   （本库不定义 CLI 参数，只定义引用语法与解析规则）。
//!
//! ## 公开路径
//!
//! 模块化之前所有公开项都在 crate 根；为不打破既有消费方（如 `aa-gpui-cli`
//! 按 `aa_bundle::write_file` 引用），根级保留一份 `pub use` 转发。
//! 新代码建议用模块路径（`aa_bundle::fs::write_file`）。

pub mod cargo;
pub mod desktop;
pub mod formats;
pub mod fs;
pub mod identity;
pub mod meta;

// 根级转发：保持既有公开路径可用（详见模块文档「公开路径」一节）。
pub use cargo::{profile_dir, profile_flag, resolve_binary_path, resolve_target_dir};
pub use desktop::{
    AppCategory, icon_size_dir, render_desktop_entry, render_template, resolve_desktop_entry,
    resolve_desktop_id, resolve_icon_name,
};
pub use formats::{pkgbuild, targz};
pub use fs::{copy_dir, relative_path, set_executable, write_bytes, write_file};
pub use identity::{derive_identifier, pkg_segment, repo_name_from};
pub use meta::{
    ArchTable, BundleMeta, DesktopAction, LinuxTable, load_bundle_meta, load_bundle_meta_layered,
    load_table, value_type_name,
};
