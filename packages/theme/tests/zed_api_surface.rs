//! API 面核对：zed `crates/theme` 的公开符号在本 crate 应全部可导入。
//!
//! 这份清单来自 zed 的真实调用方代码（用户提供的 import 列表），
//! 外部代码照 zed 的习惯写 import 时不应遇到缺符号。
#[allow(unused_imports)]
use aa_gpui_kit_theme::{
    AccentColors, Appearance, AppearanceContent, DEFAULT_DARK_THEME, DEFAULT_ICON_THEME_NAME,
    GlobalTheme, LoadThemes, PlayerColor, PlayerColors, StatusColors, SyntaxTheme,
    SystemAppearance, SystemColors, Theme, ThemeColors, ThemeFamily, ThemeRegistry,
    ThemeSettingsProvider, ThemeStyles, default_color_scales, try_parse_color,
};

#[test]
fn zed_public_api_surface_is_covered() {
    // 纯导入检查:能编译即通过。放一个使用,避免 all 空警告。
    let _ = DEFAULT_DARK_THEME;
    let _ = DEFAULT_ICON_THEME_NAME;
    let _: Appearance = SystemAppearance::default().0;
}
