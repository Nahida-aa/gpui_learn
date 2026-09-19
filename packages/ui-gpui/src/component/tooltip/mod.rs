//! component/tooltip：悬浮提示（对齐 zed `Tooltip`）。
//!
//! 由两部分组成：
//!
//! - [`Tooltip`]：一个 `ManagedView`，渲染为悬浮卡片（`elevated_surface_background`
//!   圆角底色，可带标题 / 次要说明 / 快捷键提示），与 zed `Tooltip::text`、
//!   `Tooltip::with_key_binding` 对齐。
//! - [`TooltipHost`]：自定义 gpui `Element`，把任意 trigger 子元素包起来，在
//!   鼠标**悬停触发元素**一小段时间（默认 500ms）后在元素下方弹出 [`Tooltip`]，
//!   离开元素 / 任意按下 / 鼠标移出窗口时隐藏。仿照
//!   [`RightClickMenu`](crate::component::context_menu::RightClickMenu) 的实现骨架
//!   （`anchored()` + `deferred(priority=1)` 浮层），触发从右键改为 hover 计时。
//!
//! 用法（与 zed 的 `IconButton::tooltip` 同构）：
//! ```ignore
//! IconButton::new("term", IconName::Terminal)
//!     .tooltip(Tooltip::text("Terminal"))
//!     .on_click(/* ... */)
//! ```

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use gpui::{
    Anchor, AnyElement, App, AppContext, AsyncWindowContext, Bounds, Context, DismissEvent,
    Element, ElementId, Entity, EventEmitter, FocusHandle, Focusable, GlobalElementId, Hitbox,
    HitboxBehavior, InteractiveElement, IntoElement, LayoutId, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, ParentElement, Pixels, Point, Render, SharedString, Window, anchored, deferred,
    div, prelude::*, px,
};

use crate::base::theme::ActiveTheme;

/// 独立的工具提示视图（ManagedView）。由 [`TooltipHost`] 弹出。
pub struct Tooltip {
    title: SharedString,
    meta: Option<SharedString>,
    key_binding: Option<SharedString>,
    focus_handle: FocusHandle,
}

impl Tooltip {
    /// 创建纯文字提示；返回能在 `window`/`cx` 现场建实体的工厂函数，
    /// 可直接传给 [`TooltipHost::tooltip`] / `IconButton::tooltip`。
    pub fn text(
        title: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> Entity<Tooltip> {
        let title = title.into();
        move |_window: &mut Window, cx: &mut App| {
            cx.new(|cx| Tooltip {
                title: title.clone(),
                meta: None,
                key_binding: None,
                focus_handle: cx.focus_handle(),
            })
        }
    }

    /// 带快捷键提示（`<kbd>` 样式）的提示。
    pub fn with_key_binding(
        title: impl Into<SharedString>,
        key_binding: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> Entity<Tooltip> {
        let title = title.into();
        let key_binding = key_binding.into();
        move |_window: &mut Window, cx: &mut App| {
            cx.new(|cx| Tooltip {
                title: title.clone(),
                meta: None,
                key_binding: Some(key_binding.clone()),
                focus_handle: cx.focus_handle(),
            })
        }
    }

    /// 带次要说明（第二行小字）的提示。
    pub fn with_meta(
        title: impl Into<SharedString>,
        meta: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> Entity<Tooltip> {
        let title = title.into();
        let meta = meta.into();
        move |_window: &mut Window, cx: &mut App| {
            cx.new(|cx| Tooltip {
                title: title.clone(),
                meta: Some(meta.clone()),
                key_binding: None,
                focus_handle: cx.focus_handle(),
            })
        }
    }
}

impl Focusable for Tooltip {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for Tooltip {}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors().clone();

        let title = div()
            .max_w(px(288.0))
            .child(self.title.clone())
            .into_any_element();

        let mut row = div().flex().flex_row().items_center().gap_4();

        row = if let Some(key_binding) = self.key_binding.as_ref() {
            row.justify_between().child(title).child(
                div()
                    .text_sm()
                    .text_color(colors.text_muted)
                    .child(key_binding.clone()),
            )
        } else {
            row.child(title)
        };

        // meta 是独立第二行:小号 muted 文字(对齐 zed `Label::Small + Color::Muted`)。
        let meta = self.meta.as_ref().map(|meta| {
            div()
                .max_w(px(288.0))
                .text_sm()
                .text_color(colors.text_muted)
                .child(meta.clone())
        });

        // 卡片本体:字号 text_size(13) 对齐 zed text_ui(app)（UI 字号 ~12-13px）。
        let card = div()
            .py_1()
            .px_2()
            .flex()
            .flex_col()
            .bg(colors.elevated_surface_background)
            .text_color(colors.text)
            .text_size(px(13.0))
            .child(row)
            .when_some(meta, |this, meta| this.child(meta));

        // 外层留白 `pl_2 pt_2p5`（8/10）：避免提示紧贴鼠标（对齐 zed tooltip_container）。
        div().id("tooltip").pl_2().pt_2p5().child(
            card.rounded_md()
                .border_1()
                .border_color(colors.border_variant),
        )
    }
}

/// 工具提示宿主：包裹 trigger 元素，悬停一小段时间后在其下方弹出提示。
pub struct TooltipHost {
    id: ElementId,
    /// trigger 子元素构建（首次 request_layout 消费一次）。
    child_builder: Option<Box<dyn FnOnce(bool, &mut Window, &mut App) -> AnyElement + 'static>>,
    /// hover 延迟结束后现场创建提示实体。
    tooltip_builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static>>,
    /// 悬停多久后显示提示。
    hover_delay: Duration,
    /// 提示哪一角对齐锚点（默认 `Anchor::TopLeft`）。
    anchor: Option<Anchor>,
    /// 锚点取自 trigger 的哪个角（默认 `Anchor::BottomLeft`，即提示在元素下方）。
    attach: Option<Anchor>,
}

impl TooltipHost {
    /// 用给定 id 新建宿主（id 在同一父容器内需唯一）。
    pub fn new(id: impl Into<ElementId>) -> Self {
        TooltipHost {
            id: id.into(),
            child_builder: None,
            tooltip_builder: None,
            hover_delay: Duration::from_millis(500),
            anchor: None,
            attach: None,
        }
    }

    /// 悬停延迟（默认 500ms，对齐 zed 观感）。
    pub fn hover_delay(mut self, delay: Duration) -> Self {
        self.hover_delay = delay;
        self
    }

    /// 设定提示工厂（`Tooltip::text(...)` 等）。
    pub fn tooltip(
        mut self,
        f: impl Fn(&mut Window, &mut App) -> Entity<Tooltip> + 'static,
    ) -> Self {
        self.tooltip_builder = Some(Rc::new(f));
        self
    }

    /// 设定 trigger（悬停时 `is_active` 可用于高亮）。
    pub fn trigger<F, E>(mut self, e: F) -> Self
    where
        F: FnOnce(bool, &mut Window, &mut App) -> E + 'static,
        E: IntoElement + 'static,
    {
        self.child_builder = Some(Box::new(move |is_active, window, cx| {
            e(is_active, window, cx).into_any_element()
        }));
        self
    }

    /// 提示哪一角锚定到锚点（默认 `Anchor::TopLeft`）。
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// 锚点取 trigger 的哪个角（默认 `Anchor::BottomLeft`，提示在元素下方）。
    pub fn attach(mut self, attach: Anchor) -> Self {
        self.attach = Some(attach);
        self
    }

    fn with_element_state<R>(
        &mut self,
        global_id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut TooltipHandleElementState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<TooltipHandleElementState, _>(
            Some(global_id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

/// 创建 [`TooltipHost`]。
pub fn tooltip_host(id: impl Into<ElementId>) -> TooltipHost {
    TooltipHost::new(id)
}

/// 每个宿主元素实例的运行时状态：当前显示的提示、定位点、hover/计时标志。
pub struct TooltipHandleElementState {
    tip: Rc<RefCell<Option<Entity<Tooltip>>>>,
    position: Rc<RefCell<Point<Pixels>>>,
    hover: Rc<Cell<bool>>,
    timer_active: Rc<Cell<bool>>,
}

impl Clone for TooltipHandleElementState {
    fn clone(&self) -> Self {
        Self {
            tip: Rc::clone(&self.tip),
            position: Rc::clone(&self.position),
            hover: Rc::clone(&self.hover),
            timer_active: Rc::clone(&self.timer_active),
        }
    }
}

impl Default for TooltipHandleElementState {
    fn default() -> Self {
        Self {
            tip: Rc::default(),
            position: Rc::new(RefCell::new(Point::new(px(0.0), px(0.0)))),
            hover: Rc::default(),
            timer_active: Rc::default(),
        }
    }
}

pub struct RequestLayoutState {
    child_layout_id: Option<LayoutId>,
    child_element: Option<AnyElement>,
    tooltip_element: Option<AnyElement>,
}

pub struct PrepaintState {
    hitbox: Hitbox,
}

impl Element for TooltipHost {
    type RequestLayoutState = RequestLayoutState;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, element_state, window, cx| {
                let mut tooltip_layout_id = None;

                let tooltip_element = element_state.tip.borrow_mut().as_mut().map(|tip| {
                    let mut anchored = anchored().snap_to_window_with_margin(px(8.0));
                    if let Some(anchor) = this.anchor {
                        anchored = anchored.anchor(anchor);
                    }
                    anchored = anchored.position(*element_state.position.borrow());

                    let mut element = deferred(anchored.child(div().occlude().child(tip.clone())))
                        .with_priority(1)
                        .into_any();

                    tooltip_layout_id = Some(element.request_layout(window, cx));
                    element
                });

                let mut child_element = this.child_builder.take().map(|child_builder| {
                    (child_builder)(element_state.tip.borrow().is_some(), window, cx)
                });

                let child_layout_id = child_element
                    .as_mut()
                    .map(|child_element| child_element.request_layout(window, cx));

                let layout_id = window.request_layout(
                    gpui::Style::default(),
                    tooltip_layout_id.into_iter().chain(child_layout_id),
                    cx,
                );

                (
                    layout_id,
                    RequestLayoutState {
                        child_element,
                        child_layout_id,
                        tooltip_element,
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        let child_bounds = request_layout
            .child_layout_id
            .map(|layout_id| window.layout_bounds(layout_id));

        // 每次布局后把锚点刷新到 trigger 元素的指定角。
        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, element_state, _window, _cx| {
                if let Some(child_bounds) = child_bounds {
                    let attach = this.attach.unwrap_or(Anchor::BottomLeft);
                    // Zed tooltip offset: 鼠标/锚点 + 内部 padding (pl_2=8pt, pt_2p5=10pt)
                    // → tooltip 卡片实际出现在 offset 之后。我们这里给额外 gap 避免紧贴。
                    let gap = px(8.0);
                    let position = match attach {
                        // 上方弹出：锚点取 trigger 顶部，tooltip 的 anchor 取 Bottom（卡片底部对齐锚点）
                        // 所以 position = top - gap（往上偏移 gap，让 tooltip 底部刚好在锚点上方 gap 处）
                        Anchor::TopLeft | Anchor::TopRight => {
                            child_bounds.corner(attach) - Point::new(px(0.0), gap)
                        }
                        // 下方弹出：锚点取 trigger 底部，+ gap 往下
                        _ => child_bounds.corner(attach) + Point::new(px(0.0), gap),
                    };
                    *element_state.position.borrow_mut() = position;
                }
            },
        );

        if let Some(child) = request_layout.child_element.as_mut() {
            child.prepaint(window, cx);
        }

        if let Some(tooltip) = request_layout.tooltip_element.as_mut() {
            tooltip.prepaint(window, cx);
        }

        PrepaintState { hitbox }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint_state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, element_state, window, cx| {
                if let Some(mut child) = request_layout.child_element.take() {
                    child.paint(window, cx);
                }

                if let Some(mut tooltip) = request_layout.tooltip_element.take() {
                    tooltip.paint(window, cx);
                }

                let Some(builder) = this.tooltip_builder.clone() else {
                    return;
                };

                let hover_delay = this.hover_delay;
                let tip = element_state.tip.clone();
                let hover = element_state.hover.clone();
                let timer_active = element_state.timer_active.clone();
                let hitbox_id = prepaint_state.hitbox.id;

                // 悬停计时显示；离开立即隐藏。
                window.on_mouse_event(move |_event: &MouseMoveEvent, _phase, window, cx| {
                    let is_hovered = hitbox_id.is_hovered(window);
                    let was_hovered = hover.get();
                    hover.set(is_hovered);
                    if is_hovered == was_hovered {
                        return;
                    }

                    if !is_hovered {
                        if tip.borrow().is_some() {
                            *tip.borrow_mut() = None;
                            window.refresh();
                        }
                        return;
                    }

                    if timer_active.get() {
                        return;
                    }
                    timer_active.set(true);

                    let builder = builder.clone();
                    let tip = tip.clone();
                    let hover = hover.clone();
                    let timer_active = timer_active.clone();
                    window
                        .spawn(cx, async move |async_cx: &mut AsyncWindowContext| {
                            async_cx.background_executor().timer(hover_delay).await;
                            let _ = async_cx.update(|window, cx| {
                                if !hover.get() {
                                    timer_active.set(false);
                                    return;
                                }
                                *tip.borrow_mut() = Some((builder)(window, cx));
                                timer_active.set(false);
                                window.refresh();
                            });
                        })
                        .detach();
                });

                // 任意按下 / 移出窗口都收起。
                let tip_down = element_state.tip.clone();
                window.on_mouse_event(move |_: &MouseDownEvent, _phase, window, _cx| {
                    if tip_down.borrow().is_some() {
                        *tip_down.borrow_mut() = None;
                        window.refresh();
                    }
                });
                let tip_exit = element_state.tip.clone();
                let hover_exit = element_state.hover.clone();
                let timer_exit = element_state.timer_active.clone();
                window.on_mouse_event(move |_: &MouseExitEvent, _phase, window, _cx| {
                    hover_exit.set(false);
                    timer_exit.set(false);
                    if tip_exit.borrow().is_some() {
                        *tip_exit.borrow_mut() = None;
                        window.refresh();
                    }
                });
            },
        )
    }
}

impl IntoElement for TooltipHost {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
