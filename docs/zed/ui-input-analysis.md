# zed `crates/ui_input` 分析 —— 为什么 InputField 要单独成 crate？

> 分析基于 zed rev `f6838a7c`（与我们 `[patch.crates-io]` 一致），2026-09。
> 与 [util-analysis.md](./util-analysis.md)、[path-analysis.md](./path-analysis.md)、
> [collections-analysis.md](./collections-analysis.md)、
> [settings-json-analysis.md](./settings-json-analysis.md) 同一套判据。

## 结论（且这是一个"注释骗人"的案例）

1. **crate 头注释给的理由是历史遗留，今天已不成立。** `ui_input.rs:3` 写着
   *"It can't be located in the `ui` crate because it depends on `editor`"* ——
   但**现在的 `ui_input` 根本不依赖 editor**（依赖只有 `component` / `gpui` / `ui`）。
   按今天的依赖图，它**技术上已经可以并进 `ui`**。
2. **当初确实不能。** 这个 crate 诞生时（`ui_text_field`）是直接 `editor.workspace = true`
   的，那时放进 `ui` 就会成环。理由真实过，只是后来失效了。
3. **现在剩下的全是语义理由，不是编译器强制的**：`ui` 是被上百 crate 依赖的自足控件库，
   不适合放一个"运行时没注册工厂就 panic"的组件。

## 依赖图（逐个打开 Cargo.toml 核对过）

| crate | `[dependencies]` 里的相关项 |
|---|---|
| `ui` | gpui / theme / component / ui_macros / icons … —— **无 editor** |
| `ui_input` | **component / gpui / ui** —— **无 editor** |
| `editor` | gpui / **ui** / **ui_input** |

```
        ui                 ← 共同上游，不依赖 editor
        ▲  ▲
        │  │
   ui_input                ← 只依赖 ui；定义 ErasedEditor trait + 工厂槽
        ▲  │
        │  │
      editor               ← 依赖 ui + ui_input；实现 ErasedEditor 并注册工厂
        ▲
        │
      zed(app)             ← editor::init() 把工厂装上
```

注意：把 `ui_input` 并进 `ui` **不会**产生任何环 —— `editor → ui` 这条边本来就存在，
`editor` 对 `ui_input` 的依赖会直接融进去。

## git 时间线（实证）

| 提交 | 事件 |
|---|---|
| `03d853d344` PR #10361 | 建 `ui_text_field` crate（**直接依赖 editor**） |
| `2925f3d33c` PR #13949 | 改名 `ui_input`。我 `git show 2925f3d33c:crates/ui_input/Cargo.toml` 出来的 `[dependencies]` 里**仍然有** `editor.workspace = true` |
| `c9997592e4` PR #47253 *build: Simplify build graph* | **移除 editor 依赖**，换成 `ErasedEditor` + `ERASED_EDITOR_FACTORY` 运行时注入。此后依赖只剩 component / gpui / ui |
| `8f3da5c5cd` PR #40829 | 提交信息里还在复述旧理由，但用了 **originally** 这个词：<br>*"The `ui_input` crate **originally** was meant only for the text field-like component, which couldn't be in the regular `ui` crate due to the dependency with `editor`."* |

**教训：crate 头注释/提交信息里的"为什么"会过期。判断依赖问题必须看当前
`Cargo.toml`，并追 `git log -S` 看它什么时候变的。**

## `ErasedEditor`：把编译期依赖换成运行时注入

`ui_input` 不能依赖 editor（否则 `editor → ui_input → editor` 成环），但 InputField 又
必须要一个真编辑器。解法是 45 行的 `ui_input.rs`：

```rust
// ui_input.rs:16-37（11 个方法，只暴露"输入框"需要的那点能力）
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
    fn subscribe(&self, callback: …, window: &mut Window, cx: &mut App) -> Subscription;
    fn render(&self, window: &mut Window, cx: &App) -> AnyElement;
    fn as_any(&self) -> &dyn Any;
}

// ui_input.rs:44
pub static ERASED_EDITOR_FACTORY: OnceLock<fn(&mut Window, &mut App) -> Arc<dyn ErasedEditor>> =
    OnceLock::new();
```

三处配套：

| 位置 | 作用 |
|---|---|
| `ui_input/src/input_field.rs:57-60` | 取工厂，取不到就 `.expect("ErasedEditorFactory to be initialized")` **panic** |
| `editor/src/editor.rs:1798` | `Editor::erased()` → `Arc<dyn ErasedEditor>` |
| `editor/src/editor.rs:12499` | `struct ErasedEditorImpl(Entity<Editor>)` + `impl ui_input::ErasedEditor`，把 11 个方法转发给 `Entity<Editor>` |
| `editor/src/editor.rs:416` | `editor::init()` 里 `ERASED_EDITOR_FACTORY.set(...)` 完成注册 |

**代价**：`InputField` 的构造有了隐式前置条件 —— 必须在 `editor::init()` 之后。
这是编译期检查不到的运行时契约。

## 那"拆出来"到底换来了什么？

逐条检验，只有前两条成立：

| 说法 | 是否成立 |
|---|---|
| 放进 `ui` 会成环 | **历史成立，现在不成立**（依赖已被 PR #47253 移除） |
| 让 `workspace` / `picker` 不必编译期依赖 editor | **不成立** —— 就算并进 `ui`，它们依赖 `ui` 同样不碰 editor |
| `ui` 保持"自足控件库"的语义纯净，不放带运行时前置条件的组件 | **成立（唯一硬理由）** |
| `editor → ui_input` 这条边能表达"editor 在做注入" | 成立，但弱 |

消费者的实际情况（都核过 Cargo.toml）：

- `workspace` —— 依赖 `ui_input`，**Cargo.toml 里压根没有 editor**
- `picker` —— `editor` 只在 `[dev-dependencies]`
- `git_ui` / `settings_ui` / `remote_connection` / `debugger_ui` —— 有 editor（它们本来就用编辑器）

## 一个佐证：zed 的 `ui` 里连输入框都没有

`crates/ui/src/components/` 全部 40+ 个文件里，**没有** `input.rs` / `text_field.rs` /
`textarea.rs`。zed 所有文本输入一律走 `editor`。

即：zed 宁可让 `ui` 缺一个"文本框"这种基础控件，也不肯让 `ui` 依赖 `editor`。
这条约束的强度，比 `ui_input` 这个 crate 本身更说明问题。

## 与我们 `gpui_learn` 的对比

| | zed | 我们 |
|---|---|---|
| `ui` 依赖 editor? | **否**（硬约束） | **是**（`Input` / `Textarea` facade 在 `packages/ui`） |
| `ui` 里有输入框组件? | 没有 | 有 |
| 独立的 `ui_input` 层? | 有 | 没有 |
| `editor` 依赖 `ui`? | **是** | **否**（自研 Rope 引擎，不取 ui 主题） |

两边的依赖方向正好相反：

```
zed:     ui ← ui_input ← editor     且  ui ← editor
我们:    ui → editor                 （editor 不回头）
```

**我们目前无环，但这是"editor 恰好不依赖 ui"给的，不是结构上保证的。**
一旦我们的 `editor` 开始要用 `ui` 的 `Label` / `Icon` / 主题色（比如做行号 gutter、
内联提示），`ui → editor` + `editor → ui` 立刻成环，那就是 zed 当初遇到的同一个问题。

## 决策表

| 场景 | 怎么做 |
|---|---|
| 现在 | **不动** —— 无环，且我们没有 `editor → ui` 的需求 |
| 将来 `editor` 需要用 `ui` 的东西 | 照搬 PR #47253 那手：`editor` 侧定义 trait 或把双向依赖改成 `OnceLock` 工厂注入。**不必先拆 crate** —— 先断编译期边，crate 边界可以后补 |
| 将来想要"多个编辑器实现 / 测试替身"塞进同一个输入框 | 才值得建 `packages/ui-input` 并搬 `ErasedEditor` |

## 附：本次新增的判据

前四次分析的判据是「闭包大小 / 是不是通用库 / 现在用得上吗 / 需要的部分有多少行」。
这次多两条：

1. **注释和提交信息会过期。** 判断"为什么这么分"必须 `git log -S` 追依赖变更，
   不能只读当前代码里的解释性文字。
2. **区分"编译期边"和"运行时契约"。** `ui_input` 对 editor 的依赖从编译期挪到运行时后，
   crate 边界就失去了强制性 —— 剩下的是设计意图，不是约束。
