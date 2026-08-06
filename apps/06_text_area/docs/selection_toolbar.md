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
| 现状 | **已删除**（git 历史保留作为尝试记录） | **当前启用** |

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
故方式 A 作为正式启用方案，方式 B 的代码已删除（git 历史里仍保留完整实现，
作为这次尝试的证明）。

> 方式 B 的完整代码（`packages/gpui-android/src/android/selection.rs` 的
> `SelectionHandler` / `SELECTION_COMMANDS` / `drain_selection_commands`、
> `GpuiActivity.kt` 的 `gpuiStartActionMode` 等）**已删除**，仅留存在 git 历史中
> 作为这次尝试的证明——证明我们两种方案都实做过。其专属依赖 `gpui::PlatformInputHandler::update_app`
> （仅 `selection.rs` 调用）也已一并删除。方式 A 是自绘的正式方案。

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

### 2.4 方式 B 实现要点（系统 `ActionMode`，曾尝试，现已删除）

方式 B 不走 GPUI 自绘，而是让 Android 系统弹出原生 `ActionMode` 工具条，
再把菜单点击路由回 `Editor`。以下为当时实现（代码已删除，仅作历史记录）：
代码片段中的 `SelectionHandler` / `SELECTION_COMMANDS` / `drain_selection_commands`
及 `gpuiStartActionMode` / `nativeSelectionAction` 等均已从仓库移除。

**Java 侧（`GpuiActivity.kt`）**：

```java
// 必须用 TYPE_FLOATING：NativeActivity 没有 ActionBar，单参 startActionMode
// 默认走 ActionBar 覆盖层，在 NativeActivity 上完全不可见（「从不显示」的根因）。
public void gpuiStartActionMode() {
    runOnUiThread(() -> {
        if (selectionActionMode != null) {
            selectionActionMode.invalidate();
            return;
        }
        selectionActionMode = startActionMode(
                new SelectionActionModeCallback(), ActionMode.TYPE_FLOATING);
    });
}
// 菜单项点击 → nativeSelectionAction(code) → Rust
```

菜单项 verb code（Java→Rust）：`0=Copy 1=Cut 2=Paste 3=SelectAll`，
与 Rust 侧 `SelectionVerb::from_code` 对应。

**Rust 侧（`packages/gpui-android/src/android/selection.rs`）**：

```rust
// app 在 android_main 注册 SelectionHandler（实现 copy/cut/paste/select_all）
pub trait SelectionHandler: Send + Sync {
    fn copy(&self, cx: &mut App);
    fn cut(&self, cx: &mut App);
    fn paste(&self, cx: &mut App);
    fn select_all(&self, cx: &mut App);
}

// Java 的 nativeSelectionAction 调用 enqueue_selection_command(verb)
// → 入队 SELECTION_COMMANDS（Bender 线程安全队列）
// → window.rs 每帧 on_request_frame 调 drain_selection_commands 出队
// → handler.update_app(|app| dispatch_verb(verb, app)) 派发到聚焦 editor
pub(crate) fn drain_selection_commands(input_handler: &Arc<...>) {
    let commands = std::mem::take(&mut *SELECTION_COMMANDS.lock());
    // ... 借聚焦 input_handler 的窗口上下文拿 &mut App，逐个执行
    for verb in commands {
        handler.update_app(|app| dispatch_verb(verb, app));
    }
}
```

**显示/隐藏的驱动**（`window.rs` 的 `sync_selection_action_mode`）：
每帧检测聚焦 editor 选区是否非空，状态翻转时调用 `start_action_mode` /
`finish_action_mode`（Rust→Java）。

**方式 B 的不完美（设备验证结论）**：
- `ActionMode` 在 `NativeActivity` 上用 `TYPE_FLOATING` 才能显示，但位置
  **固定在应用顶部**，无法贴着选区（QQ / 系统短信那种「选区旁边」是系统给
  原生 `TextView` 注入的，GPUI 自绘编辑器绕过了原生 TextView，系统无从定位）。
- 同样**拿不到 MIUI 的「问小爱 / 翻译」增强按钮**（原理同上）。
- 因此方式 B 作为历史尝试后已删除（git 历史保留），实际方案是方式 A。

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

### 坑 7：工具栏按钮点击冒泡到 TextArea，点「全选」后选区被折叠丢失

**现象**：点「全选」按钮，整段先被选中，但**立即丢失选中**（选区折叠成光标），
工具条也随之消失——和坑 4 的「误触剪切」表现相似，但根因不同。

**根因**：工具栏（`selection_toolbar` 返回的 `div`）是 TextArea 的 `div` 的
**子元素**。点击工具栏按钮时，按钮的 `on_mouse_down`（`toolbar_button` 里，负责
派发 `select_all`）执行后，事件**继续冒泡到外层 TextArea 自身的 `on_mouse_down`**
（`text_area.rs`），那里会用点击处坐标调 `move_to` / `select_word_at`，于是刚
`select_all` 出来的整段选区又被折叠成光标。复制/剪切按钮点击其实也会冒泡触发
`move_to`，只是复制不改选区、剪切本就要清选区，肉眼看不出。

**修复**：按钮 `on_mouse_down` 在派发动作后调用 `cx.stop_propagation()`，阻断
事件继续冒泡到外层（与 gpui `div.rs` 内部处理鼠标事件的做法一致）。所有工具栏
按钮一并受益。


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

- 方式 A（自绘）是正式启用方案，已设备验证：长按选词 → 工具条出现在选区
  **上方** → 复制 / 剪切 / 全选 / 粘贴四个按钮均正确派发到 `Editor`。
- 方式 B（系统 ActionMode）曾实现并设备验证过，但因只能落顶部、拿不到 MIUI
  增强，已删除；完整代码留在 git 历史作为尝试记录。
- 调试日志（`[toolbar] SHOW`、`[toolbar] BTN`、`[selbounds]*`、`[appview]`、
  `[textarea] RENDER` 等）仍保留在代码中，用于后续设备验证。
