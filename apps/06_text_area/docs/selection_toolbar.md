# 选中文字浮动工具条（Selection Toolbar）

本文记录 `06_text_area` 在 Android 上「选中文本后弹出操作条（复制 / 剪切 /
全选 / 粘贴）」的两种实现方案、为何最终选「方式 A（GPUI 自绘）」，以及
实现过程中踩的关键坑。**这是设备验证过的实战经验，不是理论推导。**

---

## 1. 两种方案对比

| 维度 | 方式 B：系统 `ActionMode` | 方式 A：GPUI 自绘浮动条 |
|------|---------------------------|--------------------------|
| 实现 | Rust 调 Java `startActionMode` | `TextArea::render` 里 `.absolute()` 画一个 `div` |
| 位置 | **固定在应用顶部**（ActionBar 区域） | **紧贴选区上方**（贴顶时翻到下方） |
| 坐标控制 | 系统说了算，无法贴着选区 | 完全由 `editor.selection_bounds()` 决定 |
| 依赖 | 必须 `ActionMode.TYPE_FLOATING`（NativeActivity 无 ActionBar） | 纯 GPUI，无系统依赖 |
| 按钮定制 | 受系统菜单限制 | 任意按钮、任意样式 |
| MIUI 增强（问小爱/翻译） | ❌ 拿不到（那是系统给原生 `TextView` 注入的，GPUI 自绘不触发） | ❌ 同样拿不到（原理同上） |
| 现状 | **已禁用**，保留为可切换 fallback | **当前启用** |

### 两种都尝试过，以及不完美的结果

我们**先试了方式 B（系统 ActionMode），后做了方式 A（GPUI 自绘）**，两者都
设备验证过，都不完美：

- **方式 B 的不完美**：`ActionMode` 在 `NativeActivity`（无 ActionBar）上只能
  落在**应用顶部**，无法像普通 App（QQ / 系统短信）那样浮在选区旁边。根因是
  GPUI 的编辑器是**自绘**的，不是 Android 原生 `TextView`，系统无法定位选区、
  也无法注入「问小爱 / 翻译」这类 MIUI 增强按钮。
- **方式 A 的不完美**：自绘能贴着选区，但同样是 GPUI 自绘、绕过原生 `TextView`，
  **一样拿不到系统的 MIUI 增强按钮**。另外方式 A 依赖 `prepaint` 写回的几何
  缓存来算选区坐标，且按钮点击需要通过 `editor.update` 直接派发（见坑 4）。

**结论**：两种方式都拿不到 MIUI 的系统增强，这是「GPUI 自绘编辑器」的固有
限制，不是实现疏漏。用户期望的「工具条在选区旁边」只有自绘（方式 A）能实现，
故方式 A 作为实际启用方案，方式 B 代码保留作为历史记录与可切换 fallback。

> 方式 B 的完整代码（`crates/gpui-android/src/android/selection.rs` 的
> `SelectionHandler` / `SELECTION_COMMANDS` / `drain_selection_commands`、
> `GpuiActivity.java` 的 `gpuiStartActionMode` 等）**保留未删**，作为可切换
> fallback。当前 `window.rs` 的 `sync_selection_action_mode` 已不再调用
> `start_action_mode` / `finish_action_mode`，所以系统 ActionMode 不会弹出。
> 若将来想切回，只需在那两个分支重新调用即可。

---

## 2. 方式 A 实现要点

### 2.1 选区包围盒：`Editor::selection_bounds()`

在 `editor.rs` 新增，复用 `prepaint` 写回的几何缓存：
`last_bounds` / `last_line_starts` / `last_lines` / `last_content_len`。

逐行与选区求交，返回合并后的 `Bounds<Pixels>`（**窗口坐标**）。要点：

- `prepaint` 的 `line_starts` 长度 = 行数（**末尾不另存 `content.len()`**），
  所以末行结束位置要用单独缓存的 `last_content_len`。
  **坑**：曾误判 `line_starts.len() == lines.len() + 1` 导致永远返回 `None`，
  正确判断是 `line_starts.len() == lines.len()`。
- 空文本（`content_len == 0`）的编辑器没有可选手词，`selection_bounds`
  会因「无交叉行段」返回 `None`，属正常。

### 2.2 浮动条：`selection_toolbar()`（模块级自由函数）

在 `text_area.rs`，作为 `TextArea::render` 返回的 `div` 的
`.child(selection_toolbar(...))`。

```rust
fn selection_toolbar(editor, _is_focused, _window, cx) -> impl IntoElement {
    // 只要有非空选区就显示，不依赖焦点（见坑 3）
    let Some(b) = editor.read(cx).selection_bounds() else { return div(); };
    // b 是窗口坐标 → 换算成相对 TextArea 的坐标（见坑 1）
    let edit_origin = editor.read(cx).last_bounds_origin().unwrap_or_default();
    let pad = px(8.);
    let bar_h = px(36.);
    let top = b.top() - (edit_origin.y + pad) - bar_h - gap; // 贴顶则翻到下方
    let left = b.left() - (edit_origin.x + pad);
    div()
        .absolute().top(top).left(left)
        .flex().items_center().h(bar_h)
        .bg(hsla(0.,0.,0.18,0.96))   // 深色半透明
        .child(toolbar_button("复制", ...))
        // ... 剪切 / 全选 / 粘贴
}
```

### 2.3 按钮：`toolbar_button()`

```rust
fn toolbar_button(label, editor, action) -> impl IntoElement {
    div()
        .px(px(12.)).h_full().flex().items_center()
        .cursor(CursorStyle::PointingHand)
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            editor.update(cx, |e, ecx| action(e, window, ecx));
        })
        .child(label)
}
```

按钮自带 `on_mouse_down`，点击时直接派发到 `editor.copy/cut/paste/select_all`。

---

## 3. 踩过的坑（按严重程度）

### 坑 1：`selection_bounds` 是窗口坐标，但 `.absolute()` 相对 TextArea → 被 `overflow_hidden` 裁掉

**现象**：日志里 `[toolbar] SHOW top=361px` 一直打印，但截图上工具栏区域
深色像素比例为 0（完全没画出来）。

**根因**：`b` 来自 `editor.last_bounds`（prepaint 的窗口级 bounds，top≈369），
而 `.absolute()` 定位相对**最近的 `relative` 祖先 = TextArea 的 div**，其
内容区高度只有 `box_height`（如 4 行 ≈ 112 逻辑 px）。直接把窗口坐标 361
当相对坐标用，工具栏被排到 TextArea 框外，被 `.overflow_hidden()` 裁掉。

**修复**：把 `b` 减去 `editor.last_bounds` 的原点（再减 TextArea 的 `p(px(8.))`
padding），得到相对 TextArea 的坐标。`Editor` 上新增
`last_bounds_origin() -> Option<Point<Pixels>>` 供读取。

### 坑 2：`MouseButton::Pointer` 不存在

本 gpui rev（`82aef443`）里正确的光标变体是 `CursorStyle::PointingHand`
（不是 `Pointer`）。编译期即报错，改名即可。

### 坑 3：工具栏 `is_focused` 每帧翻转 → 闪烁/消失

**现象**：`[toolbar] entered is_focused=true/false` 每帧交替，工具栏时有时无。

**根因**：`MultilineExample` 启动时 `window.focus(&ta_focus)` 聚焦的是**顶层
视图**的 focus_handle，而 `TextArea` 用的是**自己的** focus_handle。焦点状态
在两个 handle 之间抖动，gating 工具条显示 on `is_focused` 就会闪烁。

**修复**：工具条显示条件改为「**有非空选区就显示**」，不依赖焦点。语义上也
更对——用户正在选择时就该显示工具条。

### 坑 4：工具栏容器的空 `on_mouse_down` 吞掉按钮点击

**现象**：点「全选」按钮，日志却显示点到了「剪切」，且选区被清空、工具条消失。

**根因**：工具栏容器 `div` 上挂了一个空 `on_mouse_down(|_event, _window, _cx| {})`
想「拦截缝隙点击」。但 GPUI 命中测试中容器 hitbox 包住按钮，点击先命中容器
的空 handler，按钮自己的 `on_mouse_down`（真正派发动作的）收不到事件；而
点「剪切」会删除选中内容、清空选区，工具条随之消失，导致后续点击全部落空。

**修复**：**删掉容器的空 `on_mouse_down`**。按钮自身已有 `on_mouse_down` 拦截
各自区域；按钮间缝隙极小（`.px(12)` 紧贴），不影响使用。

### 坑 5：gradle `cargoBuild` 不检测 Rust 源码改动

改了 Rust 代码后 `./gradlew installDebug` 常显示 `cargoBuild UP-TO-DATE`，
`.so` 没重编。**必须** `./gradlew installDebug --rerun-tasks`（或删
`jniLibs` 目录）才强制重编。

### 坑 6：设备序列号在 `7fb0ee72` 与 `24115RA8EC` 间跳变

`adb devices` 有时显示一个、有时显示另一个，且 `get-state` 对「另一个」报
`not found`。**以 `adb devices` 当前列出的为准**，不要硬编码旧序列号。

---

## 4. 设备验证方法（MIUI / 小米 amethyst）

- **长按选词**：`adb shell input swipe X Y X Y+1 900`（原地长按 900ms，
  gpui-android 用 `click_count==2` 标记长按 → `select_word_at`）。
  注意此手势在 MIUI 上**不稳定**，偶尔被识别为滚动而失败，需重试。
- **定位输入框**：截图后用 PIL 扫描白色块确定 TextArea 逻辑坐标区间，
  再换算物理坐标点击（device scale = 3.0，物理 = 逻辑 × 3）。
- **验证按钮命中的坐标**会偏：用 `input tap X Y` 盲点常点错按钮。可靠做法是
  在 `toolbar_button` 的 `on_mouse_down` 里临时 `log::info!("[toolbar] BTN '{}'",
  label)`，点一下看日志确认命中哪个按钮，再据此校正坐标。
- **每次 `gradlew installDebug` 会关掉前台 app**，验证前务必先
  `am start -n dev.gpui.learn.text_area_06/dev.gpui.mobile.GpuiActivity`
  重新打开。

---

## 5. 当前状态

- 方式 A 已设备验证：长按选词 → 工具条出现在选区**上方** → 复制 / 剪切 /
  全选 / 粘贴四个按钮均正确派发到 `Editor`。
- 方式 B 代码保留为 fallback，未启用。
- 临时调试日志（在本文沉淀后已清理）：`[toolbar] SHOW`、`[toolbar] BTN`、
  `[selbounds]*`、`[appview]`、`[textarea] RENDER` 等。
