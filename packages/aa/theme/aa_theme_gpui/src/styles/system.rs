//! 系统层颜色，对齐 zed `styles/system.rs`（精简子集）。
//!
//! zed 在这里放 macOS 红绿灯等系统语义色；我们目前只需要透明色，
//! 后续要补系统色时按 zed 同名加字段。

use gpui::Hsla;

/// 系统层颜色(对齐 zed `SystemColors` 的精简子集)。
#[derive(Clone, Debug, PartialEq)]
pub struct SystemColors {
    pub transparent: Hsla,
}

impl Default for SystemColors {
    fn default() -> Self {
        SystemColors {
            transparent: gpui::hsla(0., 0., 0., 0.),
        }
    }
}
