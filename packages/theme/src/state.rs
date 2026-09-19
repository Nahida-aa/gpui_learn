//! state:主题的状态与装配(数据结构、全局注册、注册表装配与切换)。
//!
//! 细节分层见 crate 根文档;这里只放「状态与装配」,样式集合(颜色结构体)
//! 在 [`styles`](crate::styles),内置配色在 [`builtin`](crate::builtin),
//! JSON 解析在 [`content`](crate::content) / [`loaders`](crate::loaders)。

use std::sync::Arc;

use gpui::{App, Global, WindowAppearance, WindowBackgroundAppearance};

use crate::styles::{
    AccentColors, PlayerColors, StatusColors, SyntaxTheme, SystemColors, ThemeColors,
};

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
    /// 窗口背景外观(不透明 / 半透明 / 模糊),供平台层决定合成方式。
    pub window_background_appearance: WindowBackgroundAppearance,
    pub colors: ThemeColors,
    pub status: StatusColors,
    pub accents: AccentColors,
    pub system: SystemColors,
    /// 协作者配色。数据层先备着——渲染层还没有 decoration 机制,
    /// 等 `_32_painting` 那步再用它画远端光标与选区。
    pub players: PlayerColors,
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

    /// 协作者配色表。
    pub fn players(&self) -> &PlayerColors {
        &self.styles.players
    }

    pub fn syntax(&self) -> &Arc<SyntaxTheme> {
        &self.styles.syntax
    }
}

/// 主题家族:同一套配色的明暗成对(对齐 zed `ThemeFamily`)。
#[derive(Clone, Debug)]
pub struct ThemeFamily {
    /// 家族 id(如 `"catppuccin"`)。
    pub id: String,
    /// 家族名(如 `"Catppuccin"`)。
    pub name: String,
    /// 作者(主题扩展 JSON 里有,内置家族留空)。
    pub author: String,
    /// 家族内的主题(通常 light / dark 各一)。
    pub themes: Vec<Theme>,
}

/// 全局当前主题(gpui `Global`)。
#[derive(Clone)]
pub struct GlobalTheme(pub Arc<Theme>);

impl Global for GlobalTheme {}

/// 内置注册表(gpui `Global`):内置主题 + 运行时注册的主题。
#[derive(Clone)]
pub struct GlobalThemeRegistry(pub crate::builtin::ThemeRegistry);

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
                    crate::builtin::ThemeRegistry::with_builtins()
                        .get("ui-gpui-default-dark")
                        .expect("builtin theme must exist")
                        .clone(),
                )
            }),
        }
    }
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
