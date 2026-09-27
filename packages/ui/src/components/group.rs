//! 间距分组 helper（照搬 zed `crates/ui/src/components/group.rs`，GPL-3.0-or-later）。
//!
//! 一组「横向/纵向 + 固定 gap」的一行函数，zed 的 prelude 里就有
//! （`h_group_sm` / `h_group` / `h_group_lg` / `h_group_xl` 与 `v_*`），
//! AAgent 的 project_diff / solo_diff_view 直接写裸名调用。

use gpui::{Div, div, prelude::*};

/// Creates a horizontal group with tight, consistent spacing.
///
/// xs: ~2px @16px/rem
pub fn h_group_sm() -> Div {
    div().flex().gap_0p5()
}

/// Creates a horizontal group with consistent spacing.
///
/// s: ~4px @16px/rem
pub fn h_group() -> Div {
    div().flex().gap_1()
}

/// Creates a horizontal group with consistent spacing.
///
/// m: ~6px @16px/rem
pub fn h_group_lg() -> Div {
    div().flex().gap_1p5()
}

/// Creates a horizontal group with consistent spacing.
///
/// l: ~8px @16px/rem
pub fn h_group_xl() -> Div {
    div().flex().gap_2()
}

/// Creates a vertical group with tight, consistent spacing.
///
/// xs: ~2px @16px/rem
pub fn v_group_sm() -> Div {
    div().flex().flex_col().gap_0p5()
}

/// Creates a vertical group with consistent spacing.
///
/// s: ~4px @16px/rem
pub fn v_group() -> Div {
    div().flex().flex_col().gap_1()
}

/// Creates a vertical group with consistent spacing.
///
/// m: ~6px @16px/rem
pub fn v_group_lg() -> Div {
    div().flex().flex_col().gap_1p5()
}

/// Creates a vertical group with consistent spacing.
///
/// l: ~8px @16px/rem
pub fn v_group_xl() -> Div {
    div().flex().flex_col().gap_2()
}
