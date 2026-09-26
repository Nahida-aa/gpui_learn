//! `Styled` trait 扩展 —— 提供 Zed 风格的 `h_flex()` / `v_flex()` 快捷方法，
//! 以及 `elevation_1..3` 分层样式（对齐 zed `crates/ui/src/traits/styled_ext.rs`）。

use gpui::{App, Styled};

use crate::ElevationIndex;
use crate::prelude::*;

fn elevated<E: Styled>(this: E, cx: &App, index: ElevationIndex) -> E {
    this.bg(cx.theme().colors().elevated_surface_background)
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .shadow(index.shadow(cx))
}

fn elevated_borderless<E: Styled>(this: E, cx: &mut App, index: ElevationIndex) -> E {
    this.bg(cx.theme().colors().elevated_surface_background)
        .rounded_lg()
        .shadow(index.shadow(cx))
}

/// Extends [`gpui::Styled`] with layout helpers.
pub trait StyledExt: Styled + Sized {
    /// Horizontally stacks elements.
    ///
    /// Sets `flex()`, `flex_row()`, `items_center()`
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    /// Vertically stacks elements.
    ///
    /// Sets `flex()`, `flex_col()`
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    /// The `Surface` elevation level：Title Bar / Panel / Tab Bar / Editor 等。
    fn elevation_1(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::Surface)
    }

    /// 无边框版 [`elevation_1`](Self::elevation_1)。
    fn elevation_1_borderless(self, cx: &mut App) -> Self {
        elevated_borderless(self, cx, ElevationIndex::Surface)
    }

    /// 非模态浮层：Notification / Palette / 游离窗口面板。
    fn elevation_2(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ElevatedSurface)
    }

    /// 无边框版 [`elevation_2`](Self::elevation_2)。
    fn elevation_2_borderless(self, cx: &mut App) -> Self {
        elevated_borderless(self, cx, ElevationIndex::ElevatedSurface)
    }

    /// 模态层（最高）：Settings Modal / Dialog / Wizard。AlertModal 用的就是这层。
    fn elevation_3(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ModalSurface)
    }

    /// 无边框版 [`elevation_3`](Self::elevation_3)。
    fn elevation_3_borderless(self, cx: &mut App) -> Self {
        elevated_borderless(self, cx, ElevationIndex::ModalSurface)
    }
}

impl<E: Styled> StyledExt for E {}
