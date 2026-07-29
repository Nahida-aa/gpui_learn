# 安卓软键盘回车换行 —— `sendKeyEvent` 路径

## 现象

安卓端用系统输入法按回车不换行。但：
- 字母 / 空格能正常输入（经 IME `commitText`）。
- 粘贴多行文本能正确换行（经 IME `commitText("1\n2")`）。
- 顶部诊断框对回车没有任何显示（没有 `⏎` 图标）。

桌面端回车正常（`Enter` action → `insert_newline`）。

## 根因

小米（等部分系统输入法）在软键盘上按回车键时，**并非**走 `InputConnection.commitText("\n")`，
也**并非**走 `performEditorAction`，而是走 `InputConnection.sendKeyEvent(KeyEvent(KEYCODE_ENTER))`
把回车当成“硬件按键事件”下发。

`gpui-android` 的 `GpuiActivity.onCreateInputConnection` 返回的 `BaseInputConnection`
只重写了 `commitText` / `performEditorAction` / `setComposingText` 等，
**没有重写 `sendKeyEvent`**。于是回车键事件落到 `BaseInputConnection` 的默认实现，
被直接丢弃 → 编辑器收不到任何换行信号。

### 为什么 05 一直能显示 `\n`，而 07 之前不行？

两者共用同一份 `gpui-android`（同一 `GpuiActivity`），但**两者请求的输入类型不同**，
而输入类型决定了输入法下发回车的方式：

- **05 用 `KeyboardType::Default`** = `TYPE_CLASS_TEXT`（**不带** `TYPE_TEXT_FLAG_MULTI_LINE`）。
  单行输入下，这台小米系统输入法把回车当成 IME action → 走 `performEditorAction`
  → `nativeCommitText("\n")`。所以 05 一直能显示 `\n`，**不需要 `sendKeyEvent`**。
  （实测 05 v0.1.1 去掉 `sendKeyEvent` 后回车仍走 `performEditorAction: 0` → `\n`。）
- **07 用 `KeyboardType::MultiLine`** = `TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_MULTI_LINE`。
  多行输入下，同一台小米输入法改把回车当“硬件键”下发 → 走 `sendKeyEvent(KEYCODE_ENTER)`。
  之前 `BaseInputConnection` 没重写 `sendKeyEvent`，被默认实现吞掉 → 07 回车没反应。
  **`sendKeyEvent` 重写对多行场景是必需的，不是多余兜底。**

结论：`performEditorAction` 覆盖“单行输入法发 IME action”的设备（如 05）；
`sendKeyEvent` 覆盖“多行输入法发硬件键”的设备（如 07 在本机）。两者都需保留。
（验证：`sendKeyEvent` 去掉后，05 仍正常、但 07 回车会失效，故必须保留。）

佐证（logcat，设备 `dev.gpui.learn.text_area_07`，v0.1.3→v0.1.4）：
```
IME sendKeyEvent: code=66 action=0      ← KEYCODE_ENTER 的 DOWN
nativeCommitText: "\n"                  ← 重写后正确提交换行
IME sendKeyEvent: code=66 action=1      ← KEYCODE_ENTER 的 UP
```
（注：`KEYCODE_DPAD_LEFT=21` / `KEYCODE_DPAD_UP=19` / `KEYCODE_DEL=67` 也都走 `sendKeyEvent`，
只有可打印字符额外走 `commitText`。）

## 修复

`crates/gpui-android/android/src/main/java/dev/gpui/mobile/GpuiActivity.java`
的 `BaseInputConnection` 匿名类新增 `sendKeyEvent` 重写，把 `KEYCODE_ENTER` 的
`ACTION_DOWN` 转成 `nativeCommitText("\n")`：
```java
@Override
public boolean sendKeyEvent(android.view.KeyEvent event) {
    int keyCode = event.getKeyCode();
    Log.i("text_area_07", "IME sendKeyEvent: code=" + keyCode
            + " action=" + event.getAction());
    if (event.getAction() == android.view.KeyEvent.ACTION_DOWN) {
        if (keyCode == android.view.KeyEvent.KEYCODE_ENTER) {
            nativeCommitText("\n");   // 复用与粘贴多行完全相同的 Rust 路径
            return true;
        }
        if (keyCode == android.view.KeyEvent.KEYCODE_DEL) {
            // 双保险：覆盖“输入法用 sendKeyEvent 发删除”的边缘情况
            nativeDeleteSurroundingText(1, 0);
            return true;
        }
    }
    return super.sendKeyEvent(event);
}
```
回车 → `nativeCommitText("\n")` → Rust `replace_text_in_range` 插入 `\n` → 多行渲染，
与粘贴路径一致，诊断框显示 `⏎`。

## 版本确认机制

为随时确认设备上跑的是不是最新构建，引入版本自报：
- `apps/07_example_text_area/Cargo.toml` 与 `package.json` 的 `version` 字段保持一致。
- `src/lib.rs` 用 `env!("CARGO_PKG_VERSION")` 在启动时打印：
  - 安卓：`android_main: entered (text_area_07 vX.Y.Z)`
  - 桌面：`text_area_07 vX.Y.Z 桌面端启动`
- 每次改完代码先 `bun run version:bump`（同时升两个文件），再构建安装，
  看 logcat 里的版本号即可确认设备是最新版。

## 待办 / 已知限制

- 删除键：实测在 07 / 05 里**本来就工作**——该输入法在多数情况下走
  `deleteSurroundingText`（已在 `BaseInputConnection` 重写并桥接
  `nativeDeleteSurroundingText`），而非 `sendKeyEvent(KEYCODE_DEL)`。
  为覆盖“输入法走 sendKeyEvent 发删除”的边缘情况，`sendKeyEvent` 里额外加了
  `KEYCODE_DEL → nativeDeleteSurroundingText(1,0)` 分支作为双保险（冗余但无害）。
- 方向键（`19`/`21` 等）同样走 `sendKeyEvent`，目前透传未处理；如需在编辑器内移动光标，
  也在此处映射（参考 `editor.rs` 的 `Left`/`Right` action）。
