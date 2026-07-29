# gpui-ui-kit：GPUI 的生产级组件库

`gpui-toolkit` 的 `gpui-ui-kit` 是一个**完整的、带设计系统的 GPUI 组件库**——远比
我们在 `apps/02_hello_web` 里手写的 `div()` 高级。本文记录它的组件范围、API 风格，
以及主题/设计系统机制。作为 `apps/02`（手写 div）的「成品组件」对照。

> 位置：`gpui-toolkit/crates/gpui-ui-kit/`。第一个用它构建的真上架 App 是
> [SotF](https://github.com/pierreaubert/sotf)（App Store + Microsoft Store）。

---

## 1. 组件范围（节选，约 80+ 个模块）

来源：`src/lib.rs` 的 `pub mod` 列表 + `README.md` 的组件表。

**基础 / 输入**
`Button` · `IconButton` · `ButtonSet` · `Input` · `NumberInput` · `Select` ·
`Checkbox` · `Toggle` · `Slider` · `ColorPicker` · `SearchBar` · `Text`

**布局 / 容器**
`Card` · `Stack` · `SplitPane` · `PaneDivider` · `Sidebar` · `StatusBar` ·
`SwipePanel` · `Accordion` · `Tabs` · `Wizard` · `Toolbar` · `Breadcrumbs` ·
`EmptyState` · `Avatar` · `Badge` · `Tag`

**浮层 / 反馈**
`Dialog` · `ConfirmDialog` · `ContextMenu` · `Menu` · `Popover` · `Tooltip` ·
`Toast` · `Notification` · `LoadingOverlay` · `Spinner` · `Progress` ·
`Alert` · `StepIndicator`

**数据 / 高级**
`Table` · `TreeView` · `DataNavigation` · `QR`（`qr` 模块）· `Workflow`（工作流画布）
· `SettingsForm` · `CommandPalette`（命令面板）

**系统 / 基建**
`Theme`（主题）· `DesignSystem`（设计系统）· `ColorTokens`（颜色 token）·
`I18n`（国际化）· `Accessibility`（无障碍树）· `Animation` · `Scale` ·
`AdaptiveOverflow`（自适应溢出）· `VisualRegression`（视觉回归）

---

## 2. API 风格：声明式 builder，和我们 `02` 一致

组件是 `#[derive(IntoElement)]` 的结构体，用 builder 方法链式配置——和我们
`02` 里手写的 `div().id(...).bg(...).on_click(...)` 是**同一种范式**，只是封装成了
带主题/无障碍的成品。

`Button`（`src/button.rs`）：

```rust
let button = Button::new("submit", "Submit")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::M)
    .icon_left("check")
    .on_click(|_window, _app| { /* ... */ })
    .aria_label("Submit the form");
```

变体：`Primary` / `Secondary` / `Destructive` / `Ghost` / `Outline`。

`Input`（`src/input.rs`）：

```rust
Input::new("name")
    .value("")
    .placeholder("Your name")
    .label("Name")
    .on_change(|text, _window, _app| { /* ... */ })
    .error("Required");
```

事件：`on_change` / `on_edit_start` / `on_edit_end` / `on_text_change`。

---

## 3. 主题机制（核心差异点）

和 `02` 手写固定颜色（`rgb(0x1e1e2e)`）不同，`ui-kit` 用**全局主题 + 组件主题派生**。

`Button::render()` 里的落地（`src/button.rs`）：

```rust
let global_theme = cx.theme();                       // 取全局 Theme
let theme = self.theme
    .unwrap_or_else(|| ButtonTheme::from(global_theme.as_ref()));  // 派生组件主题
let (bg, bg_hover, text_color, border_color) =
    Self::compute_colors(self.variant, self.selected, &theme);    // 变体→颜色
```

即：每个组件默认从**全局 `cx.theme()`** 派生自己的配色，也可 `.theme(...)` 显式覆盖。

`Theme` 自带多套预设（`src/theme.rs::Theme`）：`dark()` / `light()` / `midnight()` /
`forest()` / `black_and_white()` / `onyx()` / `carbon_white()` / `carbon_gray*` 等，
并能 `for_variant(ThemeVariant)` 选暗/亮。还有 `accent_token()` / `success_token()` /
`warning_token()` / `error_token()` —— 返回 `ColorToken`（设计 token，而非裸 RGB）。

---

## 4. 设计系统（DesignSystem）与平台适配

`design.rs` 暴露：

```rust
pub fn resolve_design(explicit: Option<Arc<DesignSystem>>, cx: &mut App) -> Arc<DesignSystem>
pub fn neutral_design() -> Arc<DesignSystem>
pub fn platform_design() -> Arc<DesignSystem>   // 按平台返回 Apple/ Material 风格
```

组件 `render` 里通常先 `let design = resolve_design(self.design.clone(), cx);` 拿到设计
系统，再据其算间距/字号（`padding_for_size(size, &design)` 等）。`platform_design()`
让同一套组件在 iOS 上偏 Apple 风格、Android 上偏 Material——对应 `gpui-ui-kit`
README 里 `components/glass`（Apple）与 `components/material`（Material）两套风格实现。

主题/设计系统的全局状态在 app 启动时设置（见 `docs/scaffolder.md` 生成的
`lib.rs`、以及 `docs/mobile-backends.md` 的 `mini_app.rs::run()`）：

```rust
cx.set_global(ThemeState::with_variant(...));
cx.set_global(DesignSystemState::new());
```

---

## 5. 与本仓库的关系

- 我们 `apps/02_hello_web` 是**手写 `div()`**，适合学 GPUI 最原始的 API；
  `gpui-ui-kit` 是**成品组件 + 设计系统**，适合直接做产品 UI。两者是「基础」与
  「上层封装」的关系，不是替代。
- `ui-kit` 锁在 zed `v1.9.0`，与本仓库 `gpui_learn`（zed `82aef443`）**不兼容**，
  不能直接作为 `gpui_learn` 的依赖；但作为「GPUI 能长到什么程度」的参考标杆很有价值。
- 若想真用 `ui-kit` 起项目，走 `docs/scaffolder.md` 的 `gpui-scaffolder` 生成骨架，
  它已配好所有 path 依赖与平台 target。
