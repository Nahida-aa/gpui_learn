# 安卓端 Input 设计（gpui-android）

本文档讲清楚 `gpui-android` 后端在安卓上是怎么把**键盘输入**送进 GPUI 的。
目标读者：想在真机上做文本输入 / 多行编辑的 gpui 学习者。读完后你应该能回答：

- 软键盘打字（字母、回车、退格）分别走了哪条通道？
- 为什么"下方列表"有时有 `enter`、有时没有？
- 我们额外加的 `nativeKeyEvent` 是干什么的，为什么必须线程安全地入队？
- 为什么说 **`enter` 不等于换行**，换行只是编辑器对 `enter` 的响应？

---

## 0. 两个并行的输入通道

安卓上 GPUI 收到键盘输入有**两条相互独立**的通道，最终都汇聚到 GPUI 的
`Keystroke` / `TextInput` 机制，但走法完全不同：

| 通道 | 来源 | 触发条件 | 进入 GPUI 的方式 |
|------|------|----------|------------------|
| **IME 文本通道** | 软键盘 `InputConnection` | 打字、 composition、删除、回车(部分输入法) | `nativeCommitText` 等 → `ImeEvent` 队列 → `replace_text_in_range` |
| **KeyEvent 通道** | 硬件键 / IME `sendKeyEvent` | 物理键盘、DPAD、以及"把回车当硬件键下发"的输入法 | `AndroidKeyEvent` → `handle_key_event` → `on_key_event` → `Keystroke` |

要点：**字母通常走 IME 文本通道；回车/退格可能走任意一条，取决于输入法与
`inputType`**。我们后面加的 `nativeKeyEvent` 就是把"走错通道"的回车/退格重新
导回 KeyEvent 通道，让它能够进 keystroke 列表、并被组件当按键处理。

---

## 1. IME 文本通道（本来就有的）

GPUI 文本输入的核心抽象是 `EntityInputHandler` / `ElementInputHandler` 的
`replace_text_in_range`、`selected_text_range`、`marked_text_range` 等方法。
安卓端由 `packages/gpui-android` 实现这套接口，软键盘通过 Android 的
`InputConnection` 协议调用它。

`GpuiActivity.kt` 的 `GpuiInputView.onCreateInputConnection` 返回一个
`BaseInputConnection`，重写了以下方法（**这些是 gpui-android 原本就有的桥接**）：

- `commitText(text, ...)` → `nativeCommitText(text)` → `ImeEvent::Commit`
  → `handler.replace_text_in_range(None, &text)`。
  **字母、空格、以及粘贴/输入法直接提交的多行文本（含 `\n`）都走这里。**
- `setComposingText` / `finishComposingText` → `nativeSetComposingText` /
  `nativeFinishComposingText` → `ImeEvent::SetComposing` / `FinishComposing`
  → 控制"拼音候选"这类组合中文字符的标记范围（`marked_range`）。
- `deleteSurroundingText(before, after)` → `nativeDeleteSurroundingText(...)`
  → `ImeEvent::DeleteSurrounding` → `replace_text_in_range(Some(range), "")`。
  **多数输入法的退格走这里**，而不是 `sendKeyEvent(KEYCODE_DEL)`。
- `performEditorAction(editorAction)` → 单行输入（`TYPE_CLASS_TEXT`，不带
  `MULTI_LINE`）下，输入法把回车当成"完成/下一个/发送"等 IME action 时触发。

这些 `nativeXxx` 方法都是 **JNI 函数**，只做一件事：把数据塞进 Rust 侧的
线程安全队列（`IME_EVENTS`），然后返回。`run_event_loop` 每帧在主线程的
`on_request_frame` 里 `drain_ime_events()`，再把事件交给当前 focused 的
`input_handler`。**它们从不在 Java/IEM Binder 线程触碰 GPUI 窗口状态**，所以天然线程安全。

> 关键性质：**IME 文本通道只改文本内容，不产生 `Keystroke`**。因此任何走这条
> 通道的输入（字母、以及旧实现里被截成 `nativeCommitText("\n")` 的回车）都**不会**
> 出现在 `observe_keystrokes` 的列表里。这是"下方列表有时没有字母"的根本原因。

### 输入类型决定回车走哪条

同一台小米输入法，因 App 请求的 `inputType` 不同而表现不同：

- `KeyboardType::Default` = `TYPE_CLASS_TEXT`（05 用）：回车 → `performEditorAction`
  → 若 Java 里把它转成 `nativeCommitText("\n")`，则走 IME 文本通道（不进列表）。
- `KeyboardType::MultiLine` = `TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_MULTI_LINE`（06 用）：
  回车 → `sendKeyEvent(KEYCODE_ENTER)`（输入法把回车当硬件键下发）→ 这条**不在**
  IME 文本通道里，原本会被 `BaseInputConnection` 默认实现吞掉。

---

## 2. KeyEvent 通道（硬件键 + 我们加的转发）

### 2.1 硬件键（本来就有）

安卓系统把物理键盘 / DPAD / 系统键作为 `AInputQueue` 事件，由 `android-activity`
通过 `app.input_events_iter()` 暴露。`process_input_events()` 在主线程每帧轮询：

```
InputEvent::KeyEvent(key_event)
  → AndroidKeyEvent { key_code, action, meta_state, unicode_char }
  → win.handle_key_event(key_event)
  → on_key_event 闭包
  → android_key_to_keystroke(...) → PlatformInput::KeyDown/KeyUp
  → GPUI 内部派发（含 observe_keystrokes 回调）
```

硬件键从一开始就正确产生 `Keystroke`，所以物理键盘的字母/回车/退格**都会**进
`observe_keystrokes` 列表。

### 2.2 软键盘 `sendKeyEvent` 转发（我们加的 `nativeKeyEvent`）

问题：多行输入法把回车/退格通过 `InputConnection.sendKeyEvent(KeyEvent)` 下发，
而 `BaseInputConnection` 默认实现要么丢弃、要么派发到一个 GPUI 收不到的地方，
导致 06 回车不换行、且这些键进不了列表。

早期（被推翻的）做法是：在 Java 里 `sendKeyEvent` 重写里直接调
`nativeCommitText("\n")` / `nativeDeleteSurroundingText(1,0)` 把键"翻译成文本"。
这有两个坑：

1. **跨线程崩溃**：`sendKeyEvent` 是从 IME 的 Binder 线程回调的，而
   `handle_key_event` 会同步访问 GPUI 窗口状态（锁、`on_key_event` 闭包）。
   直接在 Binder 线程调它 → 数据竞争 / panic → 06 闪退。
2. **吞掉 keystroke**：把回车截成 `nativeCommitText("\n")` 后，它走的是 IME 文本
   通道，不再产生 `Keystroke`，于是 05 的"下方列表"对软键盘回车/退格一片空白。
   而且 `enter` 被偷换成了"换行文本"，违反了"enter 就是 enter"的语义。

**正确的做法**是新增 `nativeKeyEvent(code, action, meta)`：

```java
// GpuiActivity.kt —— sendKeyEvent / performEditorAction 都改调它
@Override
public boolean sendKeyEvent(android.view.KeyEvent event) {
    nativeKeyEvent(event.getKeyCode(), event.getAction(), event.getMetaState());
    return true;
}
@Override
public boolean performEditorAction(int editorAction) {
    nativeKeyEvent(KEYCODE_ENTER, ACTION_DOWN, 0);
    nativeKeyEvent(KEYCODE_ENTER, ACTION_UP,   0);
    return true;
}
```

Rust 侧：

```rust
// jni.rs
pub unsafe extern "C" fn Java_..._nativeKeyEvent(code, action, meta) {
    // 只入队，不在 Binder 线程碰任何 GPUI 状态
    enqueue_forwarded_key(ForwardedKey { key_code: code, action, meta_state: meta });
}

// process_input_events() 主线程每帧 drain：
for fk in drain_forwarded_keys() {
    let key_event = AndroidKeyEvent { key_code: fk.key_code, action: fk.action,
                                      meta_state: fk.meta_state, unicode_char: 0 };
    win.handle_key_event(key_event);   // 与硬件键完全相同的路径
}
```

设计要点：

- **线程安全**：`nativeKeyEvent` 仅 `push` 到一个 `Mutex<VecDeque<ForwardedKey>>`
  队列（和 `IME_EVENTS` 同款模式），真正的 `handle_key_event` 在主线程执行。
- **走与硬件键完全相同的路径**：所以软键盘的回车/退格产生的 `Keystroke` 和物理
  键盘一模一样——能进 `observe_keystrokes` 列表，也能被聚焦组件当按键处理。
- **`unicode_char` 强制为 0**：否则 `enter` 会被派生出 `\n` 字符，使
  `prefer_character_input=true`，GPUI 会把回车当字符插入而非触发 Enter action，
  导致 06 不换行。设 0 后 `enter` 是纯按键，由组件决定作用。

---

## 3. `observe_keystrokes` 与"下方列表"

`cx.observe_keystrokes(|ev, _, cx| { ... })` 是 GPUI 提供的全局 keystroke 观察
回调，每次 GPUI 内部派发一个 `Keystroke` 都会触发。**只有走 KeyEvent 通道的输入
才会到达这里**；IME 文本通道（`commitText` 等）不会。

- **05**（`apps/05_android_input`）在 `InputExample::render` 里用
  `recent_keystrokes` 列表把每次 keystroke 的 `ks.unparse()` 以及 `key_char`
  显示出来（如 `enter -> "\n"`、`x -> "x"`、backspace）。所以：
  - 物理键盘字母/回车/退格 → 进列表 ✅
  - 软键盘字母 → 走 IME 文本通道 → **不**进列表（框里有字，但列表不显示）
  - 软键盘回车/退格 → 经 `nativeKeyEvent` 转发 → 进列表 ✅（06 修复后）
- **06** 用的是 GPUI `Editor`，本身不显示 keystroke 列表，但同样通过
  `on_key_event` 收到 `enter`/`backspace` keystroke，由 Editor 自己的
  `InsertNewline` / `Backspace` action 处理。

> 你"之前在真机上看到 `x -> "x"`、`enter -> "\n"`"的疑问：软键盘字母实际走的是
> IME 文本通道，**不会**产生 `x -> "x"`；你当时看到的列表内容其实是**物理键盘**
> 测试的残留，或输入法对部分字符走了 `sendKeyEvent`。字母本身不进列表是预期行为。

---

## 4. 设计原则：`enter` 不等于换行

这是本仓库在调试 07 回车时定下的原则：

- **`enter` 是一个按键事件（keystroke）**，语义上就是"用户按了回车键"。
- **换行是编辑器对 `enter` 的响应**，不是 `enter` 本身的属性。
- 因此：
  - 06 的 `Editor` 收到 `enter` keystroke → 自己的 `InsertNewline` action → 换行。
  - 05 的 `TextInput` 是纯 IME 输入框，没绑 Enter action → 收到 `enter` 只在列表
    显示，**不换行**。这正符合"enter 就是 enter"。
  - 日志里显示 `enter`（不带 `-> "\n"`）比显示 `enter -> "\n"` 更准确；后者容易让人
    误以为"按回车 = 插入换行符"。

不要把回车在 Java 里偷偷 `nativeCommitText("\n")` 掉——那既吞掉 keystroke（列表
空白），又把"换行"硬编码进输入法桥接层，破坏了"组件决定行为"的分层。

---

## 5. 端到端数据流图

```
软键盘打字 "h"
  IME.commitText("h") → nativeCommitText → ImeEvent::Commit
  → [主线程] replace_text_in_range(None, "h")          ← 改文本，无 keystroke

软键盘回车（多行输入法 sendKeyEvent）
  IME.sendKeyEvent(KEYCODE_ENTER)
  → nativeKeyEvent(66, DOWN, 0)                         ← 仅入队，线程安全
  → [主线程 process_input_events] drain → handle_key_event
  → on_key_event → Keystroke{key:"enter"} → KeyDown
  → 06 Editor: InsertNewline（换行）                   ← 组件决定
  → 05 TextInput: 无 Enter action，仅 observe_keystrokes 显示 "enter"

物理键盘回车
  AInputQueue → input_events_iter → 同上 handle_key_event 路径

软键盘退格（多数输入法 deleteSurroundingText）
  IME.deleteSurroundingText(1,0) → nativeDeleteSurroundingText
  → ImeEvent::DeleteSurrounding → replace_text_in_range(删除)   ← 改文本

软键盘退格（少数输入法 sendKeyEvent(KEYCODE_DEL)）
  IME.sendKeyEvent(KEYCODE_DEL)
  → nativeKeyEvent(67, DOWN, 0) → [主线程] handle_key_event
  → Keystroke{key:"backspace"} → KeyDown
  → 组件 Backspace action（删字），不崩、不插入控制字符
```

---

## 6. 版本自报（确认设备跑的是最新构建）

为随时核对真机是否是最新代码：

- `apps/06_text_area/Cargo.toml` 与 `package.json` 的 `version` 保持同步。
- `src/lib.rs` 用 `env!("CARGO_PKG_VERSION")` 在启动时打印：
  - 安卓：`android_main: entered (text_area_06 vX.Y.Z)`
  - 桌面：`text_area_06 vX.Y.Z 桌面端启动`
- 改完代码先 `bun run version:bump`（同时升两个文件），再构建安装，看 logcat
  里的版本号即可确认设备跑的是不是刚编的版本。

---

## 7. 已知限制 / 坑

- **方向键**（`KEYCODE_DPAD_LEFT` 等）也走 `sendKeyEvent`，目前经 `nativeKeyEvent`
  转发成 keystroke；光标移动由聚焦组件处理（06 Editor 已绑 `Left`/`Right`）。
- **IME 文本通道不产生 keystroke**：如果将来想让"软键盘打的字母"也进
  `recent_keystrokes` 列表，需要在 `ImeEvent::Commit` 处理时额外合成一个 keystroke，
  而不是复用硬件键路径。当前设计刻意为之（字母走文本通道更可靠）。
- **`nativeKeyEvent` 必须只入队**：任何想在 Java/IEM 线程直接操作 GPUI 窗口的
  尝试都会跨线程崩溃。新增 IME 相关 native 方法时务必沿用"入队 + 主线程消费"模式。
