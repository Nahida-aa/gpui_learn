# TextArea / 文本输入组件在三个仓库中的分布

本文档记录 `06_text_area` 之外，整个生态中还存有哪些 textarea/input 组件，
以及它们的定位差异，方便学习者按需查阅。

---

## 1. zed `learn_ls/zed/crates/gpui/examples/`（官方示例）

| 文件 | 行数 | 说明 |
|------|------|------|
| `example_editor.rs` | 549 | 引擎实体：光标、闪烁、焦点、键盘、自定义文本渲染 |
| `example_text_area.rs` | 118 | 多行文本框：Editor 薄封装，Enter → insert_newline |
| `example_input.rs` | 121 | 单行输入框：同 TextArea，但不绑 Enter |
| `input.rs` | 778 | 独立的完整单行输入实现（full IME/selection 支持） |

**定位**：纯教学示例。`Editor` + `TextArea` + `Input` 三者配合，从零展示 GPUI
自定义 Element + EntityInputHandler。`06_text_area` 直接移植自此。

---

## 2. `gpui-toolkit`（位于 `/home/aa/repos/ide_ls/gpui-toolkit/`）

| 文件 | 行数 | 说明 |
|------|------|------|
| `crates/gpui-ui-kit/src/input.rs` | 1344 | 单行文本输入，完整键盘编辑/剪贴板/IME/主题/无障碍 |
| `crates/gpui-themes/src/theme/editor_theme.rs` | — | 编辑器相关主题定义 |

**定位**：产品级单行输入组件，有主题系统支持。**但无多行 TextArea**。
没有独立的 TextArea 组件，也没有 `multi_line` 模式。适合只想用单行输入框的场景。

---

## 3. `gpui-component`（位于 `/home/aa/repos/ide_ls/gpui-component/`）

| 文件 | 行数 | 说明 |
|------|------|------|
| `crates/ui/src/input/state.rs` | 3893 | `InputState` 实体：所有输入类型的状态管理，`.multi_line(true)` 开多行 |
| `crates/ui/src/input/element.rs` | 2878 | 核心渲染：pain/layout/scrollbar/行号/文本 shaping |
| `crates/ui/src/input/input.rs` | 662 | Input 元素绑定层，处理外观/前缀/清除/ARIA |
| `crates/ui/src/input/mode.rs` | 470 | `PlainText` / `CodeEditor` / `AutoGrow` 模式定义 |
| `crates/story/src/stories/textarea_story.rs` | 256 | TextArea story（固定行/auto-grow/no-wrap/chat 模式） |
| `crates/story/src/stories/editor_story.rs` | 57 | Code Editor story（tree-sitter 高亮） |
| `crates/story/examples/editor.rs` | 1211 | 完整编辑器示例（含 LSP/diagnostics/completions/文件树） |
| `docs/docs/components/editor.md` | 409 | 编辑器/textarea 完整文档 |

**定位**：**最完整的 TextArea + Code Editor 组件**。`InputState` 的 `multi_line(true)`
开启多行模式，`code_editor("rust")` 开启语法高亮。支持：
- 多行编辑/auto-grow/soft-wrap/行号/折叠
- 搜索替换、LSP 集成、diagnostics、completions
- 全部基于同一个 `Input` 元素（内部根据 mode 切换渲染路径）

如果你想在真实项目里用多行输入框，应参考此仓库。
如果想**学习 GPUI 自定义 Element 的原理**，看 `learn_ls/zed` 的示例（即 06 的来源）。

---

## 对比总结

| 维度 | zed 示例 | gpui-toolkit | gpui-component |
|------|----------|--------------|----------------|
| 多行 | ✅ TextArea | ❌ | ✅ multi_line |
| 代码编辑器 | ❌ | ❌ | ✅ tree-sitter |
| 单行输入 | ✅ Input | ✅ Input | ✅ Input |
| 主题系统 | ❌ | ✅ gpui-themes | ✅ theme 模块 |
| 教学价值 | ⭐⭐⭐ | ⭐⭐ | ⭐ |
| 可用性 | 示例级 | 产品级（单行） | 产品级（全功能） |
| 行数 | 118~778 | 1344 | 8000+ |
