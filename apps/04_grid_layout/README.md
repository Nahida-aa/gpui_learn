# 04_grid_layout —— 响应式网格布局（桌面端）

移植自 zed 官方例子 [`crates/gpui/examples/grid_layout.rs`](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/grid_layout.rs)，演示 GPUI 的两个核心布局能力。这是**桌面端**例子（对应 `03` 是 Android 端）；之后会在它基础上加文本框做成「桌面 input 例子」，再之后才把 input 移植到 `gpui-android` 后端。

## 运行

```bash
# 方式一：cargo 直接跑
cargo run -p grid_layout_04

# 方式二：package.json 脚本（IDE 会在旁边放运行按钮）
npm run dev      # = cargo run -p grid_layout_04
npm run build    # = cargo build -p grid_layout_04
npm run check    # = cargo check -p grid_layout_04
```

## 学什么

### 1. CSS Grid 风格的网格

GPUI 的 `div()` 支持类 CSS Grid 的 API，用「网格线」而非 flex 来摆放区域，是实现经典「圣杯布局」（Header / 左栏 / 内容 / 右栏 / Footer）最自然的方式：

| API                             | 作用                                      |
| ------------------------------- | ----------------------------------------- |
| `div().grid()`                  | 声明本元素用网格布局                      |
| `grid_cols(5)` / `grid_rows(5)` | 定义列数 / 行数（`px()` 也能定轨道尺寸）  |
| `col_span(n)` / `row_span(n)`   | 某格子跨几列 / 几行                       |
| `col_span_full()`               | 跨满整行（用于 Header / Footer 占满宽度） |

本例布局（宽屏，5×5 网格）：

```
┌──────────────────────────────────────┐  ← col_span_full（Header）
│  left  │      center      │  right   │     左/中/右 三栏
│  (1列) │     (3列)        │  (1列)   │
└──────────────────────────────────────┘  ← col_span_full（Footer）
```

### 2. `container_query` 响应式

和「媒体查询看视口宽度」不同，`container_query` 看的是**元素自身被分到的实测宽度**——更贴合组件化思维：一个组件不管被放在多宽的页面里，只要自己被挤窄了就塌缩。

```rust
let narrow = container_query(px(400.));   // 容器实测宽度 < 400px 时为真
div()
    .child(header)
    .when(narrow, |this| this.flex_col())  // 窄：单列堆叠
    .when(!narrow, |this| this.grid().grid_cols(5))  // 宽：三栏网格
```

把窗口拖窄到 < 400px，观察从三栏网格塌缩成单列堆叠。

## 与官方例子的差异

- 加了中文教学注释，逻辑与官方一致。
- 通过 workspace 依赖引入本仓库锁定的 `gpui`（zed `82aef443`）与 `gpui_platform`，不单独升级。
- 入口为 `fn main()` → `run_example()` → `application().run(...)`，与 `01`/`02`/`03` 风格统一。

## 下一步

- 在 `center` 区域加 `TextInput` / `TextArea`，做成「桌面 input 例子」。
- 之后把该 input 例子的逻辑移植到 `gpui-android` 后端（移动端 input）。
