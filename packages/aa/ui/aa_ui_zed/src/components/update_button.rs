//! UpdateButton：标题栏上的自动更新状态按钮（对齐 zed
//! `crates/ui/src/components/collab/update_button.rs`）。
//!
//! 五种预设状态齐活（zed `update_button.rs:102-132`）：
//! [`UpdateButton::checking`] / [`UpdateButton::downloading`] /
//! [`UpdateButton::installing`] / [`UpdateButton::updated`] /
//! [`UpdateButton::errored`]，业务侧通常按 `AutoUpdateStatus` 挑一个再挂
//! `on_click` / `on_dismiss`。
//!
//! 出处：zed `crates/ui/src/components/collab/update_button.rs`
//! （GPL-3.0-or-later）。差异：zed 的 `Component` preview 依赖它自己的
//! preview 基础设施，没搬。

use crate::prelude::*;
use crate::{ButtonLike, CircularProgress, CommonAnimationExt, Tooltip};
use gpui::{AnyView, ClickEvent, ElementId, RenderOnce, div};

const LOAD_CIRCLE_GLYPH_VIEWBOX: f32 = 16.0;
const LOAD_CIRCLE_GLYPH_STROKE_WIDTH: f32 = 1.2;
const LOAD_CIRCLE_GLYPH_RADIUS: f32 = 5.0;

/// A button component displayed in the title bar to show auto-update status.
#[derive(IntoElement)]
pub struct UpdateButton {
    icon: IconName,
    icon_animate: bool,
    icon_color: Option<Color>,
    message: SharedString,
    tooltip: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>>,
    disabled: bool,
    show_dismiss: bool,
    progress: Option<f32>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_dismiss: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl UpdateButton {
    pub fn new(icon: IconName, message: impl Into<SharedString>) -> Self {
        Self {
            icon,
            icon_animate: false,
            icon_color: None,
            message: message.into(),
            tooltip: None,
            disabled: false,
            show_dismiss: false,
            progress: None,
            on_click: None,
            on_dismiss: None,
        }
    }

    /// Sets whether the icon should have a rotation animation (for progress states).
    pub fn icon_animate(mut self, animate: bool) -> Self {
        self.icon_animate = animate;
        self
    }

    /// Sets the icon color (e.g., for warning/error states).
    pub fn icon_color(mut self, color: impl Into<Option<Color>>) -> Self {
        self.icon_color = color.into();
        self
    }

    /// Sets the tooltip text shown on hover.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(Box::new(Tooltip::text(tooltip.into())));
        self
    }

    /// Sets a tooltip builder invoked on every render, so the tooltip can
    /// display content that changes while it stays visible.
    pub fn tooltip_fn(
        mut self,
        tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.tooltip = Some(Box::new(tooltip));
        self
    }

    /// Shows a dismiss button on the right side.
    pub fn with_dismiss(mut self) -> Self {
        self.show_dismiss = true;
        self
    }

    /// Sets the click handler for the main button area.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Sets the click handler for the dismiss button.
    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn progress(mut self, progress: impl Into<Option<f32>>) -> Self {
        self.progress = progress.into();
        self
    }

    pub fn checking() -> Self {
        Self::new(IconName::LoadCircle, "Checking for Zed Updates…")
            .icon_animate(true)
            .disabled(true)
    }

    pub fn downloading(progress: Option<f32>) -> Self {
        Self::new(IconName::Download, "Downloading Zed Update…")
            .progress(progress)
            .disabled(true)
    }

    pub fn installing(version: impl Into<SharedString>) -> Self {
        Self::new(IconName::LoadCircle, "Installing Zed Update…")
            .icon_animate(true)
            .tooltip(version)
            .disabled(true)
    }

    pub fn updated(version: impl Into<SharedString>) -> Self {
        Self::new(IconName::Download, "Restart to Update")
            .tooltip(version)
            .with_dismiss()
    }

    pub fn errored(error: impl Into<SharedString>) -> Self {
        Self::new(IconName::Warning, "Failed to Update")
            .icon_color(Color::Warning)
            .tooltip(error)
            .with_dismiss()
    }

    pub fn version_tooltip_message(version: impl std::fmt::Display) -> String {
        format!("Update to Version: {version}")
    }

    pub fn downloading_tooltip_message(
        version: impl std::fmt::Display,
        progress: Option<f32>,
    ) -> String {
        let message = Self::version_tooltip_message(version);
        match progress {
            Some(progress) => format!(
                "{message} ({:.0}% downloaded)",
                progress.clamp(0.0, 1.0) * 100.0
            ),
            None => message,
        }
    }
}

impl RenderOnce for UpdateButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let border_color = if self.disabled {
            cx.theme().colors().border
        } else {
            cx.theme().colors().text.opacity(0.15)
        };

        let icon_element = if let Some(progress) = self.progress {
            let progress = progress.clamp(0.0, 1.0);
            let icon_box = IconSize::XSmall.rems().to_pixels(window.rem_size());
            let progress_color = Color::Default.color(cx);
            CircularProgress::new(progress, 1.0, icon_box, cx)
                .stroke_width(
                    icon_box * (LOAD_CIRCLE_GLYPH_STROKE_WIDTH / LOAD_CIRCLE_GLYPH_VIEWBOX),
                )
                .radius(icon_box * (LOAD_CIRCLE_GLYPH_RADIUS / LOAD_CIRCLE_GLYPH_VIEWBOX))
                .bg_color(progress_color.opacity(0.2))
                .progress_color(progress_color)
                .into_any_element()
        } else {
            let icon = Icon::new(self.icon)
                .size(IconSize::XSmall)
                .when_some(self.icon_color, |this, color| this.color(color));
            if self.icon_animate {
                icon.with_rotate_animation(2).into_any_element()
            } else {
                icon.into_any_element()
            }
        };

        let tooltip = self.tooltip;

        let button_id = ElementId::Name(self.message.clone());
        let dismiss_button_id = ElementId::Name(format!("dismiss-{}", self.message).into());

        let label_row = h_flex()
            .h_full()
            .gap_1()
            .child(icon_element)
            .child(Label::new(self.message).size(LabelSize::Small));

        h_flex()
            .mr_2()
            .rounded_sm()
            .border_1()
            .border_color(border_color)
            .child(
                ButtonLike::new(button_id)
                    .child(label_row)
                    .when_some(tooltip, |this, tooltip| this.tooltip(tooltip))
                    .disabled(self.disabled)
                    .when_some(self.on_click, |this, handler| this.on_click(handler)),
            )
            .when(self.show_dismiss, |this| {
                this.child(
                    div().border_l_1().border_color(border_color).child(
                        IconButton::new(dismiss_button_id, IconName::Close)
                            .icon_size(IconSize::Indicator)
                            .when_some(self.on_dismiss, |this, handler| this.on_click(handler))
                            .tooltip(Tooltip::text("Dismiss")),
                    ),
                )
            })
    }
}
