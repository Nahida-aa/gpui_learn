//! UI 密度,对齐 zed `crates/theme/src/ui_density.rs`(精简版)。
//!
//! 控制界面元素的疏密:紧凑(间距收紧、元素变小)↔ 舒适(间距放宽)。
//! 目前只有数据结构与换算,还没有消费方——等组件开始按密度调整间距时
//! 再从 [`ThemeSettingsProvider::ui_density`](crate::ThemeSettingsProvider)
//! 取当前值。

use serde::{Deserialize, Serialize};

/// UI 密度。
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    /// 更紧凑:间距收紧、元素更小。
    #[serde(alias = "compact")]
    Compact,
    /// 默认密度。
    #[default]
    #[serde(alias = "default")]
    Default,
    /// 更宽松:间距更大。
    #[serde(alias = "comfortable")]
    Comfortable,
}

impl UiDensity {
    /// 间距比例系数:组件把基准间距乘以它得到实际间距。
    pub fn spacing_ratio(self) -> f32 {
        match self {
            UiDensity::Compact => 0.75,
            UiDensity::Default => 1.0,
            UiDensity::Comfortable => 1.25,
        }
    }
}

impl From<String> for UiDensity {
    fn from(value: String) -> Self {
        match value.as_str() {
            "compact" => Self::Compact,
            "comfortable" => Self::Comfortable,
            _ => Self::default(),
        }
    }
}

impl From<UiDensity> for String {
    fn from(value: UiDensity) -> Self {
        match value {
            UiDensity::Compact => "compact".to_string(),
            UiDensity::Default => "default".to_string(),
            UiDensity::Comfortable => "comfortable".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_ratio_orders_by_density() {
        assert!(UiDensity::Compact.spacing_ratio() < UiDensity::Default.spacing_ratio());
        assert!(UiDensity::Default.spacing_ratio() < UiDensity::Comfortable.spacing_ratio());
    }

    #[test]
    fn round_trips_through_string() {
        for density in [
            UiDensity::Compact,
            UiDensity::Default,
            UiDensity::Comfortable,
        ] {
            let text: String = density.into();
            assert_eq!(UiDensity::from(text), density, "字符串往返应无损");
        }
        // 未知值回退默认,不 panic
        assert_eq!(UiDensity::from("nonsense".to_string()), UiDensity::Default);
    }
}
