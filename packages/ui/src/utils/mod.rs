pub mod control_characters;

pub use control_characters::*;

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
