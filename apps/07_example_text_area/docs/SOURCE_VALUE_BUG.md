# 07 多行文本框：第一个框（Source::Value 路径）完全不能输入的排查记录

> 关联：本仓库 `apps/05_android_input/docs/IME_INPUT_DEBUG.md`（IME 闪退修复）
> 代码位置：`apps/07_example_text_area/src/text_area.rs`（`TextArea`）、`app_view.rs`（`MultilineExample`）

## 现象

`07_example_text_area` 在同一窗口里放了**两个** `TextArea`：

- 第一个框：`TextArea::new(self.bio, 4)` —— 走 `Source::Value` 路径
  （内部用 `window.use_state(cx, |window, cx| Editor::over(value, ...))` 懒建 `Editor`）
- 第二个框：`TextArea::editor(notes, 4)` —— 走 `Source::Editor` 路径
  （直接持有外部已建好的 `Entity<Editor>`）

**稳定的可复现现象（桌面 + Android 都一致）：**

1. 第二个框一直能输入、能换行（回车 → `Enter` action → `insert_newline`）。
2. **第一个框从来都不能输入** —— 无论点击先后、是否先点第二个框，都完全无反应。
3. 在旧版本（两个框共用 `.id("text-area")`）还叠加了焦点锁死：先点第一个框后无法切回第二个框。

## 根因

**第一个框不能输入的元凶是 `Source::Value` 这条路径本身**，与 `.id("text-area")` 碰撞无关
（碰撞只解释焦点切换问题，不解释“完全不能输入”）。

`TextArea::render` 里两条分支表面一致，都拿到 `Entity<Editor>`：

```rust
let editor = match self.source {
    Source::Value(value) => window.use_state(cx, move |window, cx| Editor::over(value, window, cx)),
    Source::Editor(editor) => editor,
};
let focus_handle = editor.read(cx).focus_handle.clone();
```

差异只在 `Editor` entity 的来源：

- `Source::Editor`：editor 是外部 entity，在 `MultilineExample::render` 里 `use_state` 建一次，
  生命周期稳定，`focus_handle` 在多次渲染间不变。
- `Source::Value`：editor 在 **`TextArea` 自己的 `render` 内部**用 `use_state` 懒建。
  该 `use_state` 以 `ElementId::CodeLocation`（源码行号）为 key，每次 `TextArea` 渲染时若状态
  失效就会重建 editor，导致 `focus_handle` 与 `EditorText::paint` 里
  `window.handle_input(&focus_handle, ElementInputHandler::new(bounds, editor))` 使用的
  `focus_handle` 不是同一个实体 —— IME / 输入路由错乱，因而完全收不到输入。

注意：zed 原版 `example_text_area.rs` 的 `TextArea` 也用 `Source::Value` + `.id("text-area")`，
但 zed 的 demo **同一窗口只渲染一个** `TextArea`，所以这条路径的坑从不暴露；本例放了两个框
才触发。

> 关于回车不显示 `⏎`：这是**预期行为**，不是 bug。07 里回车绑定 `Enter` action →
> `Editor::insert_newline`，走的是动作路径，**不经过** `replace_text_in_range`（IME 提交路径）。
> 只有普通字符（如 `x`）才会经 IME 提交，显示成 `IME->"x"`。诊断框已注明“回车走 Enter action，
> 不在此显示”。

## 修复

让两个框**完全对称**地走 `Source::Editor`，彻底不调用 `Source::Value` 这条坏路径：

`app_view.rs` 的 `MultilineExample::render` 里，两个框都用 `Editor::over_with_log(...)` 建好
`Entity<Editor>`，再传给 `TextArea::editor(...)`：

```rust
let bio_editor = window.use_state(cx, {
    let bio = self.bio.clone();
    let log = self.debug_log.clone();
    move |window, cx| Editor::over_with_log(bio, Some(log), window, cx)
});
let notes_value = cx.new(|_| "multi\nline\nsample".to_string());
let notes = window.use_state(cx, {
    let notes_value = notes_value.clone();
    let log = self.debug_log.clone();
    move |window, cx| Editor::over_with_log(notes_value, Some(log), window, cx)
});
// ...
.child(TextArea::editor(bio_editor.clone(), 4))          // 第一个框
.child(TextArea::editor(notes.clone(), 4).color(...))   // 第二个框
```

`TextArea::new` / `Source::Value` 保留为公开 API（与 zed 对齐），但本例不再使用。

## 验证

- 桌面（`bun run run` / `cargo run -p text_area_07_android`）：两个框都能输入、能换行、能来回切换焦点。
- Android（`bun run android:run`）：同逻辑经 gpui-android IME 桥接，两个框均可输入；
  软键盘回车换行需配合 `KeyboardType::MultiLine`（见 `text_area.rs` 的
  `focus_and_show_keyboard` 与 `crates/gpui-android` 的 `MultiLine` 输入类型）。

## 结论 / 教训

- **不要在 `View::render` 内部用 `use_state` 懒建“会被焦点/输入系统长期持有”的 entity**
  （尤其 `FocusHandle`、IME `EntityInputHandler`）。应在视图拥有者（`MultilineExample::render`，
  或构造期）建好 entity，再下沉给子视图，保证 `focus_handle` 在多次渲染间稳定。
- 多实例同屏时，**不要给多个视图写死同一个 `.id(...)`**（GPUI `ElementId` 同级必须唯一，否则
  `use_state`/焦点状态相撞）。本例已移除 `.id("text-area")`，改由 `entity_id()` 自动派生唯一 id。
- 移植 zed 单实例示例到“同屏多实例”场景时，要逐个复核示例里隐含的单实例假设。
