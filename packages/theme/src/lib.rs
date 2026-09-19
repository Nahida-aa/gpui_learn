//! # aa-gpui-kit-theme —— GPUI 主题系统（对齐 zed `crates/theme` 的精简版）
//!
//! gpui 本身**不带**主题系统：它只有 8 个兜底色（`gpui::Colors`）与
//! `WindowAppearance` 明暗信号。真正的主题（语义色分组、语法色、
//! 主题家族、注册表）是 zed 在 `crates/theme` 里实现的——本 crate
//! 按同一套结构做精简移植。
//!
//! ## 用法
//!
//! ```ignore
//! // 启动时（一次）：装入内置主题 + 资产里的主题 JSON，并设为当前
//! aa_gpui_kit_theme::init_theme(cx);
//!
//! // 运行时切换
//! aa_gpui_kit_theme::set_theme_by_name(cx, "Catppuccin Mocha");
//!
//! // 任意组件取色（对齐 zed 的 cx.theme()）
//! use aa_gpui_kit_theme::ActiveTheme as _;
//! div().bg(cx.theme().colors().editor_background)
//! ```
//!
//! ## 与 zed 的对应关系
//!
//! | 本 crate | zed |
//! |---|---|
//! | [`Theme`]`{id,name,appearance,styles}` | `crates/theme/theme.rs` 同名 |
//! | [`ThemeStyles`] | `theme.rs` 同名 |
//! | [`styles`]（[`ThemeColors`] 等样式集合） | `crates/theme/styles.rs` + `styles/` |
//! | [`SyntaxTheme`] | `crates/syntax_theme` 同名 |
//! | [`ActiveTheme`] `for App` | `theme.rs:146` 同名 |
//! | [`default_colors`]（色板 + 内置 Catppuccin 配色） | `default_colors.rs` |
//! | [`registry::ThemeRegistry`] | `registry.rs` |
//!
//! ## 物理位置
//!
//! 本 crate 原为 `ui-gpui/src/base/theme`（模块路径 `ui_gpui::base::theme`），
//! 2026-09 拆成 workspace 独立包。**不提供** `ui_gpui::base::theme` 别名——
//! 调用方一律用 `aa_gpui_kit_theme::`，让「主题不隶属控件库」这件事在代码里可见。

pub mod buffer_line_height;
pub mod color_space;
pub mod default_colors;
pub mod fallback_themes;
pub mod font_family_cache;
pub mod icon_theme;
pub mod registry;
pub mod scale;
pub mod schema;
pub mod settings_provider;
pub mod styles;
pub mod ui_density;

mod state;

pub use schema::{AppearanceContent, try_parse_color};
pub use buffer_line_height::BufferLineHeight;
pub use color_space::{Oklab, Oklch, hsla_to_oklab, hsla_to_oklch, oklch_to_hsla};
pub use font_family_cache::FontFamilyCache;
pub use icon_theme::{
    ChevronIcons, DirectoryIcons, IconDefinition, IconTheme, IconThemeFamily, DEFAULT_ICON_THEME_NAME,
    default_icon_theme, parse_icon_theme_family,
};
pub use icon_theme::schema::{
    ChevronIconsContent, DirectoryIconsContent, IconDefinitionContent, IconThemeContent,
    IconThemeFamilyContent,
};
pub use registry::{ThemeMeta, ThemeNotFoundError, ThemeRegistry};
pub use scale::{ColorScale, ColorScaleSet, ColorScaleStep, ColorScales};
pub use default_colors::default_color_scales;
pub use fallback_themes::DEFAULT_DARK_THEME;
pub use settings_provider::{
    DefaultThemeSettingsProvider, ThemeSettingsProvider, buffer_font, buffer_font_size,
    scaled_spacing, set_theme_settings_provider, theme_settings, ui_density, ui_font, ui_font_size,
};
pub use ui_density::UiDensity;

// 「当前主题是什么」这份状态归本包管；**怎么把主题装进来**归 theme-settings 包管
// （JSON 的 schema 与装载流程在那边，依赖方向 theme-settings → theme）。
pub use state::{
    ActiveTheme, Appearance, GlobalTheme, LoadThemes, SystemAppearance, Theme, ThemeFamily,
    ThemeStyles, set_theme,
};
pub use styles::{
    AccentColors, DiagnosticColors, PlayerColor, PlayerColors, StatusColors,
    StatusColorsRefinement, SyntaxTheme, SystemColors, ThemeColors, ThemeColorsRefinement,
};
