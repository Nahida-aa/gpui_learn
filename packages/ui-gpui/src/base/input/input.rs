//! 单行输入框 facade:`InputState` = [`Editor`] 的 SingleLine 模式。
//!
//! 外部 API 与旧版完全兼容(`InputState::new` builder、`InputEvent::
//! Change/Submit`、[`bind_input_keys`]),实现全部下沉到 editor 引擎:
//! 单行模式下的差异由引擎内部分流——Enter 不换行(发 [`InputEvent::
//! Submit`])、粘贴去 `\n`、纵向移动不生效。

use gpui::{App, KeyBinding, SharedString};

use super::editor::{
    Backspace, Copy, Cut, Delete, Down, Editor, End, Home, Left, Newline, Paste, Right,
    SelectAll, SelectDown, SelectLeft, SelectRight, SelectUp, ShowCharacterPalette, Up,
};

/// 单行输入框 = 多行编辑器的 SingleLine 模式。
pub type InputState = Editor;

/// 单行输入框对外事件(旧 API 兼容)。
///
/// 引擎在 SingleLine 模式下自动发出;多行请订阅
/// [`EditorEvent`](crate::base::input::editor::EditorEvent)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputEvent {
    /// 内容发生变化(程序化 `set_value` 不触发)。
    Change(SharedString),
    /// Enter 提交。
    Submit(SharedString),
}

impl gpui::EventEmitter<InputEvent> for Editor {}

/// 单行输入框的 key_context 名。
pub const INPUT_KEY_CONTEXT: &str = "ui-gpui-input";

/// 把单行输入需要的按键绑到 [`INPUT_KEY_CONTEXT`] 上。
///
/// 必须在应用启动回调里调用一次,否则光标移动/删除/粘贴等全部失效。
/// 剪贴板/全选用 `secondary` 修饰符(macOS→cmd,其余→ctrl)。
pub fn bind_input_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("delete", Delete, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("left", Left, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("right", Right, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("up", Up, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("down", Down, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("shift-up", SelectUp, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("shift-down", SelectDown, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("secondary-a", SelectAll, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("secondary-v", Paste, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("secondary-c", Copy, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("secondary-x", Cut, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("home", Home, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new("end", End, Some(INPUT_KEY_CONTEXT)),
        KeyBinding::new(
            "ctrl-cmd-space",
            ShowCharacterPalette,
            Some(INPUT_KEY_CONTEXT),
        ),
        KeyBinding::new("enter", Newline, Some(INPUT_KEY_CONTEXT)),
    ]);
}
