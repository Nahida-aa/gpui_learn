# gpui-d3rs / gpui-px：GPUI 里的可视化与图表

`gpui-toolkit` 里有两层可视化能力：**底层原语 `gpui-d3rs`**（D3.js 风格的完整移植）
和**上层图表 `gpui-px`**（Plotly Express 风格的便捷 API）。本文记录它们的定位、
范围和用法，作为「GPUI 能做数据可视化到什么程度」的参考。

> 位置：`gpui-toolkit/crates/gpui-d3rs/` 与 `gpui-toolkit/crates/gpui-px/`。
> `gpui-px` 构建在 `gpui-d3rs` 之上。

---

## 1. 两层关系

```
gpui-px  (Plotly Express 风格：scatter/line/bar/heatmap/3D…，3 行出图)
   │  依赖
   ▼
gpui-d3rs (D3 风格底层原语：scale/color/shape/geo/force/delaunay/voronoi/
           contour/hexbin/sankey/axis/grid/legend，含 GPU 2D/3D)
```

`gpui-px` 的 `lib.rs` 直接 re-export 了 `gpui-d3rs` 的类型（如
`D3Color`、`gpu3d::{Colormap, Surface3DState}`、`shape::{CurveType, StrokeDashArray}`），
说明上层图表复用底层原语。

---

## 2. gpui-d3rs：D3 风格的完整移植

`src/lib.rs` 的 `pub mod` 列表几乎覆盖了 D3 的全部子系统：

| 类别       | 模块                                                                                                              |
| ---------- | ----------------------------------------------------------------------------------------------------------------- |
| 数据与变换 | `array`（statistics/ticks/transform/search/sets）、`format`、`time`、`random`                                     |
| 比例尺     | `scale`                                                                                                           |
| 颜色       | `color`（hcl / rgb / interpolate / chromatic 顺序·发散配色方案 / scheme）                                         |
| 形状       | `shape`（CurveType / StrokeDashArray）、`polygon`、`grid`、`hexbin`                                               |
| 坐标轴     | `axis`（config / layout / render / theme / orientation）                                                          |
| 几何/空间  | `geo`、`sphere_gallery`、`delaunay`、`vortex`→`voronoi`、`quadtree`                                               |
| 布局算法   | `force`、`hierarchy`、`chord`、`sankey`、`treemap`（经 px）、`contour`（marching_squares / density / thresholds） |
| 交互       | `drag`、`zoom`、`brush`、`selection`、`dispatch`                                                                  |
| 动画       | `ease`、`transition`、`timer`                                                                                     |
| 渲染后端   | `gpu2d`、`gpu3d`（**GPU 加速的 2D/3D 绘制**）、`text`、`text_layout`、`tile`                                      |
| 其它       | `legend`、`surface`、`fetch`、`feature_parity`                                                                    |

要点：

- **`gpu2d` / `gpu3d`**：说明这套可视化是走 wgpu GPU 渲染的（与 `gpui-ui-kit`、
  `gpui-android/ios` 后端同源），不是 CPU 光栅化。
- **`feature_parity` / `examples`** 模块：作者在做 D3 API 的覆盖度追踪与示例。

---

## 3. gpui-px：Plotly Express 风格的图表

`src/lib.rs` 公开的函数式构造器（每个都对应一个 `*Chart` 结构体 + 流式 builder）：

```rust
pub use scatter::{ScatterChart, ScatterTheme, scatter};
pub use line::{LineChart, line};
pub use bar::{BarChart, BarTheme, bar};
pub use area::{AreaChart, area};
pub use pie::{PieChart, donut, pie};
pub use heatmap::{HeatmapChart, heatmap};
pub use boxplot::{BoxPlotChart, boxplot};
pub use treemap::{Treemap, treemap};
pub use isoline::{IsolineChart, isoline};
pub use contour::{ContourChart, contour};
pub use surface3d::{Surface3DChart, surface3d};
```

**用法（README 示例）** —— 3 行出图，和 Python `px.scatter()` 几乎同款：

```rust
use gpui_px::{scatter, line, bar, heatmap, contour, isoline};

// 散点图
let chart = scatter(&x_data, &y_data)
    .title("My Chart")
    .build()?;
```

其余图表同理：`let chart = bar(&categories, &values).title("...").build()?;`
支持的类型：`scatter` / `line` / `bar` / `area` / `pie`·`donut` / `heatmap` /
`boxplot` / `treemap` / `isoline` / `contour` / `surface3d`。

### 不止绘图：图表周边能力

`gpui-px` 还有超出「画图」的工程化模块：

- `interaction`（`chart_interaction` / `mouse_state` / `wheel_config`）—— 缩放、平移、hover
- `accessibility`（`ChartAccessibilitySummary::to_bridge_snapshot()`）—— 把图表转成无障碍摘要
- `visual_regression` —— 视觉回归（与 toolkit 的 QA 体系对接）
- `static_export`（`StaticSvgOptions`）—— 导出静态 SVG
- `annotation` / `legend` / `chart_capabilities` / `chart_size` —— 标注、图例、能力声明、尺寸约束

---

## 4. 与本仓库的关系

- 我们 `apps/02_hello_web` 是**质数筛 UI**，不含任何图表；`gpui-px`/`gpui-d3rs`
  展示了 GPUI 在数据可视化方向能长到**接近 D3 + Plotly** 的程度。
- 两者锁 zed `v1.9.0`，与本仓库（zed `82aef443`）**不兼容**，不能直接作依赖；
  但作为「GPUI 可视化能力上限」的标杆很有参考价值。
- 若想用：`gpui-scaffolder` 生成的骨架默认不含图表，但可在其 `Cargo.toml` 加
  `gpui-px = { path = "../gpui-toolkit/crates/gpui-px" }` 后按本文 API 调用。

> 可视化是 GPU 渲染（`gpu2d`/`gpu3d`），因此无论桌面还是移动端（iOS Metal /
> Android Vulkan）都能跑——与 `docs/mobile-backends.md` 的后端机制一致。
