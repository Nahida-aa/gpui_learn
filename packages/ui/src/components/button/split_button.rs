use gpui::{
    AnyElement, App, BoxShadow, ParentElement, RenderOnce, Styled, Window, div, hsla, prelude::*,
    px,
};

use aa_gpui_kit_theme::ActiveTheme;

use crate::ElevationIndex;
use crate::traits::StyledExt;

use super::{ButtonLike, IconButton};

#[derive(Clone, Copy, PartialEq)]
pub enum SplitButtonStyle {
    Filled,
    Outlined,
    Transparent,
}

pub enum SplitButtonKind {
    ButtonLike(ButtonLike),
    IconButton(IconButton),
}

impl From<IconButton> for SplitButtonKind {
    fn from(icon_button: IconButton) -> Self {
        Self::IconButton(icon_button)
    }
}

impl From<ButtonLike> for SplitButtonKind {
    fn from(button_like: ButtonLike) -> Self {
        Self::ButtonLike(button_like)
    }
}

/// /// A button with two parts: a primary action on the left and a secondary action on the right.
///
/// The left side is a [`ButtonLike`] with the main action, while the right side can contain
/// any element (typically a dropdown trigger or similar).
///
/// The two sections are visually separated by a divider, but presented as a unified control.
///
/// 对齐 zed `SplitButton`。与 zed 的差异：Filled 的背景我们直接取
/// `colors().background` —— zed 走 `ElevationIndex::Surface.on_elevation_bg`，
/// 该海拔的取色结果与之相同；海拔体系（浮层阴影分层）待浮层阶段建立后接入。
#[derive(IntoElement)]
pub struct SplitButton {
    left: SplitButtonKind,
    right: AnyElement,
    style: SplitButtonStyle,
}

impl SplitButton {
    pub fn new(left: impl Into<SplitButtonKind>, right: AnyElement) -> Self {
        Self {
            left: left.into(),
            right,
            style: SplitButtonStyle::Filled,
        }
    }

    pub fn style(mut self, style: SplitButtonStyle) -> Self {
        self.style = style;
        self
    }
}

impl RenderOnce for SplitButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_filled_or_outlined = matches!(
            self.style,
            SplitButtonStyle::Filled | SplitButtonStyle::Outlined
        );

        let outline = BoxShadow::new(px(0.), px(0.), cx.theme().colors().border.opacity(0.8))
            .spread_radius(px(1.))
            .inset();

        div()
            .h_flex()
            .when(is_filled_or_outlined, |this| this.relative().rounded_sm())
            .when(self.style == SplitButtonStyle::Transparent, |this| {
                this.gap_px()
            })
            .child(div().flex_grow_1().child(match self.left {
                SplitButtonKind::ButtonLike(button) => button.into_any_element(),
                SplitButtonKind::IconButton(icon) => icon.into_any_element(),
            }))
            // 分隔线：等价 Divider::vertical().h_full()——我们的 Divider 尚未
            // impl Styled（固定 h_4），故这里直接画线，待其支持后换回。
            .child(
                div()
                    .when(is_filled_or_outlined, |this| {
                        this.w_px().h_full().bg(cx.theme().colors().border)
                    })
                    .when(self.style == SplitButtonStyle::Transparent, |this| {
                        this.w_px().h_4().bg(cx.theme().colors().border)
                    }),
            )
            .child(self.right)
            .when(is_filled_or_outlined, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded_sm()
                        .shadow(vec![outline]),
                )
            })
            .when(self.style == SplitButtonStyle::Filled, |this| {
                this.bg(ElevationIndex::Surface.on_elevation_bg(cx))
                    .shadow(vec![BoxShadow::new(
                        px(0.),
                        px(1.),
                        hsla(0.0, 0.0, 0.0, 0.16),
                    )])
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_gpui_base::IconName;

    /// 冒烟：left/right 两种组合的 builder 链完整走一遍。
    #[test]
    fn split_button_builds_with_both_kinds() {
        let _with_icon = SplitButton::new(
            IconButton::new("primary", IconName::ChevronDown),
            div().child("dropdown").into_any_element(),
        )
        .style(SplitButtonStyle::Filled);

        let _with_button_like = SplitButton::new(
            ButtonLike::new("primary-label").label("保存"),
            div().child("dropdown").into_any_element(),
        )
        .style(SplitButtonStyle::Outlined);

        let _transparent = SplitButton::new(
            IconButton::new("primary", IconName::ChevronDown),
            div().child("dropdown").into_any_element(),
        )
        .style(SplitButtonStyle::Transparent);
    }
}
