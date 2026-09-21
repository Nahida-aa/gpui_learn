//! # ui_input —— 需要编辑器内核的表单类输入组件
//!
//! 架构对齐 zed `crates/ui_input`。本包存在的**唯一理由**是：
//! **这里放的是「需要编辑器」的 UI 组件，而控件库 `aa_gpui_kit_ui` 不能依赖编辑器。**
//!
//! ## 依赖方向（这是理解本包的钥匙）
//!
//! ```text
//!   aa_gpui_kit_ui            ← 通用控件库：Label / Icon / Button / KeyBinding …
//!        ▲                      它不认识 editor，也不认识本包
//!        │
//!   aa_gpui_kit_ui_input      ← 本包：InputField 等表单组件
//!        ▲                      依赖 ui，但**编译期不依赖 editor**
//!        │
//!      editor                 ← 编辑器内核：依赖 ui + 本包
//!        ▲                      实现 ErasedEditor 并在启动时注册工厂
//!        │
//!       应用                   ← 调 editor::bind_input_keys() 完成注册
//! ```
//!
//! 注意 `editor` 在最上面还是最下面：它是**最下游**的包（`editor → ui_input → ui`）。
//! 而 `ui` 是最上游，谁都可以依赖它，它谁也不依赖（指我们自研包里）。
//!
//! ## 为什么不能把这些组件放进 `aa_gpui_kit_ui`？
//!
//! 因为 `InputField` 要一个真编辑器（光标、选区、IME、masked）。
//! 如果它住在 `ui` 里，`ui` 就得依赖 `editor`；而反过来 `editor` 又要用 `ui`
//! 的 `Label` / `Icon` / 主题色 —— 那就成了 `ui ↔ editor` 的循环依赖。
//!
//! **关键纠正（读 zed 源码时最容易踩的坑）**：
//! zed `crates/ui_input/src/ui_input.rs` 顶部写着
//! *"It can't be located in the `ui` crate because it depends on `editor`."*
//!
//! 这句话**今天已经不成立了**。我们查了 zed 的 git 历史：
//!
//! | 提交 | `ui_input` 的 editor 依赖 |
//! |---|---|
//! | `03d853d344` PR #10361 建 `ui_text_field` | **有** |
//! | `2925f3d33c` PR #13949 改名 `ui_input` | **仍有**（`git show` 出来 Cargo.toml 里有 `editor.workspace = true`） |
//! | `c9997592e4` PR #47253 *build: Simplify build graph* | **移除**，换成下面的运行时注入 |
//!
//! 也就是说：这条理由是**当初**的真实约束，后来依赖被拿掉了，注释却留着没改。
//! 现在的 `ui_input` 只依赖 `component / gpui / ui`，**技术上已经可以并回 `ui`**。
//!
//! 所以现在它单独成包，剩下的理由不是编译器强制的，而是**设计意图**：
//! `ui` 是被上百个 crate 依赖的自足控件库，不该放一个「运行时没注册工厂就 panic」的组件。
//!
//! ## 运行时注入：ErasedEditor + 工厂
//!
//! 本包不能 `use editor::Editor`（会成环），但 `InputField` 又必须要一个编辑器。
//! 解法是把**编译期依赖换成运行时依赖**：
//!
//! 1. 本包定义一个 trait [`ErasedEditor`]，只暴露输入框需要的那点能力；
//! 2. 本包留一个全局槽位 [`ERASED_EDITOR_FACTORY`]（`OnceLock`）；
//! 3. `editor` 包实现这个 trait（见 `editor/src/editor/…` 的 `ErasedEditorImpl`），
//!    并在 `bind_input_keys()` / `bind_editor_keys()` 里把工厂塞进槽位；
//! 4. `InputField::new()` 从槽位取工厂造编辑器。
//!
//! **代价（重要）**：`InputField` 因此有了编译期检查不到的运行时前置条件 ——
//! 必须在 `editor::bind_input_keys()` 之后才能构造，否则
//! `.expect("ErasedEditorFactory to be initialized")` 会 panic。
//! 这是用运行时契约换来的编译期解耦，别只看到好处。

use std::{
    any::Any,
    sync::{Arc, OnceLock},
};

use gpui::{AnyElement, App, FocusHandle, Subscription, Window};
pub use input_field::*;

mod input_field;

/// 被擦除类型的编辑器句柄（对齐 zed `ui_input::ErasedEditor`）。
///
/// `ui_input` 不能依赖 `editor`（会成环），于是把「我需要一个编辑器」表达成
/// 一个 trait：只列出输入框真正用到的能力，实现方（`editor` 包）在运行时注入。
pub trait ErasedEditor: 'static {
    fn text(&self, cx: &App) -> String;
    fn set_text(&self, text: &str, window: &mut Window, cx: &mut App);
    fn clear(&self, window: &mut Window, cx: &mut App);
    fn set_placeholder_text(&self, text: &str, window: &mut Window, _: &mut App);
    fn move_selection_to_end(&self, window: &mut Window, _: &mut App);
    fn select_all(&self, window: &mut Window, cx: &mut App);
    fn set_masked(&self, masked: bool, window: &mut Window, cx: &mut App);
    fn set_read_only(&self, read_only: bool, cx: &mut App);
    fn set_multiline(&self, max_lines: Option<usize>, window: &mut Window, cx: &mut App);

    fn focus_handle(&self, cx: &App) -> FocusHandle;

    fn subscribe(
        &self,
        callback: Box<dyn FnMut(ErasedEditorEvent, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut App,
    ) -> Subscription;
    fn render(&self, window: &mut Window, cx: &App) -> AnyElement;
    fn as_any(&self) -> &dyn Any;
}

/// 精简后的编辑器事件（对齐 zed `ui_input::ErasedEditorEvent`）。
///
/// 输入框只关心「内容变了」和「失焦了」两件事，其余 `EditorEvent` 变体在
/// `subscribe` 里被过滤掉。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ErasedEditorEvent {
    BufferEdited,
    Blurred,
}

/// 编辑器工厂槽位（对齐 zed `ui_input::ERASED_EDITOR_FACTORY`）。
///
/// 由 `editor` 包在启动时 `set` 一次。见本文件顶部文档的「代价」一节 ——
/// 没 set 就构造 `InputField` 会 panic。
pub static ERASED_EDITOR_FACTORY: OnceLock<fn(&mut Window, &mut App) -> Arc<dyn ErasedEditor>> =
    OnceLock::new();
