//! 主题系统的前端无关内核。
//!
//! 本 crate 处于「core + 双后端」结构中间那一层：
//!
//! ```text
//! aa_theme_core  ← 颜色类型、OKLab 转换、颜色解析
//!    ├── aa_theme_gpui   （GPUI 后端；当前仍自带一份颜色实现，尚未切换）
//!    └── aa_theme_iced   （iced 后端）
//! ```
//!
//! **边界原则**：本 crate 不依赖任何 UI 框架，也不依赖 `gpui`。它只放两个后端
//! 都需要的纯数据与纯函数。凡是需要 `App`/`Context`/全局状态的逻辑（主题注册表、
//! 加载器、字体缓存）留在各自的 `*_gpui` / `*_iced` 里 —— iced 没有
//! `cx.set_global()` 这类机制，硬塞进来只会把 gpui 的形状固化成唯一形状。
//!
//! ## 迁移状态
//!
//! - [`color`]：**完成**。逐字搬自 `gpui::color`，gpui 侧待改为依赖本 crate。
//! - [`color_space`]：**完成**。搬自 `aa_theme_gpui::color_space`。
//! - 语义色集合（`ThemeColors` / `StatusColors` 等，约 4700 行）：**待搬**。
//!   它们除 `Hsla` 外不碰 gpui，等 gpui 侧改为依赖本 crate 时可以近乎逐字搬走；
//!   在那之前先不要动，否则会同时存在两份 `ThemeColors` 而无从对照。
//! - 图标主题、语法高亮主题：**暂不搬**。终端等新前端用不上，
//!   等真有第二个消费方再说。

pub mod color;
pub mod color_space;

pub use color::{
    Hsla, Rgba, black, blue, green, hsla, opaque_grey, red, transparent_black, transparent_white,
    try_parse_color, white, yellow,
};
pub use color_space::{Oklab, Oklch, hsla_to_oklab, hsla_to_oklch, oklch_to_hsla};
