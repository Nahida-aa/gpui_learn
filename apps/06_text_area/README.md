# 06_text_area —— 用 `gpui-android` 后端在手机上跑**多行文本输入**

第五个 Android 例子，是 `05_android_input`（**单行**文本输入）的**多行版**。

## 核心结论

> **多行输入在 GPUI 上不是「后端要补功能」，而是「应用层用多行渲染 + 回车换行」**。
> 本例直接移植自 zed 仓库 `crates/gpui/examples/view_example/` 的官方教学示例
> （`example_editor.rs` + `example_text_area.rs`），它用 GPUI 原语从零展示了
> 一个多行编辑器是怎么实现的。

为什么不用 Zed 主编辑器 `crates/editor`（也就是 Zed agent 聊天输入框用的那个）？
因为它依赖整个 IDE 后端（client / git / lsp / language / project …），对一个
Android 小示例太重，而且面向 `Project`/`Buffer` 文件编辑。这里用的是 zed
**官方教学示例**里的 `Editor` / `TextArea`，思路相同但轻量、透明，正好与
`05`（单行 `Input`）形成「单行 → 多行」的递进教材。

## 多行是怎么实现的（关键点）

- **逐行渲染**：`EditorText::prepaint` 里 `content.split('\n')` 逐行 `shape_line`，
  `request_layout` 高度 = `line_height * 行数`，`paint` 里逐行 `line.paint(...)`
  并按下标偏移 y。
- **回车换行**：`TextArea` 把 `Enter` 绑定到 `Editor::insert_newline`（这是多行框
  与单行 `Input` 的唯一区别），软键盘回车即在文本里插入 `\n`。
- **光标定位**：`cursor_line_and_offset` 把 utf8 偏移映射成 (行号, 行内偏移)，
  比手写 `WrappedLine` 坐标映射直观。
- **IME 接入**：和 05 同源——`paint` 里 `window.handle_input(&focus_handle,
ElementInputHandler::new(bounds, editor), cx)`，经 gpui-android 的
  `nativeCommitText` 桥接（已在 `acbc43f` 修复闪退，见 `05_android_input/docs/IME_INPUT_DEBUG.md`）。

## 和 05_android_input 比

| 差异       | `05`（单行 `Input`）                            | `06`（多行 `TextArea`）                                       |
| ---------- | ----------------------------------------------- | ------------------------------------------------------------- |
| 文本模型   | 单行 `String`                                   | 多行 `String`，按 `\n` 分段                                   |
| 渲染       | 整段当一行 `shape_line`，高度写死 `line_height` | `split('\n')` 逐行 `shape_line`，高度 = 行数 × `line_height`  |
| 回车       | （本例不处理）                                  | `Enter` → `insert_newline`，软键盘回车即换行                  |
| 来源       | 自己写的简化 `TextInput`                        | 移植自 zed `view_example`（官方示例的 `Editor` + `TextArea`） |
| 软键盘弹出 | `on_focus_in` → `show_keyboard_android`         | 同 05，IME 路径完全一致                                       |

## 构建（与 05 相同的方式）

本例子同样**不写任何 Gradle/Kotlin 脚本**，工程由 `gpui-cli` 生成到 `gen/android/`。

```bash
bun run init     # gpui-cli android init → 生成 gen/android/
bun run apk      # cd gen/android && ./gradlew assembleDebug
bun run install  # adb install -r gen/android/app/build/outputs/apk/debug/app-debug.apk
bun run launch   # adb shell am start -n dev.gpui.learn.text_area_06/dev.gpui.mobile.GpuiActivity
```

IME 桥接排查记录见 `05_android_input/docs/IME_INPUT_DEBUG.md`（两例共用同一套
`GpuiActivity.java`，修复也通用）。
