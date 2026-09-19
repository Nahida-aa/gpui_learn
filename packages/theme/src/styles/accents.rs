//! 强调色组，对齐 zed `styles/accents.rs`。
//!
//! zed 用 `blue().dark().step_9()` 这类色板函数生成 13 色；gpui 0.2 只导出
//! blue/green/yellow/red 四个色板函数，其余用 `hsla` 显式补齐。

use std::sync::Arc;

use gpui::Hsla;

/// 强调色组(对齐 zed `AccentColors`):用于缩进参考线等按行轮换的颜色。
#[derive(Clone, Debug, PartialEq)]
pub struct AccentColors(pub Arc<[Hsla]>);

impl Default for AccentColors {
    fn default() -> Self {
        // gpui 0.2 的色板函数只有 blue/green/yellow/red;其余用 hsla 补齐
        AccentColors(
            [
                gpui::blue(),
                gpui::green(),
                gpui::yellow(),
                gpui::red(),
                gpui::hsla(0.45, 0.6, 0.6, 1.),
                gpui::hsla(0.55, 0.6, 0.6, 1.),
                gpui::hsla(0.8, 0.6, 0.65, 1.),
                gpui::hsla(0.1, 0.7, 0.6, 1.),
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
        )
    }
}
