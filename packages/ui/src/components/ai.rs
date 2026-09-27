//! AI 相关组件（对齐 zed `crates/ui/src/components/ai.rs`）。
//!
//! zed 这里挂着 5 个文件：`agent_setup_button` / `ai_setting_item` /
//! `configured_api_card` / `skills_illustration` / `thread_item`。
//! 目前只搬了 [`configured_api_card`] —— AAgent 的
//! `language_models::provider::*` 与 `copilot_ui::sign_in` 用它显示
//! 「API key 已配置」卡片。其余等有调用方再说。

mod configured_api_card;

pub use configured_api_card::*;
