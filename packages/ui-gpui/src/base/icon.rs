//! base/icon：通用图标控件。
//!
//! 一个轻量的 [`Icon`] 包装 gpui 的 `svg()`，通过 app 的 `AssetSource`
//! （见 [`crate::assets`]）按路径加载内嵌 SVG。
//!
//! 设计要点：
//! - [`IconName`] 枚举集中管理 ui-gpui 内置图标，每个变体映射到 `assets/icons/*.svg`
//!   的相对路径。新增图标只需在此加变体 + 放对应 svg 文件。
//! - SVG 以单色渲染（gpui 的 svg 渲染器用 `text_color` 上色），故图标 svg 应
//!   用 `fill="currentColor"` 的形状，颜色由 [`Icon::color`] 控制。
//! - 控件本身不内嵌资源，资源由 app 的 asset source 提供；使用方需把
//!   `ui_gpui::assets::Assets` 组合进 `with_assets(...)`。

use gpui::{
    App, Hsla, IntoElement, Pixels, Radians, RenderOnce, Transformation, Window, prelude::*, px,
    svg,
};

/// ui-gpui 内置图标名。
///
/// 每个变体对应 [`crate::assets`] 内嵌的一份 `assets/icons/*.svg`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconName {
    /// 播放（实心三角形）。
    Play,
    /// 暂停（双竖条）。
    Pause,
    /// 音量开（喇叭 + 声波）。
    VolumeOn,
    /// 音量关（喇叭 + 斜杠）。
    VolumeOff,
    /// 更多（垂直三点）。
    More,
}

impl IconName {
    /// 返回该图标在内嵌资源中的相对路径（供 `svg().path(...)` 使用）。
    pub fn path(self) -> &'static str {
        match self {
            IconName::Play => "icons/play_filled.svg",
            IconName::Pause => "icons/debug_pause.svg",
            IconName::VolumeOn => "icons/audio_on.svg",
            IconName::VolumeOff => "icons/audio_off.svg",
            IconName::More => "icons/ellipsis_vertical.svg",
        }
    }
}

impl From<IconName> for Icon {
    fn from(name: IconName) -> Self {
        Icon::new(name)
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

impl Default for Icon {
    fn default() -> Self {
        Self {
            name: IconName::Play,
            size: px(16.0),
            color: None,
            rotation: None,
        }
    }
}

impl Icon {
    /// 用内置图标名构造。
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            ..Default::default()
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

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
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
