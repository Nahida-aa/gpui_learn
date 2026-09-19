//! # theme-settings —— 主题的 JSON 装载与装配
//!
//! 本包负责「把磁盘上的主题扩展 JSON 变成可用的主题」这一段，
//! 对应 zed 的 `crates/theme_settings`：
//!
//! | 本包 | zed |
//! |---|---|
//! | [`content`] | `crates/theme_settings/src/schema.rs` |
//! | [`loaders`] | `theme_settings.rs` 的 `refine_theme_family` / `refine_theme` |
//! | [`init_theme`] / [`load_asset_themes`] | `theme_settings.rs` 的 `init` / `load_bundled_themes` |
//!
//! ## 为什么和 `aa-gpui-kit-theme` 分开
//!
//! 依赖方向是**单向**的：`theme-settings → theme`。
//!
//! `theme` 包只描述**运行时**结构（`Theme` / `ThemeColors` / 注册表），
//! 完全不知道 JSON 长什么样；本包才知道 JSON 的 schema 与装载流程。
//! 这样拆分的好处是：只想用主题（读颜色、切主题）的场景不必编译 serde，
//! 而需要接主题扩展 / 用户主题时，扩展点就在本包。
//!
//! ## 用法
//!
//! ```ignore
//! // 应用启动时（一次）：装入内置主题 + 资产里的主题 JSON，并设为当前
//! theme_settings::init_theme(cx);
//!
//! // 运行时切换
//! theme_settings::set_theme_by_name(cx, "Catppuccin Mocha");
//! ```

pub mod content;
pub mod init;
pub mod loaders;

pub use content::{
    AppearanceContent, StyleContent, SyntaxContent, ThemeContent, ThemeFamilyContent,
};
pub use init::{GlobalAssets, init_theme, list_theme_names, load_asset_themes, set_theme_by_name};
pub use loaders::parse_theme_family;
