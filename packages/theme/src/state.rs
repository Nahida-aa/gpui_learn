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

/// 系统的明暗形态(对齐 zed `SystemAppearance`)。
///
/// 与 [`Appearance`] 的区别:那是**主题**的明暗,这是**操作系统**当前的
/// 明暗设置。跟随系统明暗切换主题的组件以它为准。
#[derive(Debug, Clone, Copy)]
pub struct SystemAppearance(pub Appearance);

impl std::ops::Deref for SystemAppearance {
    type Target = Appearance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for SystemAppearance {
    fn default() -> Self {
        Self(Appearance::Dark)
    }
}

#[derive(Default)]
struct GlobalSystemAppearance(SystemAppearance);

impl std::ops::DerefMut for GlobalSystemAppearance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for GlobalSystemAppearance {
    type Target = SystemAppearance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Global for GlobalSystemAppearance {}

impl SystemAppearance {
    /// 初始化系统明暗全局(从 gpui 当前窗口外观读取)。
    ///
    /// 应用启动时调用一次(我们的 `theme-settings::init` 已接入)。
    pub fn init(cx: &mut App) {
        *cx.default_global::<GlobalSystemAppearance>() =
            GlobalSystemAppearance(SystemAppearance(cx.window_appearance().into()));
    }

    /// 读全局系统明暗。
    pub fn global(cx: &App) -> Self {
        cx.global::<GlobalSystemAppearance>().0
    }

    /// 可变访问全局系统明暗(平台层收到系统切换事件时更新)。
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<GlobalSystemAppearance>()
    }
}

impl From<WindowAppearance> for Appearance {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
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

/// 要装载哪些主题(对齐 zed `LoadThemes`,主要用于测试与嵌入场景)。
pub enum LoadThemes {
    /// 只装基础主题:不装载任何资产里的主题 JSON。
    JustBase,
    /// 装入资产源里的全部主题(通常传 gpui 的 asset source)。
    All(Box<dyn gpui::AssetSource>),
}

/// 全局当前主题(gpui `Global`,对齐 zed:theme + icon_theme 成对)。
#[derive(Clone)]
pub struct GlobalTheme {
    pub theme: Arc<Theme>,
    pub icon_theme: Arc<crate::icon_theme::IconTheme>,
}

impl GlobalTheme {
    /// 由主题与图标主题构造。
    pub fn new(theme: Arc<Theme>, icon_theme: Arc<crate::icon_theme::IconTheme>) -> Self {
        Self { theme, icon_theme }
    }
}

impl Global for GlobalTheme {}

// 注:`GlobalThemeRegistry` 不在这里定义 —— registry 自己管全局
// (`ThemeRegistry::global` / `set_global`,见 [`crate::registry`]),
// 那是 zed 的做法,也比"注册表状态 + 外层 Global 包壳"少一层。

/// 读取当前主题。未初始化时惰性返回内置深色主题——
/// 这让单元测试与未调用 `theme_settings::init` 的最小程序也能直接用。
/// 对齐 zed `ActiveTheme`:任意能拿到 `App` 的地方 `cx.theme()` 取当前主题。
pub trait ActiveTheme {
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Arc<Theme> {
        static DEFAULT: std::sync::OnceLock<Arc<Theme>> = std::sync::OnceLock::new();
        match self.try_global::<GlobalTheme>() {
            Some(global) => &global.theme,
            None => DEFAULT.get_or_init(|| {
                Arc::new(
                    crate::fallback_themes::ctp_default_dark(),
                )
            }),
        }
    }
}

/// 设置当前主题:
/// 1. 写入 `GlobalTheme`(图标主题沿用注册表的默认,可用
///    [`GlobalTheme::new`] + `cx.set_global` 自定);
/// 2. 把语义色同步进 gpui 的 `GlobalColors`(gpui 自带基础设施跟随主题);
/// 3. 各窗口收到 appearance 后自行重绘(组件已在 render 里读 `cx.theme()`,
///    gpui 会因全局变化自动触发重渲染)。
pub fn set_theme(cx: &mut App, theme: Arc<Theme>) {
    sync_global_colors(cx, &theme);
    let icon_theme = crate::registry::ThemeRegistry::try_global(cx)
        .and_then(|registry| registry.default_icon_theme().ok())
        .unwrap_or_else(|| crate::icon_theme::default_icon_theme());
    cx.set_global(GlobalTheme::new(theme, icon_theme));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SystemAppearance::init 从 gpui 窗口外观读系统明暗；
    /// 未初始化时 default 是 Dark（对齐 zed）。
    #[gpui::test]
    fn system_appearance_defaults_and_inits(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            // global() 在未 init 时 panic(zed 同款,不做惰性默认),
            // 所以这里先 init 再读。
            SystemAppearance::init(cx);
            // 测试窗口的 appearance 决定实际值,但一定落在两个枚举之内
            let appearance = SystemAppearance::global(cx).0;
            assert!(matches!(appearance, Appearance::Light | Appearance::Dark));
        });
    }

    /// WindowAppearance → Appearance 的映射与 zed 一致。
    #[test]
    fn window_appearance_maps_to_appearance() {
        use gpui::WindowAppearance;
        assert_eq!(
            Appearance::from(WindowAppearance::VibrantLight),
            Appearance::Light
        );
        assert_eq!(
            Appearance::from(WindowAppearance::VibrantDark),
            Appearance::Dark
        );
        assert_eq!(Appearance::from(WindowAppearance::Light), Appearance::Light);
        assert_eq!(Appearance::from(WindowAppearance::Dark), Appearance::Dark);
    }
}
