//! 主题系统,对齐 zed `crates/theme` 的精简版。
//!
//! gpui 本身**不带**主题系统:它只有 8 个兜底色(`gpui::Colors`)与
//! `WindowAppearance` 明暗信号。真正的主题(语义色分组、语法色、
//! 主题家族、注册表)是 zed 在 `crates/theme` 里实现的——本模块
//! 按同一套结构做精简移植。
//!
//! ## 用法
//!
//! ```ignore
//! // 启动时(一次):装入内置主题并设为当前
//! ui_gpui::base::input::theme::init_theme(cx);
//!
//! // 运行时切换
//! ui_gpui::base::input::theme::set_theme_by_name(cx, "ui-gpui Light");
//!
//! // 任意组件取色(对齐 zed 的 cx.theme())
//! div().bg(cx.theme().colors().editor_background)
//! ```
//!
//! ## 与 zed 的对应关系
//!
//! | 本模块 | zed |
//! |---|---|
//! | `Theme{id,name,appearance,styles}` | `crates/theme/theme.rs` 同名 |
//! | `ThemeColors`(约 55 个语义色) | 同名(150 字段子集) |
//! | `SyntaxTheme` | `crates/syntax_theme` 同名 |
//! | `ActiveTheme for App` | `theme.rs:146` 同名 |
//! | `ThemeRegistry` | `registry.rs` 精简版(无 JSON 加载) |
//!
//! `set_theme` 同时把语义色**同步进 gpui 的 `GlobalColors`**,让 gpui
//! 自带的基础设施(如默认滚动条)也跟随主题。

pub mod builtin;
pub mod colors;
pub mod syntax;

use std::sync::Arc;

use gpui::{App, Global, WindowAppearance};

pub use colors::{AccentColors, StatusColor, StatusColors, SystemColors, ThemeColors};
pub use syntax::SyntaxTheme;

/// 主题适配的明暗形态(对齐 zed `theme.rs` 的 `Appearance`)。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    Light,
    #[default]
    Dark,
}

impl Appearance {
    /// 映射到 gpui 的窗口外观(供 `set_window_appearance` 用)。
    pub fn to_window_appearance(self) -> WindowAppearance {
        match self {
            Appearance::Light => WindowAppearance::Light,
            Appearance::Dark => WindowAppearance::Dark,
        }
    }
}

/// 一套主题的全部样式(对齐 zed `ThemeStyles` 的精简子集)。
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeStyles {
    pub colors: ThemeColors,
    pub status: StatusColors,
    pub accents: AccentColors,
    pub system: SystemColors,
    pub syntax: Arc<SyntaxTheme>,
}

/// 一个主题:id / 名称 / 明暗 / 样式(对齐 zed 同名结构)。
#[derive(Clone, Debug)]
pub struct Theme {
    /// 唯一标识(如 `"ui-gpui-default-dark"`)。
    pub id: String,
    /// 展示名(如 `"ui-gpui Dark"`)。
    pub name: String,
    /// 明暗形态。
    pub appearance: Appearance,
    /// 样式集合。
    pub styles: ThemeStyles,
}

impl Theme {
    pub fn colors(&self) -> &ThemeColors {
        &self.styles.colors
    }

    pub fn status(&self) -> &StatusColors {
        &self.styles.status
    }

    pub fn accents(&self) -> &AccentColors {
        &self.styles.accents
    }

    pub fn syntax(&self) -> &Arc<SyntaxTheme> {
        &self.styles.syntax
    }
}

/// 主题家族:同一套配色的明暗成对(对齐 zed `ThemeFamily`)。
#[derive(Clone, Debug)]
pub struct ThemeFamily {
    pub name: String,
    /// 家族内的主题(通常 light / dark 各一)。
    pub themes: Vec<Theme>,
}

/// 全局当前主题(gpui `Global`)。
#[derive(Clone)]
pub struct GlobalTheme(pub Arc<Theme>);

impl Global for GlobalTheme {}

/// 内置注册表(gpui `Global`):内置主题 + 运行时注册的主题。
#[derive(Clone)]
pub struct GlobalThemeRegistry(pub builtin::ThemeRegistry);

impl Global for GlobalThemeRegistry {}

/// 读取当前主题。未初始化时惰性返回内置深色主题——
/// 这让单元测试与未调用 `init_theme` 的最小程序也能直接用。
/// 对齐 zed `ActiveTheme`:任意能拿到 `App` 的地方 `cx.theme()` 取当前主题。
///
/// 未调用 `init_theme` 时惰性返回内置深色主题(常量泄漏一次),
/// 这让单元测试与最小程序也能直接用。
pub trait ActiveTheme {
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Arc<Theme> {
        static DEFAULT: std::sync::OnceLock<Arc<Theme>> = std::sync::OnceLock::new();
        match self.try_global::<GlobalTheme>() {
            Some(global) => &global.0,
            None => DEFAULT.get_or_init(|| {
                Arc::new(
                    builtin::ThemeRegistry::with_builtins()
                        .get("ui-gpui-default-dark")
                        .expect("builtin theme must exist")
                        .clone(),
                )
            }),
        }
    }
}

/// 安装主题系统(应用启动时调用一次):
/// 注册内置主题并把 `theme` 设为当前主题。
pub fn init_theme(cx: &mut App) {
    cx.set_global(GlobalThemeRegistry(builtin::ThemeRegistry::with_builtins()));
    let theme: Arc<Theme> = cx.theme().clone();
    set_theme(cx, theme);
}

/// 按 id/名称切换主题(需已 `init_theme`)。找不到时保持不变并返回 false。
pub fn set_theme_by_name(cx: &mut App, id_or_name: &str) -> bool {
    let Some(theme) = cx
        .try_global::<GlobalThemeRegistry>()
        .and_then(|registry| registry.0.get(id_or_name).cloned())
    else {
        return false;
    };
    set_theme(cx, Arc::new(theme));
    true
}

/// 设置当前主题:
/// 1. 写入 `GlobalTheme`;
/// 2. 把语义色同步进 gpui 的 `GlobalColors`(gpui 自带基础设施跟随主题);
/// 3. 各窗口收到 appearance 后自行重绘(组件已在 render 里读 `cx.theme()`,
///    gpui 会因全局变化自动触发重渲染)。
pub fn set_theme(cx: &mut App, theme: Arc<Theme>) {
    sync_global_colors(cx, &theme);
    cx.set_global(GlobalTheme(theme));
}

/// 把主题语义色映射进 gpui 的 8 色兜底(`gpui::Colors`)。
fn sync_global_colors(cx: &mut App, theme: &Theme) {
    use gpui::colors::GlobalColors;
    let c = &theme.styles.colors;
    cx.set_global(GlobalColors(Arc::new(gpui::colors::Colors {
        text: c.text.into(),
        selected_text: c.text.into(),
        background: c.background.into(),
        disabled: c.text_disabled.into(),
        selected: c.element_selected.into(),
        border: c.border.into(),
        separator: c.border_variant.into(),
        container: c.surface_background.into(),
    })));
}
