# Zed 的文本输入体系：没有 gpui Input 元素，只有三层结构

> 来源仓库：`/home/aa/repos/learn_ls/zed`，`main` HEAD `027cf0def7`（2026-08-07）
> 问题：zed 中有用过 input 组件（元素）吗？

## 结论

**gpui 本体不提供现成的 `Input` UI 元素，zed 应用侧也从未使用过这种组件**——因为压根不存在。zed 的文本输入由三层结构承担：

| 层 | 位置 | 角色 |
|---|---|---|
| 1. 协议层 | `crates/gpui/src/input.rs`（485 行） | `EntityInputHandler` trait——平台文本输入底层接口（IME 组合输入、选区、剪贴板、`replace_text_in_range`），不是 UI 元素 |
| 2. 引擎层 | `crates/editor` | 一切文本输入复用 `Editor`（缓冲区编辑器），搜索框、prompt、配置输入都是它 |
| 3. 表单封装层 | `crates/ui_input` | 把 `Editor` 封装成 `InputField`（placeholder、密码 mask、多行限制、read_only） |

佐证：

- `pub struct Input` 在整个 gpui crate 中**不存在**；`gpui::elements` 目录只有 `div/img/svg/list/uniform_list` 等
- `crates/ui_input/src/ui_input.rs` crate 文档注释明确说明：
  > This crate provides UI components that can be used for form-like scenarios... **It can't be located in the `ui` crate because it depends on `editor`.**
- gpui 的 `crates/gpui/examples/input.rs` 是教学示例：演示用 `EntityInputHandler` + `ElementInputHandler` + `Window::handle_input` 从零手写输入框（鼠标选区、IME、剪贴板）

## `InputField`（ui_input）使用规模

`grep -rln "InputField::" crates/*/src` 按文件数统计：

| crate | 文件数 | 场景 |
|---|---|---|
| settings_ui | 6 | 设置页表单 |
| language_models | 6 | 各 provider 的 API key / endpoint 输入 |
| workspace | 1 | — |
| keymap_editor | 1 | 按键输入（另有自建 `keystroke_input.rs`） |
| debugger_ui | 1 | `new_process_modal.rs` |
| component_preview | 1 | 组件预览 |

依赖 `ui_input` 的 crate（Cargo.toml）：component_preview、debugger_ui、editor、git_ui、keymap_editor、language_models、picker、remote_connection、settings_ui。

## `EntityInputHandler` 要点（`crates/gpui/src/input.rs`）

视图实现该 trait 后，用 `ElementInputHandler<V>` 包装，并在 paint 时调用 `Window::handle_input` 注册。核心方法：

- `text_for_range` / `selected_text_range` / `marked_text_range` / `unmark_text`——选区与 IME
- `replace_text_in_range` / `replace_and_mark_text_in_range`——文本替换与 IME 标记
- `paste`（默认实现：从 `ClipboardItem` 取文本替换）

## 对照：gpui_learn 的两个输入 app 走的哪条路

zed 的路线拆开就是两条：**底层自实现**（协议层直接用）与 **Editor 封装**（生产做法）。我们的两个 app 恰好各对应一条：

### `apps/04_input` —— 底层自实现路线（对应 `gpui/examples/input.rs`）

单文件 `main.rs`（778 行）：
- `impl EntityInputHandler for TextInput`（`main.rs:274`）
- paint 时 `window.handle_input(..., ElementInputHandler::new(bounds, self.input.clone()), ...)`（`main.rs:553-555`）
- 自定义 actions（Backspace/Delete/Left/Right/SelectAll/Paste/Cut/Copy...）+ `unicode_segmentation` 做词边界
- 自绘光标、选区高亮（PaintQuad / fill）

### `apps/06_text_area` —— 自建「Editor 引擎 + 外壳」路线（对应 zed 的 editor + ui_input 两层，但全部自实现）

- `src/editor.rs`（1051 行）：自建轻量 `Editor` 实体——光标、闪烁、焦点、键盘、文本渲染，`impl EntityInputHandler for Editor`（`editor.rs:620`），`window.handle_input` 接入
- `src/text_area.rs`（257 行）：`TextArea` 外壳——「建立在 `Editor` 之上，Enter 插入换行」，支持 `Source::Editor(Entity<Editor>)` 与 `Source::Value`（内部 `use_state` 创建 Editor）两种来源，角色类似 zed `ui_input::InputField`
- 与 zed 的差异：zed 的 `crates/editor` 是重引擎（buffer/snapshot/display map/多缓冲），我们的 `editor.rs` 是学习用的轻量复刻；zed `InputField` 走 `ERASED_EDITOR_FACTORY` 工厂 + `ErasedEditor` trait 对象，我们的 `TextArea` 直接持有 `Entity<Editor>`

## 经验

1. 想要输入框，gpui 不给现成的；小场景抄 `examples/input.rs` 自实现，成规模就学 zed 拆「引擎实体 + 表单外壳」两层
2. `ui_input` 不能放 `ui` crate 的原因（依赖 editor）提醒我们：封装层对引擎的依赖要在 crate 划分时就想清楚
3. IME/选区/剪贴板这些平台行为，全部收敛在 `EntityInputHandler` 一个 trait 里实现，UI 渲染与输入协议分离
