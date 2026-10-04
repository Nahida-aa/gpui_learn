//! AI 相关组件（对齐 zed `crates/ui/src/components/ai.rs`）。
//!
//! zed 这里挂着 5 个文件：`agent_setup_button` / `ai_setting_item` /
//! `configured_api_card` / `skills_illustration` / `thread_item`。
//! 已搬 [`configured_api_card`]（aacode 的 `language_models::provider::*`
//! 用它显示「API key 已配置」卡片）、[`ai_setting_item`]（settings_ui 的
//! 外部代理 / MCP 页用）、[`thread_item`]（sidebar 的线程切换器用）。
//! 还差 `agent_setup_button` / `skills_illustration`，等有调用方再搬。

mod ai_setting_item;
mod configured_api_card;
mod thread_item;

pub use ai_setting_item::*;
pub use configured_api_card::*;
pub use thread_item::*;
