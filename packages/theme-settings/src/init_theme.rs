//! init_theme：主题系统的装配（启动时装一次，运行时切主题）。
//!
//! 对应 zed `crates/theme_settings` 的 `init` / `load_bundled_themes` /
//! `load_user_theme`（zed 把它们放在 lib 根；我们独立成模块，模块名带
//! `_theme` 后缀以避免与公开函数 [`init`] 撞名）。
//!
//! 这里体现「装载」与「状态」的分工：本模块只负责**把主题装进注册表**
//! （[`init`] / [`load_asset_themes`]）与**按名字切**（[`set_theme_by_name`]），
//! 而「当前主题是什么」这份状态归 theme 包的 `GlobalTheme` 管
//! （切换最终调用 `aa_gpui_kit_theme::set_theme`）。

use std::borrow::Cow;
use std::sync::Arc;

use aa_gpui_kit_theme::default_colors::{catppuccin_latte, catppuccin_mocha};
use aa_gpui_kit_theme::registry::ThemeRegistry;
use aa_gpui_kit_theme::{FontFamilyCache, LoadThemes, SystemAppearance, Theme, set_theme};
use gpui::{App, AssetSource, Result, SharedString};

/// 把 gpui 全局里的 `Arc<dyn AssetSource>` 适配成注册表要的
/// `Box<dyn AssetSource>`。
///
/// gpui 只给 `()` 提供了 `AssetSource` 实现，而 `App::asset_source()`
/// 返回的是 `&Arc<dyn AssetSource>` —— 两者不能直接对接，故这里包一层。
/// 对外导出：调用方也可以用 [`LoadThemes::All`] 直接传自己的资产源。
pub struct GlobalAssets(pub Arc<dyn AssetSource>);

impl AssetSource for GlobalAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        self.0.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        self.0.list(path)
    }
}

/// 装入内置主题（Catppuccin Mocha / Latte）。
///
/// 注册表构造时已装入兜底家族；这里再补上成对的 Mocha / Latte。
fn builtin_family() -> aa_gpui_kit_theme::ThemeFamily {
    aa_gpui_kit_theme::ThemeFamily {
        id: "ui-gpui-default".into(),
        name: "ui-gpui Default".into(),
        author: String::new(),
        themes: vec![catppuccin_mocha(), catppuccin_latte()],
    }
}

/// 安装主题系统（应用启动时调用一次）。
///
/// 形状对齐 zed 的 `theme::init(themes_to_load, cx)`：初始化系统明暗、
/// 字体缓存与注册表，再选默认主题。
///
/// 1. 初始化 [`SystemAppearance`]（从 gpui 窗口外观读系统明暗）与
///    字体家族缓存；
/// 2. 按 `themes_to_load` 构造注册表并写入全局；
/// 3. 装入内置主题家族（Catppuccin Mocha / Latte 成对）；
/// 4. [`LoadThemes::All`] 时再从资产加载 `themes/**/*.json`；
/// 5. 默认选用 `Catppuccin Mocha`，找不到再回退内置深色，最后 `set_theme` 生效。
pub fn init(themes_to_load: LoadThemes, cx: &mut App) {
    SystemAppearance::init(cx);
    FontFamilyCache::init_global(cx);

    let assets: Box<dyn AssetSource> = match themes_to_load {
        LoadThemes::JustBase => Box::new(()),
        LoadThemes::All(assets) => assets,
    };
    ThemeRegistry::set_global(assets, cx);
    let registry = ThemeRegistry::global(cx);

    registry.insert_theme_families([builtin_family()]);
    // JustBase 的空资产源 list 返回空，循环零次 —— 与 zed 同一行为，
    // 不需要按分支特判。
    load_asset_themes(&registry);

    set_theme(cx, Arc::new(default_theme(cx)));
}

/// 默认主题：`Catppuccin Mocha` 优先（资产里的主题扩展），
/// 找不到再回退内置深色（id `ui-gpui-default-dark`）。
///
/// 注册表的 `get` 返回 `Result`，所以这里逐个尝试；两个都失败时兜底
/// 直接构造内置主题 —— 兜底路径不该再依赖注册表。
fn default_theme(cx: &App) -> Theme {
    let registry = ThemeRegistry::global(cx);
    registry
        .get("Catppuccin Mocha")
        .or_else(|_| registry.get("ui-gpui-default-dark"))
        .map(|theme| (*theme).clone())
        .unwrap_or_else(|_| catppuccin_mocha())
}

/// 从资产源加载 `themes/` 下的所有主题 JSON 进注册表
/// （资产由 assets crate 的 RustEmbed 内嵌）。
///
/// 单个文件解析失败只告警不中断，保证其中一个损坏不影响其余主题。
pub fn load_asset_themes(registry: &ThemeRegistry) {
    let Ok(paths) = registry.assets().list("themes/") else {
        return;
    };
    for path in paths {
        if !path.ends_with(".json") {
            continue;
        }
        let Ok(Some(bytes)) = registry.assets().load(&path) else {
            continue;
        };
        match crate::loaders::parse_theme_family(&bytes) {
            Ok(family) => {
                tracing::info!("theme family: {} (n={})", family.name, family.themes.len());
                registry.insert_theme_families([family]);
            }
            Err(err) => tracing::warn!("failed to parse theme {path}: {err:#}"),
        }
    }
}

/// 按 id / 名称切换主题（需已 [`init`]）。找不到时保持不变并返回 false。
pub fn set_theme_by_name(cx: &mut App, id_or_name: &str) -> bool {
    let Ok(theme) = ThemeRegistry::global(cx).get(id_or_name) else {
        return false;
    };
    set_theme(cx, theme);
    true
}

/// 列出注册表里全部主题名（调试 / 主题选择器用）。
pub fn list_theme_names(cx: &App) -> Vec<gpui::SharedString> {
    ThemeRegistry::global(cx).list_names()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_gpui_kit_theme::ActiveTheme as _;

    /// 兜底路径：未 `init` 时 `cx.theme()` 也应拿到内置深色主题。
    /// 这条链路在注册表重构后改成直接构造，单独锁一下。
    #[gpui::test]
    fn theme_falls_back_to_builtin_dark(cx: &mut gpui::TestAppContext) {
        let theme = cx.update(|cx| cx.theme().clone());
        assert_eq!(theme.id, "ui-gpui-default-dark");
        assert_eq!(theme.name, "ui-gpui Dark");
    }

    /// 内置家族是成对的深浅主题（纯函数，不需要 App）。
    #[test]
    fn builtin_family_has_dark_and_light() {
        let family = builtin_family();
        assert_eq!(family.themes.len(), 2);
        assert_eq!(family.themes[0].id, "ui-gpui-default-dark");
        assert_eq!(family.themes[1].id, "ui-gpui-default-light");
        // 注册表默认构造（无资产源）也会装入兜底家族，不应 panic
        let registry = ThemeRegistry::default();
        let _ = registry.list_names();
    }
}
