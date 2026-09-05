# ui-gpui 文本输入/编辑器开发路线

> 目标：在 `packages/ui-gpui/src/base/input` 建设自己的文本编辑引擎及衍生组件
> （单行 Input → 多行 Textarea → Editor），最终能力对标
> `/home/aa/repos/ide_ls/gpui-component/crates/base/src/input` 的分层。
>
> 本文档回答的问题：zed 的 gpui examples 还没走完，是继续走完还是现在开工？
> ——**现在开工**。examples 里与 editor 直接相关的没走项只剩少数几个，
> 把它们作为各阶段的「课前阅读」穿插进来，而不是独立关卡。

## 1. 三个参照系

| 来源 | 角色 |
|---|---|
| `gpui-component/crates/base/src/input` | **主参考**：成熟的三层实现（约 14k 行），目录结构即路线图 |
| `learn_ls/zed/crates/gpui/examples` | **知识补给**：与 editor 相关的少数示例作课前阅读 |
| 本仓库已有资产 | **迁移来源**：`04_input`、`06_text_area`、ui-gpui 现有 Input 已实现过三遍引擎骨架 |

gpui-component 的 `input` 模块分层（自述见其 `README.md`）：

```
input/
├── base/     11912 行  共享编辑引擎：state、layout、cursor、selection、
│                      movement、masking、native、painting、undo、rope
├── input/       33 行  单行 Input 薄 facade（Input/InputState）
├── textarea/    30 行  多行 Textarea 薄 facade（Textarea/TextareaState）
└── editor/    1863 行  Editor 增强：display_map、highlighting、search、
                       diagnostics、decorations、indent、LSP
```

关键架构事实：**引擎做厚、facade 做薄**（facade 只有 33/30 行）；`mod.rs`
是外部缝（显式 `#[path]` 让内部重组不影响调用方 import）；三种组件共用一个
引擎，只差 `kind.rs` 里的 mode（`InputMode / TextareaMode / EditorMode`）。

## 2. 现状盘点

| 资产 | 规模 | 状态 |
|---|---|---|
| `apps/04_input` | 778 行单文件 | 手写 `EntityInputHandler` 教学版（对应 zed `examples/input.rs`） |
| `apps/06_text_area` | editor.rs 1051 行 + text_area.rs 257 行 | 自建轻量 `Editor` 引擎 + `TextArea` 外壳（`Source::Editor`/`Source::Value` 双来源） |
| `packages/ui-gpui/src/base/input` | input_state.rs 584 + element.rs 211 | **工程化单行 Input**：`Entity<InputState>` + `Render`、`InputEvent::{Change, Submit}`、IME `marked_range`、`key_context` 限定绑定 |
| `apps/10_input_button` | — | 单行 Input 完整手动验证清单（含中文 IME 组字），已通过 |

现有 Input 的存储是 `SharedString` + 单行 `ShapedLine` 布局缓存 +
`selected_range`/`marked_range`，`unicode-segmentation` 做字素簇移动。

## 3. 架构决策（先记录，后遵守）

1. **引擎厚、facade 薄**：`Input`/`Textarea`/`Editor` 三个 facade 只做
   mode 差异与 API 包装，逻辑全部下沉引擎层。
2. **一个引擎 + mode 区分**：参照 `kind.rs`，用
   `InputMode/TextareaMode/EditorMode` 统一三种组件，避免三份复制。
3. **文本存储：sum_tree + 自研 Rope（已定，不用 ropey）**：
   - `sum_tree` 是 zed 的并发友好 B-tree（`publish = false`，crates.io 无，
     需 git 依赖 zed 仓库、与 gpui 的 `[patch.crates-io]` 用同一 rev）
   - Rope 参照 zed `crates/rope/src/rope.rs` 自研精简版：
     `pub struct Rope { chunks: SumTree<Chunk> }`
   - 选型依据（2026-09 调研）：IDE 级编辑器（display_map、软换行、折叠）
     本就在路线图内，sum_tree 的 `TextDimension`/`Dimensions<D1, D2>`
     多坐标系联合查询（字节/UTF-16/行列一棵树内 O(log n) 互转）是
     ropey 给不了的；与其 ropey 先上再迁移，不如一步到位
   - 不直接依赖 zed `crates/rope` 的原因：硬依赖 `util`
     （anyhow/serde/regex/rust-embed 等大闭包）；且 Rope 引擎正是本仓库
     的核心学习对象。rope 对 util 的实际使用仅
     `debug_panic`/`is_utf8_char_boundary`/`RandomCharIter`（测试），
     自行补三行即可绕开
4. **`mod.rs` 作为外部缝**：公开 re-export 只放 `mod.rs`，内部文件用显式
   `#[path]` 组织，重组不破坏调用方。
5. **布局/选区索引约定**：沿用现有 UTF-16（`UTF16Selection`，与平台 IME
   对齐）+ 字素簇移动的组合，多行阶段引入 `Point`(行/列) 坐标系时再明确
   三者换算规则（参照 gpui-component `base/rope_ext.rs` 的 `Point`）。

## 4. 阶段路线

每阶段 = 迁移/新写 + 一个可运行验证（复用或新增 apps/1x_*）。
「课前阅读」指 zed `crates/gpui/examples/` 下对应示例，需要时再走。

### 阶段 0｜sum_tree 接入与自研 Rope（现在即可开工）
- 根 `Cargo.toml` 增加 `sum_tree = { git = "https://github.com/zed-industries/zed",
  rev = "<与 gpui 的 [patch.crates-io] 同 rev>" }`，升级时同 rev 一起 bump
- 在 `base/input` 下建 `engine/`，参照 zed `crates/rope/src/rope.rs` 自研
  精简 Rope：`SumTree<Chunk>` + `TextSummary`（含 chars/len/PointUtf16 等
  摘要）+ `Point`/`PointUtf16`/`OffsetUtf16` 坐标类型；chunk 内仍是连续
  `String` 段
- 把现有 `InputState` 的 `SharedString` 存量逻辑迁到 Rope 之上；编辑操作
  产出 `InputEdit` 式增量记录（参照 gpui-component `base/rope_ext.rs`，
  供 undo/display map 复用）
- 验收：单行 Input 行为与 `10_input_button` 清单完全一致（回归）；
  补一个 Rope 单元/property 测试（随机编辑序列 vs `String` 参照实现）

### 阶段 1｜布局引擎多行化
- 课前阅读：zed `text_layout`、`text_wrapper`、`text`
- 参照 gpui-component `base/layout.rs`：单 `ShapedLine` → 多行布局 +
  软换行（wrapping）+ 视口滚动裁剪
- 验收：粘贴/删除大段文本不卡顿；行号坐标与鼠标点选一致

### 阶段 2｜光标、选区、移动、undo
- 参照 `base/cursor.rs`、`base/selection.rs`、`base/movement.rs`、
  `base/undo_manager.rs`
- 04/06 已有大部分逻辑可迁移：词边界（unicode-segmentation）、Home/End、
  SelectAll、剪贴板
- 验收：多行选择、Shift/双击选词、undo/redo 栈

### 阶段 3｜Textarea facade
- 课前阅读：（无硬需求，可选 `scrollable`）
- 参照 `textarea/mod.rs`（30 行薄 facade 的样子）+ 你 06 的
  `TextArea` 外壳经验（`Source::Editor`/`Source::Value` 双来源保留）
- 验收：多行输入、Enter 换行（区别于 Input 的 Submit）、自动高度或滚动

### 阶段 4｜浮层与装饰（Editor 前置）
- 课前阅读：zed `popover`、`anchor`、`painting`、`focus_visible`、`tab_stop`
- 产出：ui-gpui 通用浮层能力（deferred + anchored）+ 下划线/波浪线装饰绘制
- 验收：Input 内右键菜单或补全提示框可弹出、定位正确

### 阶段 5｜Editor：display_map、highlighting、search
- 参照 `editor/display_map/`（折叠/软换行映射——zed editor 同名概念的
  延续）、`editor/highlighting.rs`、`editor/search.rs`
- 诊断/indent/LSP 按需后置
- 验收：语法高亮多行编辑器 + 搜索跳转

## 5. 进度(2026-09)

- [x] 阶段 0:sum_tree 接入 + 自研 Rope(e28158a)
- [x] 阶段 A:核心引擎 selection/undo/Editor 骨架(de54a39)
- [x] 阶段 B:display_map 软换行映射 + movement 原语(a9130cb)
- [x] 阶段 C:EditorElement 渲染 + Textarea facade + apps/11_editor(5cbdc52)
- [x] 阶段 D:Input facade 收口(旧 input_state 删除,10_input_button 零改动)
- 待办:渲染接入真实软换行度量(map 已就绪)、movement 切 display 空间、
  光标自动滚动(cursor_pixel_position 已备)、多光标/高亮/搜索

## 5. 不做的事

- 不等 zed examples「走完」：上表之外示例（image/gif/animation/menus/
  window 系列）与 editor 无关，用到再看
- 不照抄 gpui-component 的 LSP/diagnostics 全家桶：那是 Editor 场景的
  深水区，按需渐进
- 不在 facade 层塞逻辑：发现 facade 变厚即回炉引擎
