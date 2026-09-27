//! 缩进参考线（对齐 zed `crates/ui/src/components/indent_guides.rs`）。
//!
//! 只搬了 [`IndentGuideColors`] 与 [`indent_guides`] 需要的常量：git_graph /
//! git_panel 这类面板自己拿 `IndentGuideColors::panel(cx)` 配色，真正的
//! `IndentGuides` 元素（自绘 canvas + hitbox 交互，zed 里几百行）暂未搬。
//!
//! 出处：zed `crates/ui/src/components/indent_guides.rs`（GPL-3.0-or-later）。

use crate::prelude::*;
use gpui::Hsla;

/// Represents the colors used for different states of indent guides.
#[derive(Debug, Clone)]
pub struct IndentGuideColors {
    /// The color of the indent guide when it's neither active nor hovered.
    pub default: Hsla,
    /// The color of the indent guide when it's hovered.
    pub hover: Hsla,
    /// The color of the indent guide when it's active.
    pub active: Hsla,
}

impl IndentGuideColors {
    /// Returns the indent guide colors that should be used for panels.
    pub fn panel(cx: &App) -> Self {
        Self {
            default: cx.theme().colors().panel_indent_guide,
            hover: cx.theme().colors().panel_indent_guide_hover,
            active: cx.theme().colors().panel_indent_guide_active,
        }
    }
}
