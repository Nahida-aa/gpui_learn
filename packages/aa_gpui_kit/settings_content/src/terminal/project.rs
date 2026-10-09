//! 项目级 terminal settings — 可被 `.aa/settings.json` 覆盖。
//!
//! 单独拆模块是因为 Zed 用 `#[serde(flatten)]` 把这个 struct
//! 内嵌进顶层 `TerminalSettingsContent`，逻辑上是两层 settings。
//!
//! 本模块还定义了 `Shell` enum（纯 serde 数据类型 + runtime 方法），
//! 它依赖 `util::shell::ShellKind` 做检测。

use std::path::PathBuf;

use collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::terminal::shell::VenvSettings;

use super::shell::{PathHyperlinkRegex, Shell, WorkingDirectory};

/// 项目级 terminal settings — 被 `serde(flatten)` 进 TerminalSettingsContent。
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ProjectTerminalSettingsContent {
    /// Shell — 复用 `util::shell::Shell`。
    pub shell: Option<Shell>,
    /// 工作目录策略。
    pub working_directory: Option<WorkingDirectory>,
    /// 环境变量（会加进 terminal 进程环境）。
    pub env: Option<HashMap<String, String>>,
    /// Python venv 自动激活。
    pub detect_venv: Option<VenvSettings>,
    /// 路径超链接 regex 列表。
    pub path_hyperlink_regexes: Option<Vec<PathHyperlinkRegex>>,
    /// 超链接发现超时（ms）。
    pub path_hyperlink_timeout_ms: Option<u64>,
}
