//! 输入控件 facade(单行 `InputState` / 多行 `Textarea`)。
//!
//! 2026-09 从 `packages/ui/src/base/input` 搬来。搬的原因不是"换个地方放",
//! 而是**翻转依赖方向**:
//!
//! ```text
//! 旧:  aa_gpui_kit_ui → editor      （ui 认识编辑器内核）
//! 新:  editor → aa_gpui_kit_ui_input → aa_gpui_kit_ui
//! ```
//!
//! 翻转后 `ui` 是纯粹的上游控件库,谁都可以依赖它、它谁也不依赖(自研包里);
//! 而"需要编辑器的输入组件"分两处:
//! - 想直接用内核的(本文件的类型别名)→ 依赖本包;
//! - 想要表单壳(标签/占位符/masked/校验)的 → 用 `ui_input::InputField`,
//!   它经 [`crate::register_erased_editor_factory`] 运行时拿到本包的编辑器。
//!
//! 详见 `docs/zed/ui-input-analysis.md`。
//!
//! 这几个名字本质是 `Editor` 的别名:单行 = [`Editor::single_line`],
//! 多行 = `EditorMode::MultiLine` / `AutoHeight`。单行模式下的行为差异
//! (Enter 不换行、粘贴去 `\n`、纵向移动不生效)由引擎内部分流。

use gpui::{App, KeyBinding};

use super::{
    Backspace, Copy, Cut, Delete, Down, Editor, End, Home, Left, Newline, Paste, Right, SelectAll,
    SelectDown, SelectLeft, SelectRight, SelectUp, ShowCharacterPalette, Up, INPUT_KEY_CONTEXT,
};

/// 单行输入框 = 多行编辑器的 SingleLine 模式。
pub type InputState = Editor;

/// 多行文本域(状态侧;与 [`Textarea`] 同义,保留旧名)。
pub type TextareaState = Editor;

/// 多行文本域 = `Editor` 的多行模式。
///
/// 对齐 gpui-component 的 facade 形态:状态/渲染分离已在 editor 层完成,
/// facade 无需再包一层。Enter 换行、Shift 组合、垂直滚动、undo 全部由引擎提供。
pub type Textarea = Editor;

/// 把单行输入需要的按键绑到 [`INPUT_KEY_CONTEXT`] 上。
///
/// 必须在应用启动回调里调用一次,否则光标移动/删除/粘贴等全部失效。
/// 剪贴板/全选用 `secondary` 修饰符(macOS→cmd,其余→ctrl)。
///
/// 顺带会注册 `ui_input` 的编辑器工厂(见 [`super::register_erased_editor_factory`]),
/// 所以调过它之后 `ui_input::InputField` 也可以构造了。
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

    super::register_erased_editor_factory();
}
