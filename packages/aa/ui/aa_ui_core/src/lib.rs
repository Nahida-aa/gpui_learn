//! 前端无关的 UI 基础设施。
//!
//! 对应 zed 的 `crates/ui` 里那些不依赖 gpui 的部分。gpui 侧（`aa_ui_gpui`
//! 之类）与 iced 侧（`aa_ui_iced`）共用这里的东西，避免两份实现漂移。

pub mod utils;
