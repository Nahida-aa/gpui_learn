//! context_menu/right_click_menu：右键菜单宿主元素。
//!
//! 移植自 zed `crates/ui/src/components/right_click_menu.rs` 的核心机制：
//! 一个自定义 gpui `Element`，把任意 trigger 子元素包起来，在**右键**（`MouseDown` +
//! `MouseButton::Right`，命中本元素 hitbox）时：
//!
//! 1. 通过 `menu_builder` 现场创建一个菜单实体（如 [`ContextMenu`](super::menu)）；
//! 2. 用 `anchored()` + `deferred(priority=1)` 把菜单渲染为覆盖在内容上方的浮层，
//!    定位到鼠标坐标（或 `attach` 指定的子元素角落）；
//! 3. 菜单容器带 `occlude()`，点击菜单外部即触发菜单的 `DismissEvent`；
//! 4. 订阅 `DismissEvent` 清除菜单、归还之前持有焦点的控件。
//!
//! 之所以是自定义 `Element` 而不是普通组件，是因为菜单的定位/关闭都依赖
//! `window` 层的 hitbox 与鼠标事件分发，这是组件层拿不到的能力。
//! 与 zed 版一致，无 overlay 层即可工作。

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Bounds, DismissEvent, DispatchPhase, Element, ElementId, Entity,
    Focusable as _, GlobalElementId, Hitbox, HitboxBehavior, InteractiveElement, IntoElement,
    LayoutId, ManagedView, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Window,
    anchored, deferred, div, px,
};

/// 右键菜单宿主：包裹 trigger 元素，右键时在其上方弹出一个菜单实体。
pub struct RightClickMenu<M: ManagedView> {
    id: ElementId,
    /// trigger 子元素构建（首次 request_layout 消费一次，结果为 `is_active` 已知）。
    child_builder: Option<Box<dyn FnOnce(bool, &mut Window, &mut App) -> AnyElement + 'static>>,
    /// 右键时现场创建菜单实体。
    menu_builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> Option<Entity<M>> + 'static>>,
    /// 菜单哪一角对齐锚点（默认 `Anchor::TopLeft`，即菜单右下方展开）。
    anchor: Option<Anchor>,
    /// 锚点取自 trigger 的哪个角（默认鼠标位置）。
    attach: Option<Anchor>,
}

impl<M: ManagedView> RightClickMenu<M> {
    /// 设定菜单创建器（`Entity<M>` 需实现 `ManagedView`）。
    pub fn menu(mut self, f: impl Fn(&mut Window, &mut App) -> Entity<M> + 'static) -> Self {
        self.menu_builder = Some(Rc::new(move |window, cx| Some(f(window, cx))));
        self
    }

    /// 设定 trigger（右击时高亮可用 `is_active` 改变外观）。
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

    /// 菜单哪一角锚定到锚点（默认 `Anchor::TopLeft`）。
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// 锚点取 trigger 的哪个角（默认跟随鼠标）。
    pub fn attach(mut self, attach: Anchor) -> Self {
        self.attach = Some(attach);
        self
    }

    fn with_element_state<R>(
        &mut self,
        global_id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut MenuHandleElementState<M>, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<MenuHandleElementState<M>, _>(
            Some(global_id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

/// 创建 [`RightClickMenu`]。
pub fn right_click_menu<M: ManagedView>(id: impl Into<ElementId>) -> RightClickMenu<M> {
    RightClickMenu {
        id: id.into(),
        child_builder: None,
        menu_builder: None,
        anchor: None,
        attach: None,
    }
}

/// 每个宿主元素实例的运行时状态：当前打开的菜单 + 定位点。
pub struct MenuHandleElementState<M> {
    menu: Rc<RefCell<Option<Entity<M>>>>,
    position: Rc<RefCell<Point<Pixels>>>,
}

impl<M> Clone for MenuHandleElementState<M> {
    fn clone(&self) -> Self {
        Self {
            menu: Rc::clone(&self.menu),
            position: Rc::clone(&self.position),
        }
    }
}

impl<M> Default for MenuHandleElementState<M> {
    fn default() -> Self {
        Self {
            menu: Rc::default(),
            position: Rc::default(),
        }
    }
}

pub struct RequestLayoutState {
    child_layout_id: Option<LayoutId>,
    child_element: Option<AnyElement>,
    menu_element: Option<AnyElement>,
}

pub struct PrepaintState {
    hitbox: Hitbox,
    child_bounds: Option<Bounds<Pixels>>,
}

impl<M: ManagedView> Element for RightClickMenu<M> {
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
                let mut menu_layout_id = None;

                let menu_element = element_state.menu.borrow_mut().as_mut().map(|menu| {
                    let mut anchored = anchored().snap_to_window_with_margin(px(8.));
                    if let Some(anchor) = this.anchor {
                        anchored = anchored.anchor(anchor);
                    }
                    anchored = anchored.position(*element_state.position.borrow());

                    let mut element = deferred(anchored.child(div().occlude().child(menu.clone())))
                        .with_priority(1)
                        .into_any();

                    menu_layout_id = Some(element.request_layout(window, cx));
                    element
                });

                let mut child_element = this.child_builder.take().map(|child_builder| {
                    (child_builder)(element_state.menu.borrow().is_some(), window, cx)
                });

                let child_layout_id = child_element
                    .as_mut()
                    .map(|child_element| child_element.request_layout(window, cx));

                let layout_id = window.request_layout(
                    gpui::Style::default(),
                    menu_layout_id.into_iter().chain(child_layout_id),
                    cx,
                );

                (
                    layout_id,
                    RequestLayoutState {
                        child_element,
                        child_layout_id,
                        menu_element,
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        if let Some(child) = request_layout.child_element.as_mut() {
            child.prepaint(window, cx);
        }

        if let Some(menu) = request_layout.menu_element.as_mut() {
            menu.prepaint(window, cx);
        }

        PrepaintState {
            hitbox,
            child_bounds: request_layout
                .child_layout_id
                .map(|layout_id| window.layout_bounds(layout_id)),
        }
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

                if let Some(mut menu) = request_layout.menu_element.take() {
                    menu.paint(window, cx);
                }

                let Some(builder) = this.menu_builder.take() else {
                    return;
                };

                let attach = this.attach;
                let menu = element_state.menu.clone();
                let position = element_state.position.clone();
                let child_bounds = prepaint_state.child_bounds;

                let hitbox_id = prepaint_state.hitbox.id;
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble
                        && event.button == MouseButton::Right
                        && hitbox_id.is_hovered(window)
                    {
                        cx.stop_propagation();
                        window.prevent_default();

                        let Some(new_menu) = (builder)(window, cx) else {
                            return;
                        };
                        let menu2 = menu.clone();
                        let previous_focus_handle = window.focused(cx);

                        window
                            .subscribe(&new_menu, cx, move |modal, _: &DismissEvent, window, cx| {
                                if modal.focus_handle(cx).contains_focused(window, cx)
                                    && let Some(previous_focus_handle) =
                                        previous_focus_handle.as_ref()
                                {
                                    window.focus(previous_focus_handle, cx);
                                }
                                *menu2.borrow_mut() = None;
                                window.refresh();
                            })
                            .detach();

                        // 菜单以 deferred 方式渲染，焦点树里要到下一帧才连上，
                        // 所以推迟两帧再聚焦，保证宿主焦点仍在（避免状态栏按钮闪动）。
                        let focus_handle = new_menu.focus_handle(cx);
                        window.on_next_frame(move |window, _cx| {
                            window.on_next_frame(move |window, cx| {
                                window.focus(&focus_handle, cx);
                            });
                        });
                        *menu.borrow_mut() = Some(new_menu);
                        *position.borrow_mut() = if let Some(child_bounds) = child_bounds {
                            if let Some(attach) = attach {
                                child_bounds.corner(attach)
                            } else {
                                window.mouse_position()
                            }
                        } else {
                            window.mouse_position()
                        };
                        window.refresh();
                    }
                });
            },
        )
    }
}

impl<M: ManagedView> IntoElement for RightClickMenu<M> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
