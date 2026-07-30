//! `TextArea` —— 多行文本框外壳：建立在 `Editor` 之上，Enter 插入换行。
//!
//! （移植自 zed crates/gpui/examples/view_example/example_text_area.rs，
//! 简化掉 `editor.cached()` 以兼容本节点的 gpui rev，直接 child(editor)。）
//!
//! 本模块平台无关：`on_mouse_down` 里「点击弹软键盘」只在 Android 上调用
//! （见 `focus_and_show_keyboard` 的 cfg 分支），桌面端点击只聚焦，不弹键盘。

use gpui::{
    App, Context, CursorStyle, Entity, EntityId, Hsla, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, View, Window, div, hsla, px, white,
};
use gpui::prelude::*;

use crate::editor::{Copy, Cut, Editor, Enter, Paste, SelectAll, standard_actions};

enum Source {
    Value(Entity<String>),
    Editor(Entity<Editor>),
}

#[derive(IntoElement)]
pub struct TextArea {
    source: Source,
    rows: usize,
    color: Option<Hsla>,
}

impl TextArea {
    pub fn new(value: Entity<String>, rows: usize) -> Self {
        Self {
            source: Source::Value(value),
            rows,
            color: None,
        }
    }

    pub fn editor(editor: Entity<Editor>, rows: usize) -> Self {
        Self {
            source: Source::Editor(editor),
            rows,
            color: None,
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// 点击输入框：聚焦；Android 上同时主动弹出软键盘（Android 不会自动弹）。
fn focus_and_show_keyboard(focus_handle: &gpui::FocusHandle, window: &mut Window, cx: &mut App) {
    window.focus(focus_handle, cx);
    #[cfg(target_os = "android")]
    {
        gpui_android::android::jni::show_keyboard_android(gpui_android::KeyboardType::MultiLine);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = focus_handle;
    }
}

impl View for TextArea {
    fn entity_id(&self) -> Option<EntityId> {
        Some(match &self.source {
            Source::Value(value) => value.entity_id(),
            Source::Editor(editor) => editor.entity_id(),
        })
    }

    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let editor = match self.source {
            Source::Value(value) => {
                window.use_state(cx, move |window, cx| Editor::over(value, window, cx))
            }
            Source::Editor(editor) => editor,
        };

        let focus_handle = editor.read(cx).focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let text_color = self.color.unwrap_or(hsla(0., 0., 0.1, 1.));
        let row_height = px(24.);
        let box_height = row_height * self.rows as f32 + px(16.);

        let border = if is_focused {
            hsla(220. / 360., 0.8, 0.5, 1.)
        } else {
            hsla(0., 0., 0.75, 1.)
        };

        div()
            .key_context("TextInput")
            .track_focus(&focus_handle)
            .relative()
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                let editor_id = editor.entity_id();
                let editor = editor.clone();
                move |event: &MouseDownEvent, window, cx| {
                    log::info!("[textarea] on_mouse_down focus editor id={:?} pos={:?} click_count={}", editor_id, event.position, event.click_count);
                    focus_and_show_keyboard(&focus_handle, window, cx);
                    let offset = editor.read(cx).index_for_point(event.position);
                    editor.update(cx, |e, cx| {
                        if event.click_count >= 2 {
                            // 长按选词（gpui-android 用 click_count=2 标记长按）。
                            e.is_selecting = true;
                            e.select_word_at(offset, cx);
                        } else if event.modifiers.shift {
                            // Shift+点击：以当前选区为锚点，扩展到点击处。
                            e.is_selecting = true;
                            e.move_active_to(offset, cx);
                        } else {
                            // 普通点击：定位光标并开启拖拽选区（同一点即折叠）。
                            e.is_selecting = true;
                            e.move_to(offset, cx);
                        }
                    });
                }
            })
            .on_mouse_move({
                let editor = editor.clone();
                move |event: &MouseMoveEvent, _window, cx| {
                    // 仅拖拽中（按住左键移动）才扩展选区。
                    if !editor.read(cx).is_selecting || !event.pressed_button.is_some() {
                        return;
                    }
                    let offset = editor.read(cx).index_for_point(event.position);
                    editor.update(cx, |e, cx| e.move_active_to(offset, cx));
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let editor = editor.clone();
                move |_event: &MouseUpEvent, _window, cx| {
                    editor.update(cx, |e, _cx| e.is_selecting = false);
                }
            })
            .map(standard_actions(editor.clone()))
            // Enter 是多行框与单行输入唯一的区别：插入换行而非忽略/提交。
            .on_action({
                let editor = editor.clone();
                let editor_id = editor.entity_id();
                move |_: &Enter, _window, cx| {
                    log::info!("[textarea] Enter action -> editor id={:?}", editor_id);
                    editor.update(cx, |e, cx| e.insert_newline(cx))
                }
            })
            .w_full()
            .h(box_height)
            .p(px(8.))
            .bg(white())
            .border_1()
            .border_color(border)
            .rounded(px(4.))
            .overflow_hidden()
            .line_height(row_height)
            .text_size(px(18.))
            .text_color(text_color)
            .child(editor.clone())
            .child(selection_toolbar(editor, is_focused, window, cx))
    }
}

/// 工具条上的一个按钮：等宽文字，点击时把对应动作派发到 `editor`。
/// 按钮自带 hitbox（因挂了 `on_mouse_down`），会拦截其区域内的点击，
/// 不会穿透到底层编辑器。
fn toolbar_button(
    label: &'static str,
    editor: Entity<Editor>,
    action: impl Fn(&mut Editor, &mut Window, &mut Context<Editor>) + 'static,
) -> impl IntoElement {
    div()
        .px(px(12.))
        .h_full()
        .flex()
        .items_center()
        .cursor(CursorStyle::PointingHand)
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            editor.update(cx, |e, ecx| action(e, window, ecx));
        })
        .child(label)
}

/// 选中文字时，在选区正上方（贴顶时翻到下方）绘制一个浮动工具条，
/// 含 复制 / 剪切 / 全选 / 粘贴。这是「方式 A」——用 GPUI 自绘，位置
/// 由 `editor.selection_bounds()` 决定，紧贴选区，不依赖系统 ActionMode。
///
/// 无焦点或折叠光标时返回一个零尺寸占位 `div`，不渲染任何内容。
fn selection_toolbar(
    editor: Entity<Editor>,
    is_focused: bool,
    _window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    // 只要有非空选区就显示工具条（方式 A）。不依赖焦点：焦点状态在
    // Android 上随输入事件抖动，gating on focus 会让工具条闪烁/消失；
    // 而非空选区本身就意味着「用户正在选择」，正是该显示工具条的时机。
    let sel = editor.read(cx).selection_bounds();
    let Some(b) = sel else {
        return div();
    };
    // `b` 是窗口坐标（来自 editor 的 prepaint bounds）。工具栏用 `.absolute()`
    // 定位，相对最近的 `relative` 祖先——本 TextArea 的 div——其原点是
    // TextArea 的 padding 盒。editor 作为 child 位于 TextArea 的 padding(8px)
    // 之内，故编辑器窗口原点减去 padding 即 TextArea padding 盒的窗口原点。
    // 把 `b` 减掉这个原点，得到相对 TextArea 的坐标，工具栏才不会因
    // `overflow_hidden` 被裁到框外。
    let edit_origin = editor.read(cx).last_bounds_origin().unwrap_or_default();
    let pad = px(8.);
    let bar_h = px(36.);
    let gap = px(4.);
    let sel_top_rel = b.top() - (edit_origin.y + pad);
    let sel_left_rel = b.left() - (edit_origin.x + pad);
    let mut top = sel_top_rel - bar_h - gap;
    if top < px(0.) {
        // 选区贴顶，浮条翻转到选区下方，避免超出可视区。
        let sel_bottom_rel = b.bottom() - (edit_origin.y + pad);
        top = sel_bottom_rel + gap;
    }
    let left = sel_left_rel;

    div()
        .absolute()
        .top(top)
        .left(left)
        .flex()
        .items_center()
        .h(bar_h)
        .rounded(px(6.))
        .bg(hsla(0.0, 0.0, 0.18, 0.96))
        .border_1()
        .border_color(hsla(0.0, 0.0, 1.0, 0.25))
        .text_color(white())
        .text_size(px(14.))
        .child(toolbar_button("复制", editor.clone(), |e, window, cx| e.copy(&Copy, window, cx)))
        .child(toolbar_button("剪切", editor.clone(), |e, window, cx| e.cut(&Cut, window, cx)))
        .child(toolbar_button("全选", editor.clone(), |e, window, cx| e.select_all(&SelectAll, window, cx)))
        .child(toolbar_button("粘贴", editor.clone(), |e, window, cx| e.paste(&Paste, window, cx)))
}
