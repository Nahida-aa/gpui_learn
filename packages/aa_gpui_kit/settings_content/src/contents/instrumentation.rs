use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

/// Configuration for developer-oriented instrumentation tools that collect
/// diagnostic data about a running Zed instance.
#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct InstrumentationSettingsContent {
    /// Configuration for the performance profiler, accessed via the
    /// `zed: open performance profiler` action.
    pub performance_profiler: Option<PerformanceProfilerSettingsContent>,
}

/// Configuration for the performance profiler which collects timing data
/// for foreground and background executor tasks.
#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct PerformanceProfilerSettingsContent {
    /// Whether to collect timing data for foreground and background executor
    /// tasks. Enabling this may lead to increased memory usage, hence it's
    /// disabled by default for regular builds.
    ///
    /// Default: false
    pub enabled: Option<bool>,
}
