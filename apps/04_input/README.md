# 04_input —— 文本输入例子（桌面端）

移植自 zed 官方例子 [`crates/gpui/examples/input.rs`](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs)，演示 GPUI 中**从零手写一个文本输入框**需要的全部机制：光标、选区、IME 输入法、剪贴板、键盘布局切换。

这是 `04` 号例子（桌面端）。它是在 `05_grid_layout` 之后按你的规划排定的「桌面 input 例子」——移动端 input 要等它学完、再把这套逻辑移植到 `gpui-android` 后端。

## 运行

```bash
# 方式一：cargo 直接跑
cargo run -p input_04

# 方式二：package.json 脚本（IDE 会在旁边放运行按钮）
bun run dev      # = cargo run -p input_04
bun run build    # = cargo build -p input_04
bun run check    # = cargo check -p input_04
```

## 学什么

这个例子**不依赖** GPUI 内置的 `TextInput` 组件，而是用底层 API 自己实现一个，因此能看清输入系统的全貌：

| 机制                        | 代码位置 / API                                       | 说明                                                                                                                                       |
| --------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `Focusable` + `track_focus` | `TextInput` / `InputExample` impl `Focusable`        | 输入框要先能拿到焦点，才能收到键盘事件                                                                                                     |
| `ElementInputHandler`       | `window.handle_input(&focus, handler, cx)`           | 把「元素区域」注册成输入目标，GPUI 才会把按键 / IME Composition 派发给它                                                                   |
| `EntityInputHandler`        | `impl EntityInputHandler for TextInput`              | 实现 `text_for_range` / `replace_text_in_range` / `selected_text_range` 等，供 GPUI 读写文本与选区（含 UTF-16 偏移换算，处理中文等多字节） |
| IME 合成                    | `marked_range` + `replace_and_mark_text_in_range`    | 输入法候选词阶段的高亮；`TextRun` 的下划线渲染                                                                                             |
| 选区 / 光标绘制             | `TextElement::prepaint`                              | 用 `ShapedLine.x_for_index` 算光标 x 坐标，自己 `fill()` 画光标和选区矩形                                                                  |
| 剪贴板                      | `cx.read_from_clipboard` / `cx.write_to_clipboard`   | 实现 Copy / Cut / Paste                                                                                                                    |
| 鼠标定位光标                | `index_for_mouse_position` + `closest_index_for_x`   | 点击 / 拖拽选中                                                                                                                            |
| 键盘布局                    | `cx.keyboard_layout().name()` + `observe_keystrokes` | 顶部显示当前键盘布局，下方回显最近按键（含 `key_char`）                                                                                    |
| 按键绑定                    | `cx.bind_keys([KeyBinding::new(...)])`               | Backspace / 方向键 / Cmd-A / Cmd-C/V/X / Home/End 等                                                                                       |

> 提示：这是「教学用的最简实现」，目的是暴露原理。真实项目应优先用 GPUI 自带的 `TextInput` 组件。

## 与官方例子的差异

- 加了 `apps/04_input/` 的 `Cargo.toml` / `package.json` / 本 README，逻辑与官方一致。
- 通过 workspace 依赖引入本仓库锁定的 `gpui`（zed `82aef443`）与 `gpui_platform`，不单独升级。
- `main.rs` 顶部 `#![cfg_attr(target_family = "wasm", no_main)]` + wasm 入口保留，便于将来编译成 Web 端对照 `02`；桌面端走 `fn main()`。

## 下一步

- 把本例的 `TextInput` 逻辑移植到 `crates/gpui-android` 后端，作为「移动端 input 例子」（移动端 input 应排在桌面 input 之后学，见 `docs/mobile-backends.md`）。
- 可在 `05_grid_layout` 的 center 区域嵌入输入框，组合练习布局 + 输入。
