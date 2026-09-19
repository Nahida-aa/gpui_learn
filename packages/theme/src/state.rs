//! state:主题的状态与装配(数据结构、全局注册、注册表装配与切换)。
//!
//! 细节分层见 crate 根文档;这里只放「状态与装配」,颜色定义在
//! [`colors`](crate::colors),内置配色在 [`builtin`](crate::builtin),
//! JSON 解析在 [`content`](crate::content) / [`loaders`](crate::loaders)。

use std::sync::Arc;

use gpui::{App, Global, WindowAppearance};

pub use crate::colors::{AccentColors, StatusColors, SystemColors, ThemeColors};
pub use crate::syntax::SyntaxTheme;

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

/// 安装主题系统(应用启动时调用一次):
/// 1. 从 `asset_source` 加载 `themes/**/*.json`([`load_asset_themes`]);
/// 2. 合并内置主题,写入 [`GlobalThemeRegistry`];
/// 3. 默认选用 JSON 里的 `Catppuccin Mocha`(与旧硬编码同配色,视觉不变),
///    找不到再回退内置深色,最后`set_theme` 生效。
pub fn init_theme(cx: &mut App) {
    let mut registry = crate::builtin::ThemeRegistry::with_builtins();
    load_asset_themes(cx, &mut registry);
    cx.set_global(GlobalThemeRegistry(registry));
    set_theme(cx, Arc::new(default_theme(cx)));
}

/// 默认主题:JSON 里的 `Catppuccin Mocha` 优先(与旧硬编码同配色,视觉不变),
/// 找不到再回退内置深色。
fn default_theme(cx: &App) -> Theme {
    cx.try_global::<GlobalThemeRegistry>()
        .and_then(|registry| {
            registry
                .0
                .get("Catppuccin Mocha")
                .or_else(|| registry.0.get("ui-gpui-default-dark"))
                .cloned()
        })
        .expect("theme registry must have a default")
}

/// 从 `asset_source` 加载 `themes/` 下的所有主题 JSON 进注册表
/// (assets crate 的 RustEmbed 内嵌 `themes/**/*.json`)。
///
/// 单个文件解析失败只告警不中断,保证其中一个损坏不影响其余主题。
pub fn load_asset_themes(cx: &mut App, registry: &mut crate::builtin::ThemeRegistry) {
    let Ok(paths) = cx.asset_source().list("themes/") else {
        return;
    };
    for path in paths {
        if !path.ends_with(".json") {
            continue;
        }
        let Ok(Some(bytes)) = cx.asset_source().load(&path) else {
            continue;
        };
        match crate::loaders::parse_theme_family(&bytes) {
            Ok(family) => {
                tracing::info!("theme family: {} (n={})", family.name, family.themes.len());
                registry.load_theme_family(family);
            }
            Err(err) => tracing::warn!("failed to parse theme {path}: {err:#}"),
        }
    }
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
