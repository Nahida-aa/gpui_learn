//! 单行输入框 facade:`InputState` = [`Editor`] 的 SingleLine 模式。
//!
//! 外部 API 与拆包前完全兼容(`InputState::new` builder、`InputEvent::
//! Change/Submit`、[`bind_input_keys`]),实现全部在 `editor` 包的内核里:
//! 单行模式下的差异由引擎内部分流——Enter 不换行、粘贴去 `\n`、纵向移动不生效。
//!
//! 内核已独立成包(`packages/editor`),本文件只是控件库的门面薄壳:
//! 类型从这里转出,`INPUT_KEY_CONTEXT` / `InputEvent` 的**定义**在内核。

use gpui::{App, KeyBinding};

use editor::editor::{
    Backspace, Copy, Cut, Delete, Down, End, Home, Left, Newline, Paste, Right, SelectAll,
    SelectDown, SelectLeft, SelectRight, SelectUp, ShowCharacterPalette, Up,
};
pub use editor::{Editor, InputEvent, INPUT_KEY_CONTEXT};

/// 单行输入框 = 多行编辑器的 SingleLine 模式。
pub type InputState = Editor;

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
