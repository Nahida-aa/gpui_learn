//! 主题相关的字体/密度设置,对齐 zed `crates/theme/src/theme_settings_provider.rs`。
//!
//! ## 这个 trait 解决什么问题
//!
//! 主题系统需要知道 UI 字体、等宽字体、各自字号与 UI 密度——但这些值的
//! **来源**是应用设置,而 `theme` crate 不该依赖具体的设置基础设施
//! (zed 里那是 `settings` / `theme_settings` crate,依赖方向是
//! `theme_settings → theme`,反向依赖会成环)。
//!
//! 所以 zed 的做法是:在 `theme` 里定义一个 **provider trait**,由拥有
//! 具体设置的 crate 实现并注册为 gpui 全局;主题侧只通过
//! [`theme_settings`] 取,不关心实现从哪来。
//!
//! ## 与本 crate 已有机制的关系
//!
//! `theme-settings` 包的 `init_theme` 负责主题**配色**的装配;
//! 本模块负责主题相关的**字体与密度**,两者正交:
//! 配色随主题切换,字体/字号/密度随用户设置变化。
//!
//! 我们没有独立的 settings crate,因此默认没有 provider。调用方要么自己
//! 注册一个([`set_theme_settings_provider`]),要么用本模块自带的
//! [`DefaultThemeSettingsProvider`] 作为兜底。

use gpui::{App, Font, Global, Pixels, px};

use crate::ui_density::UiDensity;

/// 提供主题相关的设置(字体、字号、UI 密度),不耦合具体设置基础设施。
///
/// 具体实现由持有那些设置的 crate 提供,并注册为全局。
pub trait ThemeSettingsProvider: Send + Sync + 'static {
    /// UI 元素用的字体。
    fn ui_font<'a>(&'a self, cx: &'a App) -> &'a Font;

    /// 编辑器 / 终端用的等宽字体。
    fn buffer_font<'a>(&'a self, cx: &'a App) -> &'a Font;

    /// UI 字号。
    fn ui_font_size(&self, cx: &App) -> Pixels;

    /// 编辑器字号。
    fn buffer_font_size(&self, cx: &App) -> Pixels;

    /// 当前 UI 密度。
    fn ui_density(&self, cx: &App) -> UiDensity;
}

/// 兜底实现:字号取 gpui 默认(16px),字体用 gpui 的无衬线/等宽默认。
///
/// 没注册 provider 时用它,保证 [`theme_settings`] 恒有值可用——
/// 与 [`ActiveTheme`](crate::ActiveTheme) 未初始化时惰性回退内置主题
/// 是同一个思路。
///
/// 注意:trait 方法只拿得到 `&App`,而 gpui 的 `text_style()` / `rem_size()`
/// 挂在 `Window` 上(需要窗口上下文)。所以这里**不查窗口**,直接给一份
/// 常量默认值——需要跟随窗口文本样式的调用方应当自己注册 provider。
pub struct DefaultThemeSettingsProvider;

impl ThemeSettingsProvider for DefaultThemeSettingsProvider {
    fn ui_font<'a>(&'a self, _cx: &'a App) -> &'a Font {
        static FONT: std::sync::LazyLock<Font> = std::sync::LazyLock::new(|| {
            Font {
                family: ".SystemUIFont".into(),
                ..Default::default()
            }
        });
        &FONT
    }

    fn buffer_font<'a>(&'a self, _cx: &'a App) -> &'a Font {
        static FONT: std::sync::LazyLock<Font> = std::sync::LazyLock::new(|| Font {
            family: "monospace".into(),
            ..Default::default()
        });
        &FONT
    }

    fn ui_font_size(&self, _cx: &App) -> Pixels {
        // gpui 的默认 rem 基准
        px(16.)
    }

    fn buffer_font_size(&self, _cx: &App) -> Pixels {
        px(16.)
    }

    fn ui_density(&self, _cx: &App) -> UiDensity {
        UiDensity::Default
    }
}

/// 全局 provider 的包壳。
struct GlobalThemeSettingsProvider(Box<dyn ThemeSettingsProvider>);

impl Global for GlobalThemeSettingsProvider {}

/// 注册全局 [`ThemeSettingsProvider`],由拥有具体设置的 crate 在启动时调用。
pub fn set_theme_settings_provider(provider: Box<dyn ThemeSettingsProvider>, cx: &mut App) {
    cx.set_global(GlobalThemeSettingsProvider(provider));
}

/// 取全局 [`ThemeSettingsProvider`];未注册时回退 [`DefaultThemeSettingsProvider`]。
///
/// 与 zed 的差异:zed 未注册时 **panic**;我们回退默认实现——
/// 本仓库是教学/组件库场景,拿不到设置不该让整个应用崩掉。
pub fn theme_settings(cx: &App) -> &dyn ThemeSettingsProvider {
    static DEFAULT: DefaultThemeSettingsProvider = DefaultThemeSettingsProvider;
    match cx.try_global::<GlobalThemeSettingsProvider>() {
        Some(global) => &*global.0,
        None => &DEFAULT,
    }
}

/// 便捷函数:当前 UI 字体。
pub fn ui_font(cx: &App) -> Font {
    theme_settings(cx).ui_font(cx).clone()
}

/// 便捷函数:当前 UI 字号。
pub fn ui_font_size(cx: &App) -> Pixels {
    theme_settings(cx).ui_font_size(cx)
}

/// 便捷函数:当前等宽字体。
pub fn buffer_font(cx: &App) -> Font {
    theme_settings(cx).buffer_font(cx).clone()
}

/// 便捷函数:当前编辑器字号。
pub fn buffer_font_size(cx: &App) -> Pixels {
    theme_settings(cx).buffer_font_size(cx)
}

/// 便捷函数:当前 UI 密度。
pub fn ui_density(cx: &App) -> UiDensity {
    theme_settings(cx).ui_density(cx)
}

/// 便捷函数:按当前密度缩放一个间距值。
pub fn scaled_spacing(base: Pixels, cx: &App) -> Pixels {
    px(base.as_f32() * ui_density(cx).spacing_ratio())
}
