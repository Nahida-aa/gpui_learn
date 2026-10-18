//! 矢量图（对齐 zed `crates/ui/src/components/image.rs`）。
//!
//! [`Vector`] 与 [`Icon`](aa_gpui_base::Icon) 的区别：图标有「标准尺寸」的概念
//! （`IconSize` 那几档），而矢量图是**按具体尺寸**展示的（logo、印章、插画）。
//! 资源在 `assets/images/*.svg`，由 assets 包统一 RustEmbed。
//!
//! 出处：zed `crates/ui/src/components/image.rs`（GPL-3.0-or-later）。

use std::sync::Arc;

use gpui::{
    App, IntoElement, Rems, RenderOnce, Size, Styled, Transformation, Window, svg,
};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString, IntoStaticStr};

use crate::prelude::*;
use crate::traits::Transformable;
use crate::rems_from_px;

#[derive(
    Debug, PartialEq, Eq, Copy, Clone, EnumIter, EnumString, IntoStaticStr, Serialize, Deserialize,
)]
#[strum(serialize_all = "snake_case")]
pub enum VectorName {
    BusinessStamp,
    VipStamp,
    Grid,
    ProTrialStamp,
    ProUserStamp,
    StudentStamp,
    ZedLogo,
    ZedXCopilot,
}

impl VectorName {
    /// Returns the path to this vector image.
    pub fn path(&self) -> Arc<str> {
        let file_stem: &'static str = self.into();
        format!("images/{file_stem}.svg").into()
    }
}

/// A vector image, such as an SVG.
///
/// A [`Vector`] is different from an [`Icon`](aa_gpui_base::Icon) in that it is
/// intended to be displayed at a specific size, or series of sizes, rather
/// than conforming to the standard size of an icon.
#[derive(IntoElement, RegisterComponent)]
pub struct Vector {
    path: Arc<str>,
    color: Color,
    size: Size<Rems>,
    transformation: Transformation,
}

impl Vector {
    /// Creates a new [`Vector`] image with the given [`VectorName`] and size.
    pub fn new(vector: VectorName, width: Rems, height: Rems) -> Self {
        Self {
            path: vector.path(),
            color: Color::default(),
            size: Size { width, height },
            transformation: Transformation::default(),
        }
    }

    /// Creates a new [`Vector`] image where the width and height are the same.
    pub fn square(vector: VectorName, size: Rems) -> Self {
        Self::new(vector, size, size)
    }

    /// Sets the vector color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the vector size.
    pub fn size(mut self, size: impl Into<Size<Rems>>) -> Self {
        let size = size.into();
        self.size = size;
        self
    }
}

impl Transformable for Vector {
    fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = transformation;
        self
    }
}

impl RenderOnce for Vector {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let width = self.size.width;
        let height = self.size.height;

        svg()
            // By default, prevent the SVG from stretching
            // to fill its container.
            .flex_none()
            .w(width)
            .h(height)
            .path(self.path)
            .text_color(self.color.color(cx))
            .with_transformation(self.transformation)
    }
}

impl Component for Vector {
    fn scope() -> ComponentScope {
        ComponentScope::Images
    }

    fn name() -> &'static str {
        "Vector"
    }

    fn description() -> &'static str {
        "A vector image component that can be displayed at specific sizes."
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> AnyElement {
        let size = rems_from_px(60_f32);

        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Basic Usage",
                    vec![
                        single_example(
                            "Default",
                            Vector::square(VectorName::ZedLogo, size).into_any_element(),
                        ),
                        single_example(
                            "Custom Size",
                            h_flex()
                                .h(rems_from_px(120_f32))
                                .justify_center()
                                .child(Vector::new(
                                    VectorName::ZedLogo,
                                    rems_from_px(120_f32),
                                    rems_from_px(200_f32),
                                ))
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Colored",
                    vec![
                        single_example(
                            "Accent Color",
                            Vector::square(VectorName::ZedLogo, size)
                                .color(Color::Accent)
                                .into_any_element(),
                        ),
                        single_example(
                            "Error Color",
                            Vector::square(VectorName::ZedLogo, size)
                                .color(Color::Error)
                                .into_any_element(),
                        ),
                    ],
                ),
                example_group_with_title(
                    "Different Vectors",
                    vec![single_example(
                        "Zed X Copilot",
                        Vector::square(VectorName::ZedXCopilot, rems_from_px(100_f32))
                            .into_any_element(),
                    )],
                ),
            ])
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_path() {
        assert_eq!(VectorName::ZedLogo.path().as_ref(), "images/zed_logo.svg");
    }

    /// 每个 `VectorName` 都必须能映射到 `assets/images/` 下的真实文件，
    /// 否则预览里会静默渲染成空白。
    #[test]
    fn every_vector_name_has_a_path() {
        use strum::IntoEnumIterator;

        let names: Vec<String> = VectorName::iter().map(|v| v.path().to_string()).collect();
        for expected in [
            "images/business_stamp.svg",
            "images/vip_stamp.svg",
            "images/grid.svg",
            "images/pro_trial_stamp.svg",
            "images/pro_user_stamp.svg",
            "images/student_stamp.svg",
            "images/zed_logo.svg",
            "images/zed_x_copilot.svg",
        ] {
            assert!(names.iter().any(|n| n == expected), "缺少 {expected}");
        }
        assert_eq!(names.len(), 8);
    }
}
