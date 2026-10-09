//! Terminal 相关 settings 类型。
//!
//! 对齐 Zed `crates/settings_content/src/terminal.rs`。
//!
//! - `project_terminal` — 项目级（ProjectTerminalSettingsContent 可在 .aa/settings.json 覆盖）
//! - 本模块还包含 TerminalSettingsContent（全局用户 settings.json 的 terminal 块）
//!
//! `Shell` enum 定义在 `project_terminal`，runtime 方法调 `util::shell::ShellKind`。

use schemars::JsonSchema;
mod project;
pub use project::ProjectTerminalSettingsContent;

mod display;
mod dock;
mod shell;
mod toolbar;
pub use display::{
    AlternateScroll, CursorShapeContent, ScrollbarSettingsContent, ShowScrollbar, TerminalBell,
    TerminalBlink, TerminalLineHeight,
};
pub use dock::TerminalDockPosition;
pub use shell::{
    ActivateScript, CondaManager, PathHyperlinkRegex, Shell, ShellDiscriminants, VenvSettings,
    WorkingDirectory, WorkingDirectoryDiscriminants,
};
pub use toolbar::TerminalToolbarContent;

use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::{FontFamilyName, FontSize, FontWeightContent, theme::font::FontFeaturesContent};

/// Terminal settings 顶层 — 被 `#[serde(flatten)]` 内嵌 project 级设置。
///
/// 注意：不能加 `#[with_fallible_options]` —— serde flatten 与该 macro
/// 自动追加的字段级 `#[serde(default)]` 冲突。
/// 改为 struct 级 `#[serde(default)]`，所有字段本来就是 `Option<T>`。
#[derive(Clone, Debug, PartialEq, Default, Serialize, JsonSchema, MergeFrom)]
#[serde(default)]
pub struct TerminalSettingsContent {
    /// 项目级设置（shell、working_directory、env、venv、path_hyperlink）。
    #[serde(flatten)]
    pub project: ProjectTerminalSettingsContent,

    // ---- 字体 ----
    /// 未设时跟随 buffer。
    pub font_size: Option<FontSize>,
    pub font_family: Option<FontFamilyName>,
    /// 未设时跟随 buffer。
    #[schemars(extend("uniqueItems" = true))]
    pub font_fallbacks: Option<Vec<FontFamilyName>>,
    /// 默认：Comfortable
    pub line_height: Option<TerminalLineHeight>,
    pub font_features: Option<FontFeaturesContent>,
    pub font_weight: Option<FontWeightContent>,

    // ---- 光标 ----
    /// 默认：Block
    pub cursor_shape: Option<CursorShapeContent>,
    /// 默认：TerminalControlled
    pub blinking: Option<TerminalBlink>,
    /// 默认：On
    pub alternate_scroll: Option<AlternateScroll>,

    // ---- 交互 ----
    /// 默认：false
    pub option_as_meta: Option<bool>,
    /// 默认：false
    pub copy_on_select: Option<bool>,
    /// 默认：true
    pub keep_selection_on_copy: Option<bool>,
    /// 默认：true（Ctrl/Cmd+点击打开超链接，即使 terminal app 开启 mouse reporting）
    pub open_links_in_mouse_mode: Option<bool>,

    // ---- Dock 相关 ----
    /// 默认：true
    pub button: Option<bool>,
    pub dock: Option<TerminalDockPosition>,
    /// 默认：false
    pub starts_open: Option<bool>,
    /// 默认：true
    pub flexible: Option<bool>,
    /// 默认：640
    pub default_width: Option<crate::PixelSetting>,
    /// 默认：320
    pub default_height: Option<crate::PixelSetting>,

    // ---- Scrolling ----
    /// 默认：10_000（最大 100_000）
    pub max_scroll_history_lines: Option<usize>,
    /// 默认：1.0
    pub scroll_multiplier: Option<f32>,

    // ---- UI 细项 ----
    pub toolbar: Option<TerminalToolbarContent>,
    pub scrollbar: Option<ScrollbarSettingsContent>,
    /// 默认：false（terminal 面板图标上显示 terminal 数量 badge）
    pub show_count_badge: Option<bool>,
    /// 默认：System
    pub bell: Option<TerminalBell>,

    // ---- 可访问性 ----
    /// APCA 最小对比度 0-106，默认 45。
    #[serde(serialize_with = "crate::serialize_optional_f32_with_two_decimal_places")]
    pub minimum_contrast: Option<f32>,
}

crate::fallible_options::flattened_deserialize!(TerminalSettingsContent {
    sections: { project },
    options: {
        font_size, font_family, font_fallbacks, line_height, font_features, font_weight,
        cursor_shape, blinking, alternate_scroll, option_as_meta, copy_on_select,
        keep_selection_on_copy, open_links_in_mouse_mode, button, dock, starts_open, flexible,
        default_width, default_height, max_scroll_history_lines, scroll_multiplier, toolbar,
        scrollbar, minimum_contrast, show_count_badge, bell,
    },
    defaults: {},
});

#[cfg(test)]
mod tests {
    use crate::terminal::shell::{CondaManager, Shell, WorkingDirectory};

    use super::*;
    use serde_json::json;

    #[test]
    fn test_terminal_settings_default() {
        let s: TerminalSettingsContent = serde_json::from_value(json!({})).unwrap();
        assert!(s.font_size.is_none());
        assert!(s.project.shell.is_none());
    }

    #[test]
    fn test_project_shell() {
        let s: TerminalSettingsContent = serde_json::from_value(json!({
            "shell": { "with_arguments": { "program": "/bin/zsh", "args": ["-l"] } }
        }))
        .unwrap();
        match &s.project.shell {
            Some(Shell::WithArguments { program, args, .. }) => {
                assert_eq!(program, "/bin/zsh");
                assert_eq!(args, &["-l"]);
            }
            other => panic!("expected WithArguments, got {:?}", other),
        }
    }

    #[test]
    fn test_terminal_line_height() {
        assert_eq!(TerminalLineHeight::Comfortable.value(), 1.618);
        assert_eq!(TerminalLineHeight::Standard.value(), 1.3);
        assert_eq!(TerminalLineHeight::Custom(2.0).value(), 2.0);
        assert_eq!(TerminalLineHeight::Custom(0.5).value(), 1.0);
    }

    #[test]
    fn test_cursor_shape_deserialize() {
        let s: TerminalSettingsContent = serde_json::from_value(json!({
            "cursor_shape": "bar"
        }))
        .unwrap();
        assert_eq!(s.cursor_shape, Some(CursorShapeContent::Bar));
    }

    #[test]
    fn test_venv_on() {
        let s: TerminalSettingsContent = serde_json::from_value(json!({
            "detect_venv": {
                "on": {
                    "conda_manager": "mamba",
                    "venv_name": ".venv"
                }
            }
        }))
        .unwrap();
        let resolved = s.project.detect_venv.unwrap().resolve().unwrap();
        assert_eq!(resolved.conda_manager, CondaManager::Mamba);
        assert_eq!(resolved.venv_name, ".venv");
    }

    #[test]
    fn test_working_directory() {
        let s: TerminalSettingsContent = serde_json::from_value(json!({
            "working_directory": { "always": { "directory": "~/work" } }
        }))
        .unwrap();
        assert!(matches!(
            s.project.working_directory,
            Some(WorkingDirectory::Always { .. })
        ));
    }

    #[test]
    fn test_dock_position() {
        let s: TerminalSettingsContent = serde_json::from_value(json!({
            "dock": "right"
        }))
        .unwrap();
        assert_eq!(s.dock, Some(TerminalDockPosition::Right));
    }
}
