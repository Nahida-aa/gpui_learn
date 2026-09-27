use aa_gpui_kit_theme::ActiveTheme as _;
use gpui::App;

pub mod apca_contrast;
pub mod color_contrast;
pub mod constants;
pub mod control_characters;
pub mod search_input;
pub mod with_rem_size;

pub use apca_contrast::*;
pub use color_contrast::*;
pub use constants::*;
pub use control_characters::*;
pub use search_input::*;
pub use with_rem_size::*;

/// 返回「在文件管理器中显示」这一动作的本地化文案（对齐 zed
/// `crates/ui/src/utils.rs:45`）。
///
/// 远程项目下没有 Finder / Explorer 可显示，一律退回通用文案。
pub fn reveal_in_file_manager_label(is_remote: bool) -> &'static str {
    if cfg!(target_os = "macos") && !is_remote {
        "Reveal in Finder"
    } else if cfg!(target_os = "windows") && !is_remote {
        "Reveal in File Explorer"
    } else {
        "Reveal in File Manager"
    }
}

/// 把首字符改成大写，其余原样保留（对齐 zed `crates/ui/src/utils.rs:69`）。
///
/// 注意它不是「首字母大写 + 其余小写」：`capitalize("WORLD") == "WORLD"`。
/// `KeyBinding` 用它把 `cmd` / `escape` 之类的按键名显示成 `Cmd` / `Escape`。
///
/// # Examples
///
/// ```
/// use aa_gpui_kit_ui::utils::capitalize;
///
/// assert_eq!(capitalize("hello"), "Hello");
/// assert_eq!(capitalize("WORLD"), "WORLD");
/// assert_eq!(capitalize(""), "");
/// ```
pub fn capitalize(str: &str) -> String {
    let mut chars = str.chars();
    match chars.next() {
        None => String::new(),
        Some(first_char) => first_char.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// 编辑器缓冲区文本样式（对齐 zed `crates/ui/src/utils.rs:29`）：等宽字体 +
/// 缓冲区字号 + 主题正文色，行高 1.0。
///
/// outline / markdown 预览这些「按编辑器排版渲染」的组件用它取统一字体。
pub fn buffer_text_style(cx: &App) -> gpui::TextStyle {
    let buffer_font = aa_gpui_kit_theme::buffer_font(cx);
    gpui::TextStyle {
        color: cx.theme().colors().text,
        font_family: buffer_font.family.clone(),
        font_features: buffer_font.features.clone(),
        font_fallbacks: buffer_font.fallbacks.clone(),
        font_size: gpui::AbsoluteLength::from(aa_gpui_kit_theme::buffer_font_size(cx)),
        font_weight: buffer_font.weight,
        line_height: gpui::relative(1.),
        ..gpui::TextStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalizes_only_the_first_char() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("WORLD"), "WORLD");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
        // 非 ASCII 首字符也要正确大写（多字节不能切开）。
        assert_eq!(capitalize("élan"), "Élan");
    }
}
