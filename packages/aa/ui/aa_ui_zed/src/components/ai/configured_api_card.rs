//! API 已配置卡片（照搬 zed
//! `crates/ui/src/components/ai/configured_api_card.rs`，GPL-3.0-or-later）。
//!
//! ## 与 zed 的差距
//!
//! | zed | 本文件 |
//! |---|---|
//! | `#[derive(IntoElement, RegisterComponent)]` + `impl Component`（preview 示例群） | 只留 `IntoElement`：我们的 `RegisterComponent` 要求同时 `impl Component`，而 preview 依赖 zed 的示例库基建 |
//! | `use crate::{Tooltip, prelude::*};` | 同（`Tooltip` 不在 prelude 里，得单独引入） |
//! | 渲染主体 | 原样 |

use crate::{Tooltip, prelude::*};
use gpui::{ClickEvent, ElementId, IntoElement, ParentElement, SharedString};

#[derive(IntoElement)]
pub struct ConfiguredApiCard {
    id: ElementId,
    label: SharedString,
    button_label: Option<SharedString>,
    button_tab_index: Option<isize>,
    tooltip_label: Option<SharedString>,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl ConfiguredApiCard {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            button_label: None,
            button_tab_index: None,
            tooltip_label: None,
            disabled: false,
            on_click: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn button_label(mut self, button_label: impl Into<SharedString>) -> Self {
        self.button_label = Some(button_label.into());
        self
    }

    pub fn tooltip_label(mut self, tooltip_label: impl Into<SharedString>) -> Self {
        self.tooltip_label = Some(tooltip_label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn button_tab_index(mut self, tab_index: isize) -> Self {
        self.button_tab_index = Some(tab_index);
        self
    }
}

impl RenderOnce for ConfiguredApiCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let button_label = self.button_label.unwrap_or("Reset Key".into());
        let button_id = self.id;

        h_flex()
            .min_w_0()
            .mt_0p5()
            .p_1()
            .flex_wrap()
            .justify_between()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background.opacity(0.5))
            .child(
                h_flex()
                    .min_w_0()
                    .gap_1()
                    .child(Icon::new(IconName::Check).color(Color::Success))
                    .child(Label::new(self.label)),
            )
            .child(
                Button::new(button_id, button_label)
                    .when_some(self.button_tab_index, |elem, tab_index| {
                        elem.tab_index(tab_index)
                    })
                    .label_size(LabelSize::Small)
                    .start_icon(
                        Icon::new(IconName::Undo)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
                    .disabled(self.disabled)
                    .when_some(self.tooltip_label, |this, label| {
                        this.tooltip(Tooltip::text(label))
                    })
                    .when_some(
                        self.on_click.filter(|_| !self.disabled),
                        |this, on_click| this.on_click(on_click),
                    ),
            )
    }
}
