//! # ⚠️ 已废弃：本包当前没有消费者，不要作为参考
//!
//! 2026-10-14 确认：原先唯一的消费方已换成 AAgent 自己的
//! `packages/theme_settings`
//! （AAgent `Cargo.toml`：`theme_settings = { path = "packages/theme_settings" }`），
//! 本包在 gpui_learn 与 AAgent 两侧都**没有依赖方**，处于「留着未删」状态。
//!
//! **排查主题相关问题时不要读本包**：下面的说明与 `init_theme.rs` 的内容都已
//! 过时——例如它写着「内置主题只有 Mocha / Latte、默认回退 Mocha」，而实际
//! 入口在 AAgent 侧：
//! - `packages/theme_settings/src/bundled_themes.rs`：注册内置主题
//! - `packages/theme_settings/src/lib.rs`：`init()` 与默认主题兜底（现在是
//!   `catppuccin_macchiato()`）
//!
//! 保留条件：将来若要把「主题 JSON 装载」这段搬回 gpui_learn，本包仍是参考
//! 实现；在那之前不要改它，也不要照它改别处。
//!
//! ---
//!
//! 以下为历史说明（对应 zed `crates/theme_settings` 的设计）：
//!
//! # theme-settings —— 主题的 JSON 装载与装配
//!
//! 本包负责「把磁盘上的主题扩展 JSON 变成可用的主题」这一段，
//! 对应 zed 的 `crates/theme_settings`：
//!
//! | 本包 | zed |
//! |---|---|
//! | [`content`] | `crates/theme_settings/src/schema.rs` |
//! | [`loaders`] | `theme_settings.rs` 的 `refine_theme_family` / `refine_theme` |
//! | [`init`] / [`load_asset_themes`] | `theme_settings.rs` 的 `init` / `load_bundled_themes` |
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
//! theme_settings::init(LoadThemes::All(assets), cx);
//!
//! // 运行时切换
//! theme_settings::set_theme_by_name(cx, "Catppuccin Mocha");
//! ```

pub mod content;
pub mod init_theme;
pub mod loaders;

pub use content::{
    AppearanceContent, StyleContent, SyntaxContent, ThemeContent, ThemeFamilyContent,
};
pub use init_theme::{GlobalAssets, init, list_theme_names, load_asset_themes, set_theme_by_name};
pub use loaders::parse_theme_family;
