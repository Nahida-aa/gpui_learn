//! Git 与诊断状态色，对齐 zed `styles/status.rs`。
//!
//! 与 zed 的差异：zed 每个状态是三个独立字段（`error` / `error_background`
//! / `error_border`，靠 `#[derive(Refineable)]` 生成），我们收敛成一个
//! [`StatusColor`] 三色组——字段少一半，取值仍是 `status.error.base`
//! 这样的直白路径。

use gpui::Hsla;

/// 单个 Git/诊断状态的颜色组(base / background / border)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusColor {
    pub base: Hsla,
    pub background: Hsla,
    pub border: Hsla,
}

/// Git 与诊断状态色(对齐 zed `StatusColors`,每状态收敛为三色组)。
#[derive(Clone, Debug, PartialEq)]
pub struct StatusColors {
    pub conflict: StatusColor,
    pub created: StatusColor,
    pub deleted: StatusColor,
    pub error: StatusColor,
    pub hidden: StatusColor,
    pub ignored: StatusColor,
    pub info: StatusColor,
    pub modified: StatusColor,
    pub renamed: StatusColor,
    pub success: StatusColor,
    pub warning: StatusColor,
}
