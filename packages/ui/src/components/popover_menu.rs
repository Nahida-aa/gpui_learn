//! popover_menu：点击触发的锚定浮层菜单，对齐 zed
//! `crates/ui/src/components/popover_menu.rs`。
//!
//! 与 [`RightClickMenu`](super::right_click_menu) 同一套浮层机制
//! （`anchored()` + `deferred(priority=1)` + `DismissEvent` + 焦点归还），
//! 差异在触发与定位：
//!
//! - **触发**：trigger 元素（[`PopoverTrigger`]，即任何 impl
//!   `Clickable + Toggleable` 的元素，如 [`IconButton`](crate::IconButton)）
//!   的 `on_click` 打开/关闭；打开状态经 `Toggleable::toggle_state` 反映到
//!   trigger 外观；
//! - **定位**：锚定到 trigger 元素的角（[`PopoverMenu::attach`]），而非鼠标位置；
//! - **可编程**：[`PopoverMenuHandle`] 允许在任意代码里 show / hide / toggle
//!   （比如工具栏按钮与键盘快捷键共用同一个菜单）。
//!
//! 与 zed 的差异：未实现 `full_width` 与 `trigger_with_tooltip`
//! （后者依赖 `ButtonCommon::tooltip` 挂载点，我们尚未有该 trait）。
//!
//! 用法（与 zed 同构）：
//! ```ignore
//! let handle = PopoverMenuHandle::default();
//! PopoverMenu::new("toolbar-menu")
//!     .menu(|window, cx| Some(ContextMenu::build(window, cx, |menu, _, _| menu)))
//!     .with_handle(handle.clone())
//!     .anchor(Anchor::BottomLeft)
//!     .trigger(IconButton::new("trigger", IconName::Ellipsis))
//! ```

use std::{cell::RefCell, rc::Rc};

use gpui::{
    Anchor, AnyElement, App, Bounds, DismissEvent, DispatchPhase, Element, ElementId, Entity,
    Focusable as _, GlobalElementId, HitboxBehavior, HitboxId, InteractiveElement, IntoElement,
    LayoutId, ManagedView, MouseDownEvent, ParentElement, Pixels, Point, Style, Window, anchored,
    deferred, div, point, prelude::FluentBuilder, px, rems,
};

use crate::traits::{Clickable, Toggleable};

/// 任何能当 popover 触发器的元素：可点击（挂开/关回调）、可切换
/// （打开态反映到外观）。对齐 zed `PopoverTrigger`；
/// [`IconButton`](crate::IconButton) 已自动满足（它 impl 了
/// `Clickable` 与 `Toggleable`）。
pub trait PopoverTrigger: IntoElement + Clickable + Toggleable + 'static {}

impl<T: IntoElement + Clickable + Toggleable + 'static> PopoverTrigger for T {}

type MenuBuilder<M> = Rc<dyn Fn(&mut Window, &mut App) -> Option<Entity<M>> + 'static>;

/// 编程式控制一个 [`PopoverMenu`] 的开合。
///
/// 把它 `clone` 一份交给 PopoverMenu::with_handle 之后，任何拿到 handle 的
/// 代码都可以 show / hide / toggle（如键盘快捷键与按钮共用同一菜单）。
/// 对齐 zed `PopoverMenuHandle`。
pub struct PopoverMenuHandle<M>(Rc<RefCell<Option<PopoverMenuHandleState<M>>>>);

/// handle 背后的运行时状态；由 PopoverMenu 元素在 request_layout 时填充。
struct PopoverMenuHandleState<M> {
    menu_builder: MenuBuilder<M>,
    menu: Rc<RefCell<Option<Entity<M>>>>,
}

impl<M> Clone for PopoverMenuHandle<M> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<M> Default for PopoverMenuHandle<M> {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }
}

fn show_menu<M: ManagedView>(
    builder: &MenuBuilder<M>,
    menu: &Rc<RefCell<Option<Entity<M>>>>,
    window: &mut Window,
    cx: &mut App,
) {
    let previous_focus_handle = window.focused(cx);
    let Some(new_menu) = (builder)(window, cx) else {
        return;
    };
    let menu2 = menu.clone();

    window
        .subscribe(&new_menu, cx, move |modal, _: &DismissEvent, window, cx| {
            if modal.focus_handle(cx).contains_focused(window, cx)
                && let Some(previous_focus_handle) = previous_focus_handle.as_ref()
            {
                window.focus(previous_focus_handle, cx);
            }
            *menu2.borrow_mut() = None;
            window.refresh();
        })
        .detach();

    // 菜单以 deferred 方式渲染，焦点树里要到 deferred 回调跑完才连上。
    // 推迟两帧再聚焦，保证宿主焦点行为正确（避免触发按钮的选中态闪动）。
    let focus_handle = new_menu.focus_handle(cx);
    window.on_next_frame(move |window, _cx| {
        window.on_next_frame(move |window, cx| {
            window.focus(&focus_handle, cx);
        });
    });
    *menu.borrow_mut() = Some(new_menu);
    window.refresh();
}

impl<M: ManagedView> PopoverMenuHandle<M> {
    /// 打开菜单（已打开则不动）。
    pub fn show(&self, window: &mut Window, cx: &mut App) {
        if let Some(state) = self.0.borrow().as_ref()
            && state.menu.borrow().as_ref().is_none()
        {
            show_menu(&state.menu_builder, &state.menu, window, cx);
        }
    }

    /// 关闭菜单（未打开则不动）。
    pub fn hide(&self, cx: &mut App) {
        if let Some(state) = self.0.borrow().as_ref()
            && let Some(menu) = state.menu.borrow().as_ref()
        {
            menu.update(cx, |_, cx| cx.emit(DismissEvent));
        }
    }

    /// 已开则关、未开则开。
    pub fn toggle(&self, window: &mut Window, cx: &mut App) {
        if self.is_deployed() {
            self.hide(cx);
        } else {
            self.show(window, cx);
        }
    }

    /// 菜单当前是否已打开。
    pub fn is_deployed(&self) -> bool {
        self.0
            .borrow()
            .as_ref()
            .is_some_and(|state| state.menu.borrow().as_ref().is_some())
    }

    /// 菜单当前是否持有焦点。
    pub fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.0.borrow().as_ref().is_some_and(|state| {
            state
                .menu
                .borrow()
                .as_ref()
                .is_some_and(|model| model.focus_handle(cx).is_focused(window))
        })
    }
}

/// 点击触发的锚定浮层菜单（对齐 zed `PopoverMenu`）。
pub struct PopoverMenu<M: ManagedView> {
    id: ElementId,
    /// trigger 构建闭包：拿到 (menu 共享状态, menu_builder)，对 trigger 挂
    /// on_click / toggle_state 后产出元素（由 [`PopoverMenu::trigger`] 生成）。
    child_builder: Option<
        Box<dyn FnOnce(Rc<RefCell<Option<Entity<M>>>>, Option<MenuBuilder<M>>) -> AnyElement>,
    >,
    menu_builder: Option<MenuBuilder<M>>,
    /// 菜单的哪一角对齐锚点（默认 `Anchor::TopLeft`，即菜单从锚点右下方展开）。
    anchor: Anchor,
    /// 锚点取 trigger 的哪个角（默认由 anchor 垂直反转推出）。
    attach: Option<Anchor>,
    /// 菜单内容的额外偏移（默认水平 5px，按 anchor 方向）。
    offset: Option<Point<Pixels>>,
    /// 编程式控制句柄（request_layout 时注入运行时状态）。
    trigger_handle: Option<PopoverMenuHandle<M>>,
    /// 菜单打开时的回调。
    on_open: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl<M: ManagedView> PopoverMenu<M> {
    /// 新建（id 在同一父容器内需唯一）。
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            child_builder: None,
            menu_builder: None,
            anchor: Anchor::TopLeft,
            attach: None,
            offset: None,
            trigger_handle: None,
            on_open: None,
        }
    }

    /// 设定菜单创建器（`Entity<M>` 需实现 `ManagedView`，如
    /// [`ContextMenu`](crate::ContextMenu)）。
    pub fn menu(mut self, f: impl Fn(&mut Window, &mut App) -> Entity<M> + 'static) -> Self {
        self.menu_builder = Some(Rc::new(move |window, cx| Some(f(window, cx))));
        self
    }

    /// 绑定编程式控制句柄。
    pub fn with_handle(mut self, handle: PopoverMenuHandle<M>) -> Self {
        self.trigger_handle = Some(handle);
        self
    }

    /// 设定触发元素（任何 impl `Clickable + Toggleable` 的组件；
    /// 打开/关闭回调与本组件的开关状态自动挂上去）。
    pub fn trigger<T: PopoverTrigger>(mut self, t: T) -> Self {
        let on_open = self.on_open.clone();
        self.child_builder = Some(Box::new(move |menu, builder| {
            let open = menu.borrow().is_some();
            t.toggle_state(open)
                .when_some(builder, |el, builder| {
                    el.on_click(move |_event, window, cx| {
                        if menu.borrow().as_ref().is_some() {
                            // 已打开：本次点击交给 paint 阶段的关闭监听处理
                            // （那里会 emit Dismiss），这里不再重复打开。
                        } else {
                            show_menu(&builder, &menu, window, cx);
                            if let Some(on_open) = on_open.as_ref() {
                                on_open(window, cx);
                            }
                        }
                    })
                })
                .into_any_element()
        }));
        self
    }

    /// 菜单的哪一角锚定到锚点（默认 `Anchor::TopLeft`）。
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// 锚点取 trigger 的哪个角（默认由 anchor 垂直反转推出）。
    pub fn attach(mut self, attach: Anchor) -> Self {
        self.attach = Some(attach);
        self
    }

    /// 菜单内容的额外偏移。
    pub fn offset(mut self, offset: Point<Pixels>) -> Self {
        self.offset = Some(offset);
        self
    }

    /// 菜单打开时回调。
    pub fn on_open(
        mut self,
        on_open: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(on_open));
        self
    }

    /// attach 的解析：未显式指定时取 anchor 的垂直反转
    /// （菜单在下方展开 ↔ 锚点取 trigger 顶角，反之亦然）。
    fn resolved_attach(&self) -> Anchor {
        self.attach.unwrap_or(match self.anchor {
            Anchor::TopLeft => Anchor::BottomLeft,
            Anchor::TopCenter => Anchor::BottomCenter,
            Anchor::TopRight => Anchor::BottomRight,
            Anchor::BottomLeft => Anchor::TopLeft,
            Anchor::BottomCenter => Anchor::TopCenter,
            Anchor::BottomRight => Anchor::TopRight,
            Anchor::LeftCenter => Anchor::LeftCenter,
            Anchor::RightCenter => Anchor::RightCenter,
        })
    }

    /// 偏移的解析：默认 5px（按 rem 缩放：4px 内边距 + 1px 边框），按 anchor 水平方向。
    fn resolved_offset(&self, window: &mut Window) -> Point<Pixels> {
        self.offset.unwrap_or_else(|| {
            let offset = rems(5.) * window.rem_size();
            match self.anchor {
                Anchor::TopRight | Anchor::BottomRight | Anchor::RightCenter => {
                    point(-offset, px(0.))
                }
                Anchor::TopLeft | Anchor::BottomLeft | Anchor::LeftCenter => point(offset, px(0.)),
                Anchor::TopCenter | Anchor::BottomCenter => point(px(0.), px(0.)),
            }
        })
    }
}

/// 每个宿主元素实例的持久状态（`window.with_element_state` 托管）。
struct PopoverMenuElementState<M> {
    menu: Rc<RefCell<Option<Entity<M>>>>,
    child_bounds: Option<Bounds<Pixels>>,
}

// 手写 Default：derive 会给 M 添上不必要的 `M: Default` 约束
// （Rc<RefCell<Option<Entity<M>>>> 的默认值与 M 无关）。
impl<M> Default for PopoverMenuElementState<M> {
    fn default() -> Self {
        Self {
            menu: Rc::default(),
            child_bounds: None,
        }
    }
}

pub struct RequestLayoutState<M> {
    child_layout_id: Option<LayoutId>,
    child_element: Option<AnyElement>,
    menu_element: Option<AnyElement>,
    menu_handle: Rc<RefCell<Option<Entity<M>>>>,
    /// 触发器构建闭包在 paint 阶段才消费（要拿 child hitbox 与状态）。
    _marker: std::marker::PhantomData<M>,
}

pub struct PrepaintState {
    hitbox: Option<HitboxId>,
}

impl<M: ManagedView> Element for PopoverMenu<M> {
    type RequestLayoutState = RequestLayoutState<M>;
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
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state(
            id.unwrap(),
            |element_state: Option<PopoverMenuElementState<M>>, window| {
                let element_state = element_state.unwrap_or_default();
                let mut menu_layout_id = None;

                // 菜单已打开：anchored + deferred 渲染为顶层浮层，
                // 锚定到 trigger 的角 + 偏移。
                let menu_element = element_state.menu.borrow_mut().as_mut().map(|menu| {
                    let offset = self.resolved_offset(window);
                    let mut anchored = anchored()
                        .snap_to_window_with_margin(px(8.))
                        .anchor(self.anchor)
                        .offset(offset);
                    if let Some(child_bounds) = element_state.child_bounds {
                        anchored =
                            anchored.position(child_bounds.corner(self.resolved_attach()) + offset);
                    }
                    let mut element = deferred(anchored.child(div().occlude().child(menu.clone())))
                        .with_priority(1)
                        .into_any();

                    menu_layout_id = Some(element.request_layout(window, cx));
                    element
                });

                // trigger：闭包接收 (menu 状态, builder)，自行挂 on_click /
                // toggle_state（见 trigger 方法）。
                let mut child_element = self
                    .child_builder
                    .take()
                    .map(|child_builder| {
                        (child_builder)(
                            element_state.menu.clone(),
                            self.menu_builder.clone(),
                        )
                    });

                // 把运行时状态注入 handle（外部代码由此获得 show/hide 能力）。
                if let Some(trigger_handle) = self.trigger_handle.take()
                    && let Some(menu_builder) = self.menu_builder.clone()
                {
                    *trigger_handle.0.borrow_mut() = Some(PopoverMenuHandleState {
                        menu_builder,
                        menu: element_state.menu.clone(),
                    });
                }

                let child_layout_id = child_element
                    .as_mut()
                    .map(|child_element| child_element.request_layout(window, cx));

                // zed 有 full_width 选项时才把宽度设为 relative(1.)；我们
                // 未提供该选项，trigger 容器按内容自适应。
                let layout_id = window.request_layout(
                    Style::default(),
                    menu_layout_id.into_iter().chain(child_layout_id),
                    cx,
                );

                (
                    (
                        layout_id,
                        RequestLayoutState {
                            child_element,
                            child_layout_id,
                            menu_element,
                            menu_handle: element_state.menu.clone(),
                            _marker: std::marker::PhantomData,
                        },
                    ),
                    element_state,
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(child) = request_layout.child_element.as_mut() {
            child.prepaint(window, cx);
        }

        if let Some(menu) = request_layout.menu_element.as_mut() {
            menu.prepaint(window, cx);
        }

        // 记录 trigger 的 bounds（菜单锚定要用），并取其 hitbox
        // （"点击 trigger 关闭菜单"要用）。
        let hitbox = request_layout
            .child_layout_id
            .map(|layout_id| {
                let bounds = window.layout_bounds(layout_id);
                window.with_element_state(id.unwrap(), |element_state, _cx| {
                    let mut element_state: PopoverMenuElementState<M> =
                        element_state.unwrap_or_default();
                    element_state.child_bounds = Some(bounds);
                    ((), element_state)
                });
                window.insert_hitbox(bounds, HitboxBehavior::Normal).id
            });

        PrepaintState {
            hitbox,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _prepaint_state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(mut child) = request_layout.child_element.take() {
            child.paint(window, cx);
        }

        if let Some(mut menu) = request_layout.menu_element.take() {
            menu.paint(window, cx);

            // 菜单打开时：鼠标按下命中 trigger 即关闭菜单（emit Dismiss），
            // 这样"再次点击触发按钮 = 关闭"，而不会关了又立刻重开。
            if let Some(child_hitbox) = _prepaint_state.hitbox {
                let menu_handle = request_layout.menu_handle.clone();
                window.on_mouse_event(move |_: &MouseDownEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble && child_hitbox.is_hovered(window) {
                        if let Some(menu) = menu_handle.borrow().as_ref() {
                            menu.update(cx, |_, cx| {
                                cx.emit(DismissEvent);
                            });
                        }
                        cx.stop_propagation();
                    }
                });
            }
        }
    }
}

impl<M: ManagedView> IntoElement for PopoverMenu<M> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IconButton;
    use gpui::TestAppContext;

    /// 冒烟：builder 链完整走一遍 + handle 在菜单未注入前是“未部署”态。
    /// 真实的开合交互依赖窗口与点击事件，由 demo 手动验证。
    #[gpui::test]
    fn popover_menu_builds_and_handle_defaults(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let handle: PopoverMenuHandle<crate::ContextMenu> = PopoverMenuHandle::default();
            assert!(!handle.is_deployed(), "未注入元素状态前不应部署");

            let popover = PopoverMenu::new("test-popover")
                .menu(|window, cx| {
                    crate::ContextMenu::build(cx, |menu, _cx| menu)
                })
                .with_handle(handle.clone())
                .anchor(Anchor::BottomLeft)
                .trigger(IconButton::new("trigger", crate::IconName::Ellipsis));

            // builder 链产出元素不 panic；handle 仍指向未部署状态
            // （元素状态在 request_layout 时才注入）。
            let _ = popover.into_any_element();
            assert!(!handle.is_deployed());
        });
    }
}
