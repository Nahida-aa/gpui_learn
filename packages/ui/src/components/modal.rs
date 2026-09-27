//! 弹窗组件（对齐 zed `crates/ui/src/components/modal.rs`）。
//!
//! 目前只搬了 [`ModalHeader`]（弹窗头部：可选图标 / 标题 / 描述 / 返回与关闭
//! 按钮）。zed 同文件里的 `Modal` / `ModalRow` / `ModalFooter` 尚未搬 ——
//! 等有调用面再照搬，避免背上没人用的 API。

use crate::{IconButtonShape, prelude::*};

use gpui::{prelude::FluentBuilder, *};
use smallvec::SmallVec;

#[derive(IntoElement)]
pub struct ModalHeader {
    icon: Option<Icon>,
    headline: Option<SharedString>,
    description: Option<SharedString>,
    children: SmallVec<[AnyElement; 2]>,
    show_dismiss_button: bool,
    show_back_button: bool,
}

impl Default for ModalHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModalHeader {
    pub fn new() -> Self {
        Self {
            icon: None,
            headline: None,
            description: None,
            children: SmallVec::new(),
            show_dismiss_button: false,
            show_back_button: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the headline of the modal.
    ///
    /// This will insert the headline as the first item
    /// of `children` if it is not already present.
    pub fn headline(mut self, headline: impl Into<SharedString>) -> Self {
        self.headline = Some(headline.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn show_dismiss_button(mut self, show: bool) -> Self {
        self.show_dismiss_button = show;
        self
    }

    pub fn show_back_button(mut self, show: bool) -> Self {
        self.show_back_button = show;
        self
    }
}

impl ParentElement for ModalHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for ModalHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut children = self.children;

        if let Some(headline) = self.headline {
            children.insert(
                0,
                Headline::new(headline)
                    .size(HeadlineSize::Small)
                    .color(Color::Muted)
                    .into_any_element(),
            );
        }

        h_flex()
            .min_w_0()
            .flex_none()
            .justify_between()
            .w_full()
            .px(DynamicSpacing::Base12.rems(cx))
            .pt(DynamicSpacing::Base08.rems(cx))
            .pb(DynamicSpacing::Base04.rems(cx))
            .gap(DynamicSpacing::Base08.rems(cx))
            .when(self.show_back_button, |this| {
                this.child(
                    IconButton::new("back", IconName::ArrowLeft)
                        .shape(IconButtonShape::Square)
                        .on_click(|_, window, cx| {
                            window.dispatch_action(menu::Cancel.boxed_clone(), cx);
                        }),
                )
            })
            .child(
                v_flex()
                    .min_w_0()
                    .flex_1()
                    .child(
                        h_flex()
                            .w_full()
                            .gap_1()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .when_some(self.icon, |this, icon| this.child(icon))
                                    .children(children),
                            )
                            .when(self.show_dismiss_button, |this| {
                                this.child(
                                    IconButton::new("dismiss", IconName::Close)
                                        .icon_size(IconSize::Small)
                                        .on_click(|_, window, cx| {
                                            window.dispatch_action(menu::Cancel.boxed_clone(), cx);
                                        }),
                                )
                            }),
                    )
                    .when_some(self.description, |this, description| {
                        this.child(Label::new(description).color(Color::Muted).mb_2().flex_1())
                    }),
            )
    }
}
