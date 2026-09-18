//! base/icon：通用图标控件。
//!
//! `IconName` 枚举由 `aa-gpui-kit-assets` crate 的 build.rs 自动生成
//! （扫描 assets/icons/*.svg）。加图标 = 扔 SVG 文件即可，零手动维护。
//!
//! `Icon` 是一次渲染控件，持有 `IconName` + 尺寸 + 颜色 + 旋转，
//! 渲染时通过 `IconName::path()` 拿到 RustEmbed 路径。

use gpui::{
    Hsla, IntoElement, Pixels, Radians, RenderOnce, Transformation, Window, prelude::*, px, svg,
};

pub use aa_gpui_kit_assets::IconName;

/// 图标控件：渲染一个内嵌 SVG。
#[derive(Clone, IntoElement)]
pub struct Icon {
    name: IconName,
    size: Pixels,
    color: Option<Hsla>,
    rotation: Option<Radians>,
}

impl Icon {
    /// 用内置图标名构造。
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: px(16.0),
            color: None,
            rotation: None,
        }
    }

    /// 指定边长（正方形，单位 px）。
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// 指定颜色；不指定则取当前 `text_style().color`。
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// 旋转角度（弧度）。
    pub fn rotate(mut self, radians: impl Into<Radians>) -> Self {
        self.rotation = Some(radians.into());
        self
    }
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            name: IconName::Check,
            size: px(16.0),
            color: None,
            rotation: None,
        }
    }
}

impl From<IconName> for Icon {
    fn from(name: IconName) -> Self {
        Self::new(name)
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        let color = self.color.unwrap_or_else(|| window.text_style().color);
        svg()
            .path(self.name.path())
            .size(self.size)
            .flex_none()
            .text_color(color)
            .when_some(self.rotation, |this, rotation| {
                this.with_transformation(Transformation::rotate(rotation))
            })
    }
}
