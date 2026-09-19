//! styles:主题的样式集合,按 zed `crates/theme/src/styles/` 的文件划分组织。
//!
//! zed 这里用 `styles.rs` + `styles/` 目录(而非 `styles/mod.rs`),子模块
//! 声明为**私有**、再 `pub use xxx::*` 展平——调用方看到的是扁平命名空间
//! (`styles::ThemeColors`、`styles::StatusColors`…),文件怎么切分是实现细节。
//!
//! 与 zed 的对应:
//!
//! | 本模块 | zed `styles/` |
//! |---|---|
//! | [`colors`] | `colors.rs`(`ThemeColors`) |
//! | [`status`] | `status.rs`(`StatusColors`) |
//! | [`system`] | `system.rs`(`SystemColors`) |
//! | [`accents`] | `accents.rs`(`AccentColors`) |
//! | [`syntax`] | `syntax.rs`(`SyntaxTheme`,在 zed 是 `syntax_theme` 的重导出) |
//!
//! 与本 crate 其他模块的关系:本模块只放**颜色结构体本身**;
//! 装配(注册表、全局状态、切换)在 [`state`](crate::state),
//! 内置配色值在 [`builtin`](crate::builtin),JSON 解析在
//! [`content`](crate::content) / [`loaders`](crate::loaders)。

mod accents;
mod colors;
mod status;
mod syntax;
mod system;

pub use accents::AccentColors;
pub use colors::ThemeColors;
pub use status::{StatusColor, StatusColors};
pub use syntax::SyntaxTheme;
pub use system::SystemColors;
