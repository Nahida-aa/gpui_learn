# Zed 产品中的文本输入框调研

记录 Zed 中两个真实产品的文本输入框实现方式，对比 `06_text_area` 的教学版异同。

---

## 1. `agent_ui` 消息输入框

**位置**：`crates/agent_ui/src/message_editor.rs`
**组件**：`crates/editor` 的全功能 `Editor`，包裹在 `MessageEditor` struct 中

### 核心配置

```rust
Editor::new(
    EditorMode::AutoHeight { min_lines: 1, max_lines: Some(N) },  // 1 ~ N 行自适应
    buffer, None, window, cx,
);
editor.set_soft_wrap();
editor.set_use_modal_editing(true);
editor.set_show_indent_guides(false, cx);
editor.set_placeholder_text(&placeholder, window, cx);
// buffer 语言设为 Markdown → 输入语法高亮
```

### 键盘模式

通过 key context 链 `AcpThread > MessageEditor > Editor` 实现：

| 模式 | Enter | Shift+Enter | Ctrl+Enter |
|------|-------|-------------|------------|
| 默认 | → `agent::Chat`（发送） | → `editor::Newline`（换行） | → `ChatWithFollow` |
| `use_modifier_to_send` | → `editor::Newline`（换行） | 同上 | → `agent::Chat`（发送） |

### 亮点

- @mention / 文件嵌入：通过 Editor 的 **crease** 系统嵌入交互式 inline widget
- 粘贴智能：文件路径→mention，代码→crease，图片→上下文
- 草稿持久化：debounce 保存到 thread store
- 发送按钮三态：loading / generating（stop）/ 发送

---

## 2. `git_ui` 提交信息输入框

**位置**：`crates/git_ui/src/git_panel.rs`（工厂函数 `commit_message_editor`）
**组件**：同样是 `crates/editor` 的 `Editor`，但配置更精简

### 核心配置

```rust
Editor::new(
    EditorMode::AutoHeight {
        min_lines: max_lines,   // 面板 6 行，模态框 18 行
        max_lines: Some(max_lines),
    },
    buffer, None, window, cx,
);
editor.set_show_gutter(false, cx);
editor.set_use_modal_editing(true);
editor.set_show_wrap_guides(false, cx);
editor.set_show_indent_guides(false, cx);
editor.set_use_autoclose(false);
editor.set_placeholder_text(&placeholder, window, cx);
// buffer 语言设为 "Git Commit" → subject/trailer 语法高亮
```

### 键盘模式

```
Enter              → editor::Newline              ← 永远换行
Ctrl+Enter         → git::Commit                  ← 提交
Ctrl+Shift+Enter   → git::Amend                   ← 修正提交
Alt+L              → git::GenerateCommitMessage   ← AI 生成
Tab / Escape       → git_panel::FocusChanges      ← 切回文件列表
```

**Enter 永远换行**（commit 要写 subject + body），Ctrl+Enter 才提交。

### 亮点

- 面板 + 模态框共用同一 `Buffer`，改动自动同步
- 草稿持久化：`BufferEvent::Edited` → 序列化 → 重开恢复
- 标题长度警告：首行超 `commit_title_max_length` → 边框变色
- 独立字号：`git_commit_buffer_font_size` 独立于代码编辑器

---

## 3. `terminal_view` 终端模拟器

**位置**：`crates/terminal_view/src/terminal_view.rs` + `terminal_element.rs`
**组件**：**完全不使用 `Editor`**，而是自绘字符网格的 `TerminalElement`（自定义 GPUI `Element`）

### 架构：键盘直通 PTY

```
Key Press
  → TerminalView::key_down()
    → TerminalView::process_keystroke()
      → terminal::Terminal::try_keystroke()   ← 按键 → 字节序列
        → 写入 PTY 文件描述符                   ← 原始字节流
          → Shell 回应 → 解析转义序列 → 更新字符网格
            → TerminalElement::prepaint() + paint()  ← 自绘网格
```

### 渲染：`TerminalElement`（自定义 Element）

```rust
pub struct TerminalElement {
    terminal: Entity<Terminal>,
    // ...
}
```

- 从 `Terminal` 实体读取字符网格
- 相邻同风格 cell 合并为 `BatchedTextRun` 批量 shaping
- 自绘背景、光标、选区、块元素、IME 预编辑文字
- 逐像素对齐网格（`cell_width × line_height`）

### IME：`TerminalInputHandler`

终端自己实现 `InputHandler` trait（不是 `EntityInputHandler`），不走 `Editor`：
- `commit_text` → 直接将合成文字写入 PTY
- `set_marked_text` → 缓存预编辑文字供渲染

### `Editor` 的导入只用于标签重命名

`terminal_view.rs` 确实 `use editor::Editor`，但**仅用于标签重命名**：

```rust
// 文件第 156 行
rename_editor: Option<Entity<Editor>>,

// 第 480 行：创建临时单行编辑器作为覆盖层
let rename_editor = cx.new(|cx| Editor::single_line(window, cx));
```

终端的**所有实际输入**（按键 → 字节流 → PTY → 输出渲染）与 `Editor` 完全无关。除此之外，`EditorSettings` 的导入只用来读滚动条配置。

---

## 4. 对比总结

| 维度 | `06_text_area` | `agent_ui` 消息框 | `git_ui` 提交框 | `terminal_view` 终端 |
|------|:---:|:---:|:---:|:---:|
| 底层组件 | 自绘 `Editor` + `EditorText` | `crates/editor` | `crates/editor` | 自绘 `TerminalElement` |
| 输入模型 | `EntityInputHandler` | Editor 内置 | Editor 内置 | `InputHandler`（直通 PTY） |
| 多行模式 | 自己算行高 | `AutoHeight` | `AutoHeight` | 网格固有 |
| 软换行 | ❌ | ✅ `set_soft_wrap()` | ✅ 默认换行 | 终端转义控制 |
| 占位文字 | 自己绘 "Type here..." | `set_placeholder_text()` | `set_placeholder_text()` | ❌ |
| 语言高亮 | ❌ | Markdown | Git Commit | 终端 ANSI |
| @mention | ❌ | ✅ crease | ❌ | ❌ |
| 行数 | 589（教学） | 数千行 | 数千行 | 数千行 |
| 用途 | 学习 GPUI 原理 | 产品级聊天输入 | 产品级 commit 输入 | 终端模拟器 |

### 关键结论

**Zed 的文本输入有三种实现路线：**

1. **`crates/editor` 复用**（`agent_ui`、`git_ui`、搜索框、重命名……）
   - 通过 `EditorMode` + 配置项切换形态
   - `SingleLine` → 单行 / `AutoHeight` → 自适应 / `Full` → 全尺寸
   - **没有"专为 X 写的 TextArea"——全是同一个 Editor**

2. **自绘 Element**（`terminal_view`、`06_text_area`）
   - 当需要完全控制渲染、输入不经过常规文本编辑时使用
   - 终端：字符网格 + 键盘直通 PTY
   - 06：`split('\n')` 逐行 shape + `EntityInputHandler`

3. **自绘 + 语言高亮/代码编辑**（`gpui-component` 的 `InputState`）
   - 产品级多行输入 + tree-sitter 语法高亮
   - 见 `TEXTAREA_LANDSCAPE.md`
