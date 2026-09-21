//! 把 `Editor` 擦除成 `ui_input::ErasedEditor`(对齐 zed `editor.rs` 的
//! `ErasedEditorImpl`)。
//!
//! ## 为什么需要这一层
//!
//! `ui_input` 里放的是「需要编辑器」的 UI 组件(如 `InputField`),但它**编译期
//! 不能依赖本包** —— 否则 `editor → ui_input → editor` 成环。于是:
//!
//! 1. `ui_input` 定义 `ErasedEditor` trait,只列输入字段真正用到的能力;
//! 2. 本包在这里实现它,把方法转发给 `Entity<Editor>`;
//! 3. [`register_erased_editor_factory`] 把工厂注册进
//!    `ui_input::ERASED_EDITOR_FACTORY`(由 `bind_editor_keys` /
//!    `bind_input_keys` 调用)。
//!
//! 于是 `ui_input` 拿到编辑器的方式从「编译期 `use editor::Editor`」变成了
//! 「运行时 `OnceLock` 取工厂」。代价见 `ui_input` 包顶部文档的「代价」一节:
//! 必须先调过一次 bind_* 才能构造输入字段,否则 panic。
//!
//! 详见 `docs/zed/ui-input-analysis.md`。

use aa_gpui_kit_ui_input::{ErasedEditor, ErasedEditorEvent};
use gpui::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, Subscription, Window,
};

use super::{Editor, EditorElement, EditorEvent};

/// 一个 `Editor` 实体的类型擦除句柄。
pub struct ErasedEditorImpl(pub Entity<Editor>);

impl ErasedEditor for ErasedEditorImpl {
    fn text(&self, cx: &App) -> String {
        self.0.read(cx).text()
    }

    fn set_text(&self, text: &str, _window: &mut Window, cx: &mut App) {
        self.0.update(cx, |this, cx| this.set_text(text, cx));
    }

    fn clear(&self, _window: &mut Window, cx: &mut App) {
        self.0.update(cx, |this, cx| this.clear(cx));
    }

    fn set_placeholder_text(&self, text: &str, _window: &mut Window, cx: &mut App) {
        self.0
            .update(cx, |this, cx| this.set_placeholder_text(text.to_string(), cx));
    }

    fn move_selection_to_end(&self, _window: &mut Window, cx: &mut App) {
        self.0
            .update(cx, |this, cx| this.move_selection_to_end(cx));
    }

    fn select_all(&self, window: &mut Window, cx: &mut App) {
        // 复用编辑器已有的 `select_all` 动作(见 actions.rs),传默认参数即可。
        self.0.update(cx, |this, cx| {
            this.select_all(&Default::default(), window, cx)
        });
    }

    fn set_masked(&self, masked: bool, _window: &mut Window, cx: &mut App) {
        self.0.update(cx, |this, cx| this.set_masked(masked, cx));
    }

    fn set_read_only(&self, read_only: bool, cx: &mut App) {
        self.0
            .update(cx, |this, cx| this.set_read_only(read_only, cx));
    }

    fn set_multiline(&self, max_lines: Option<usize>, _window: &mut Window, cx: &mut App) {
        self.0
            .update(cx, |this, cx| this.set_multiline(max_lines, cx));
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.0.read(cx).focus_handle(cx)
    }

    fn subscribe(
        &self,
        mut callback: Box<dyn FnMut(ErasedEditorEvent, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut App,
    ) -> Subscription {
        window.subscribe(&self.0, cx, move |_, event: &EditorEvent, window, cx| {
            // 输入字段只关心两个事件,其余丢弃(对齐 zed 的过滤)。
            let event = match event {
                EditorEvent::Edited => ErasedEditorEvent::BufferEdited,
                EditorEvent::Blurred => ErasedEditorEvent::Blurred,
                _ => return,
            };
            (callback)(event, window, cx);
        })
    }

    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
        EditorElement {
            editor: self.0.clone(),
        }
        .into_any_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.0
    }
}

/// 把本包的编辑器工厂注册进 `ui_input` 的槽位(**幂等**)。
///
/// 注册后 `ui_input::InputField::new()` 才可用。`bind_editor_keys` 与
/// `bind_input_keys` 都会调它,所以正常启动流程下不会漏。
pub fn register_erased_editor_factory() {
    // `_ =` 忽略重复设置:OnceLock::set 已设置过会返回 Err,不是错误。
    let _ = aa_gpui_kit_ui_input::ERASED_EDITOR_FACTORY.set(|window, cx| {
        cx.new(|cx| Editor::single_line(cx))
            .update(cx, |editor, cx| {
                let _ = window;
                editor.erased(cx)
            })
    });
}
