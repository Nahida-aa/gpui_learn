//! iced 侧的主题 —— zed 语义色令牌（`ThemeColors`）的 iced 版本。
//!
//! GPUI 侧（`aa_theme_gpui`）已经把 zed 的 `ThemeColors` 整套搬过来了，但那个 crate
//! 依赖 gpui，iced 前端吃不到。这里把**我们真正用到的那部分令牌**用 iced 的 `Color`
//! 重写一遍，字段名与 zed 一一对应（`surface_background` / `ghost_element_hover`…），
//! 这样照抄 zed 的界面代码时不用做名字翻译。
//!
//! 色值取自 Catppuccin（Macchiato 深色 / Latte 浅色），与
//! `aa_theme_gpui::default_colors` **同源**（注释里保留了「这是哪个 catppuccin 色」的
//! 标注）。改色值请两边一起改。
//!
//! 与上游的差别（有意为之）：
//!
//! - **只有用到的令牌**（约 40 个），不是 zed 那 143 个。需要哪个再加。
//! - **没有主题 JSON / 主题切换**。zed 的主题是运行时拿 JSON 合成再 refine；我们暂时
//!   只有内置的深/浅两套，等设置层做主题选择时再补加载路径。
//! - 颜色类型是 iced 的 `Color`（不是 gpui 的 `Hsla`）。令牌值本来就是 sRGB 十六进制，
//!   转换本身无歧义。
//!
//! ```
//! use aa_theme_iced::Theme;
//!
//! let theme = Theme::dark();
//! assert_eq!(theme.colors().text, aa_theme_iced::colors::rgb(0xcad3f5));
//! ```

pub mod colors;

pub use colors::{Colors, StatusColors, TerminalColors};

/// 主题类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    /// 深色。
    Dark,
    /// 浅色。
    Light,
}

/// 主题：一套语义色 + 一套状态色。
///
/// 对应 zed 的 `Theme`：那边令牌按用途分了 `colors()` / `status()` / `accents()`
/// 等几组，我们目前只需要前两组。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    kind: ThemeKind,
    colors: Colors,
    status: StatusColors,
}

impl Theme {
    /// 内置深色（Catppuccin Macchiato）。
    pub fn dark() -> Self {
        let colors = colors::macchiato();
        Self {
            kind: ThemeKind::Dark,
            colors,
            status: colors.status,
        }
    }

    /// 内置浅色（Catppuccin Latte）。
    pub fn light() -> Self {
        let colors = colors::latte();
        Self {
            kind: ThemeKind::Light,
            colors,
            status: colors.status,
        }
    }

    /// 深浅。
    pub fn kind(&self) -> ThemeKind {
        self.kind
    }

    /// 界面语义色。
    pub fn colors(&self) -> &Colors {
        &self.colors
    }

    /// 状态色（错误 / 警告 / 成功 / 提示）。
    pub fn status(&self) -> &StatusColors {
        &self.status
    }
}

/// 默认深色 —— 与终端调色板（aa_term 里那份 Catppuccin 深色）一致。
impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 深色就是深色：底色的感知亮度要明显低于浅色，别把两套色值写反。
    #[test]
    fn dark_is_darker_than_light() {
        let luminance =
            |color: iced_core::Color| 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
        let dark = luminance(Theme::dark().colors().background);
        let light = luminance(Theme::light().colors().background);
        assert!(dark < 0.2, "深色底色应当很暗，实际 {dark}");
        assert!(light > 0.8, "浅色底色应当很亮，实际 {light}");
    }

    /// 表面色与底色要能分出来，否则侧边栏和主区糊成一片。
    #[test]
    fn the_surface_is_distinguishable_from_the_background() {
        for theme in [Theme::dark(), Theme::light()] {
            let colors = theme.colors();
            assert_ne!(
                colors.surface_background,
                colors.background,
                "{:?} 主题里表面色与底色相同",
                theme.kind()
            );
        }
    }

    /// `ghost_*` 令牌是半透明的 —— 它们要叠在所在表面上（zed 就是这么用的），
    /// 写成不透明会让行悬停时盖掉底色。
    #[test]
    fn ghost_element_tokens_are_translucent() {
        let dark = Theme::dark();
        let colors = dark.colors();
        assert_eq!(colors.ghost_element_background.a, 0.0, "幽灵底本身透明");
        for (name, color) in [
            ("ghost_element_hover", colors.ghost_element_hover),
            ("ghost_element_active", colors.ghost_element_active),
            ("ghost_element_selected", colors.ghost_element_selected),
        ] {
            assert!(
                color.a > 0.0 && color.a < 1.0,
                "{name} 应当是半透明的，实际 alpha={}",
                color.a
            );
        }
        assert_eq!(colors.border_transparent.a, 0.0, "透明边框要真透明");
    }

    /// 状态色得能分辨出四态（写错色值常表现为四个字段全一样）。
    #[test]
    fn status_colors_are_distinct() {
        for theme in [Theme::dark(), Theme::light()] {
            let status = theme.status();
            let all = [status.error, status.warning, status.info, status.success];
            for (index, left) in all.iter().enumerate() {
                for right in &all[index + 1..] {
                    assert_ne!(left, right, "{:?} 主题里有两条状态色相同", theme.kind());
                }
            }
        }
    }

    /// 终端底色要与侧边栏的 `surface` 区分开 —— 终端铺满主区（zed 用 `base`），
    /// 侧边栏/标题栏用 `mantle`，写成同一个值两边就糊成一片了。
    #[test]
    fn the_terminal_background_differs_from_the_sidebar_surface() {
        for theme in [Theme::dark(), Theme::light()] {
            let colors = theme.colors();
            assert_ne!(colors.terminal.background, colors.surface_background);
        }
    }

    /// ANSI 16 色要齐全且两两可分辨（16 个槽位写错常表现为整排同色，
    /// 或 normal 与 bright 抄反）。
    #[test]
    fn the_ansi_palette_is_complete_and_distinct() {
        for theme in [Theme::dark(), Theme::light()] {
            let terminal = theme.colors().terminal;
            let all: Vec<_> = terminal
                .ansi_normal
                .iter()
                .chain(&terminal.ansi_bright)
                .collect();
            for (index, left) in all.iter().enumerate() {
                for right in &all[index + 1..] {
                    assert_ne!(left, right, "{:?} 主题里有两条 ANSI 色相同", theme.kind());
                }
            }
        }
    }

    /// 终端的光标 / 选区取自玩家色，得是这么回事：光标不透明，选区半透明
    /// （选区要能透出底色，写不透明会盖掉文字底下的背景）。
    #[test]
    fn the_terminal_cursor_and_selection_are_sane() {
        for theme in [Theme::dark(), Theme::light()] {
            let terminal = theme.colors().terminal;
            assert_eq!(
                terminal.cursor.a,
                1.0,
                "{:?} 主题的光标不该是半透明的",
                theme.kind()
            );
            assert!(
                terminal.selection_background.a > 0.0 && terminal.selection_background.a < 1.0,
                "{:?} 主题的选区底色应当是半透明的，实际 alpha={}",
                theme.kind(),
                terminal.selection_background.a
            );
        }
    }
}
