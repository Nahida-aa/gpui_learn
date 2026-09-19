//! init：主题系统的装配（启动时装一次，运行时切主题）。
//!
//! 对应 zed `crates/theme_settings/src/theme_settings.rs` 的 `init` /
//! `load_bundled_themes` / `load_user_theme`。
//!
//! 这里体现「装载」与「状态」的分工：本包只负责**把主题装进注册表**
//! （[`init_theme`] / [`load_asset_themes`]）与**按名字切**（[`set_theme_by_name`]），
//! 而「当前主题是什么」这份状态归 theme 包的 `GlobalTheme` 管
//! （切换最终调用 `aa_gpui_kit_theme::set_theme`）。

use std::sync::Arc;

use aa_gpui_kit_theme::builtin::ThemeRegistry;
use aa_gpui_kit_theme::{GlobalThemeRegistry, Theme, set_theme};
use gpui::App;

/// 安装主题系统（应用启动时调用一次）：
/// 1. 装入内置主题（`ThemeRegistry::with_builtins`）；
/// 2. 从 `asset_source` 加载 `themes/**/*.json`（[`load_asset_themes`]）；
/// 3. 写入 `GlobalThemeRegistry`；
/// 4. 默认选用 JSON 里的 `Catppuccin Mocha`（与内置深色同配色），
///    找不到再回退内置深色，最后 `set_theme` 生效。
pub fn init_theme(cx: &mut App) {
    let mut registry = ThemeRegistry::with_builtins();
    load_asset_themes(cx, &mut registry);
    cx.set_global(GlobalThemeRegistry(registry));
    set_theme(cx, Arc::new(default_theme(cx)));
}

/// 默认主题：JSON 里的 `Catppuccin Mocha` 优先，找不到再回退内置深色。
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
/// （资产由 assets crate 的 RustEmbed 内嵌）。
///
/// 单个文件解析失败只告警不中断，保证其中一个损坏不影响其余主题。
pub fn load_asset_themes(cx: &mut App, registry: &mut ThemeRegistry) {
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

/// 按 id / 名称切换主题（需已 [`init_theme`]）。找不到时保持不变并返回 false。
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
