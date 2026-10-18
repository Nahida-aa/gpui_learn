//! 进度条（照搬 zed `crates/ui/src/components/progress/progress_bar.rs`，
//! GPL-3.0-or-later）。
//!
//! ## 与 zed 的差距
//!
//! | zed | 本文件 |
//! |---|---|
//! | 路径 `components/progress/progress_bar.rs`（与 `circular_progress.rs` 同在 `progress/` 下） | 平铺为 `components/progress_bar.rs`：我们先前搬的 `CircularProgress` 也是平铺的（`components/circular_progress.rs`），保持本包内部一致，不为了对齐目录去挪文件 |
//! | `#[derive(IntoElement, RegisterComponent, Documented)]` + `impl Component`（preview） | 只留 `IntoElement`；`Documented` 只为 `Component::description` 服务，一并去掉 |
//! | 渲染主体 | 原样 |

use gpui::Hsla;

use crate::prelude::*;

/// A progress bar is a horizontal bar that communicates the status of a process.
///
/// A progress bar should not be used to represent indeterminate progress.
#[derive(IntoElement)]
pub struct ProgressBar {
    id: ElementId,
    value: f32,
    max_value: f32,
    bg_color: Hsla,
    over_color: Hsla,
    fg_color: Hsla,
}

impl ProgressBar {
    pub fn new(id: impl Into<ElementId>, value: f32, max_value: f32, cx: &App) -> Self {
        Self {
            id: id.into(),
            value,
            max_value,
            bg_color: cx.theme().colors().background,
            over_color: cx.theme().status().error,
            fg_color: cx.theme().status().info,
        }
    }

    /// Sets the current value of the progress bar.
    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    /// Sets the maximum value of the progress bar.
    pub fn max_value(mut self, max_value: f32) -> Self {
        self.max_value = max_value;
        self
    }

    /// Sets the background color of the progress bar.
    pub fn bg_color(mut self, color: Hsla) -> Self {
        self.bg_color = color;
        self
    }

    /// Sets the foreground color of the progress bar.
    pub fn fg_color(mut self, color: Hsla) -> Self {
        self.fg_color = color;
        self
    }

    /// Sets the over limit color of the progress bar.
    pub fn over_color(mut self, color: Hsla) -> Self {
        self.over_color = color;
        self
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let fill_width = (self.value / self.max_value).clamp(0.02, 1.0);

        div()
            .id(self.id.clone())
            .w_full()
            .h_2()
            .p_0p5()
            .rounded_full()
            .bg(self.bg_color)
            .shadow(vec![gpui::BoxShadow::new(
                px(0.),
                px(1.),
                gpui::black().opacity(0.08),
            )])
            .child(
                div()
                    .h_full()
                    .rounded_full()
                    .when(self.value > self.max_value, |div| div.bg(self.over_color))
                    .when(self.value <= self.max_value, |div| div.bg(self.fg_color))
                    .w(relative(fill_width)),
            )
    }
}
