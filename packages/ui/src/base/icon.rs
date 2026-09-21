//! base/icon：通用图标控件。
//!
//! `IconName` 枚举由 `aa-gpui-kit-assets` crate 的 build.rs 自动生成
//! （扫描 assets/icons/*.svg）。加图标 = 扔 SVG 文件即可，零手动维护。
//!
//! `Icon` 是一次渲染控件，持有 `IconName` + 尺寸 + 颜色 + 旋转，
//! 渲染时通过 `IconName::path()` 拿到 RustEmbed 路径。

use gpui::{
    Hsla, IntoElement, Pixels, Radians, Rems, RenderOnce, Transformation, Window, prelude::*, px,
    svg,
};

pub use aa_gpui_kit_assets::IconName;

use crate::styles::units::rems_from_px;

/// 图标尺寸的语义档位（对齐 zed `crates/ui/src/components/icon.rs:54`）。
///
/// `KeyIcon`（`KeyBinding` 的图标键）用它取默认尺寸；`Custom(Rems)` 让调用方
/// 传入任意 rem 值。
///
/// 与 zed 的差异：zed 另有 `square_components()` / `square()` 算「含 padding 的
/// 正方形边长」，依赖 `ui_density(cx)`。我们没有设置系统（密度需显式传参），
/// 且目前没有调用方需要它，故先不移植 —— 需要时再补。
#[derive(Default, PartialEq, Copy, Clone)]
pub enum IconSize {
    /// 10px
    Indicator,
    /// 12px
    XSmall,
    /// 14px
    Small,
    #[default]
    /// 16px
    Medium,
    /// 48px
    XLarge,
    Custom(Rems),
}

impl IconSize {
    pub fn rems(self) -> Rems {
        match self {
            IconSize::Indicator => rems_from_px(10_f32),
            IconSize::XSmall => rems_from_px(12_f32),
            IconSize::Small => rems_from_px(14_f32),
            IconSize::Medium => rems_from_px(16_f32),
            IconSize::XLarge => rems_from_px(48_f32),
            IconSize::Custom(size) => size,
        }
    }
}

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

    /// 当前边长。供同 crate 的复合件布局用（如 `DecoratedIcon` 要让容器
    /// 与图标等大）。不对外暴露——外部直接调 [`Self::size`] 设置即可。
    pub(crate) fn size_px(&self) -> Pixels {
        self.size
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
