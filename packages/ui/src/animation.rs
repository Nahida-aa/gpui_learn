//! 常用入场动画（对齐 zed `crates/ui/src/styles/animation.rs`，导出为顶级
//! `crate::animation` 模块 —— 与 zed 的 `ui::animation::DefaultAnimations` 同路径）。
//!
//! 用法：
//!
//! ```ignore
//! use aa_gpui_kit_ui::animation::DefaultAnimations;
//!
//! div()
//!     .id("panel")
//!     .child(content)
//!     .animate_in_from_bottom(true)  // 从下方滑入 + 淡入
//! ```

use std::time::Duration;

use gpui::{AnimationElement, AnimationExt, Styled, ease_out_quint};

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationDuration {
    Instant = 50,
    Fast = 150,
    Slow = 300,
}

impl AnimationDuration {
    pub fn duration(&self) -> Duration {
        Duration::from_millis(*self as u64)
    }
}

impl From<AnimationDuration> for Duration {
    fn from(duration: AnimationDuration) -> Self {
        duration.duration()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationDirection {
    FromBottom,
    FromLeft,
    FromRight,
    FromTop,
}

/// 给任意 `Styled + Element` 加「从某方向滑入」的入场动画。
///
/// blanket impl 挂在 gpui 的 `Styled + Element` 上，所以 `div()` 之类直接可用。
pub trait DefaultAnimations: Styled + Sized + Element {
    fn animate_in(
        self,
        animation_type: AnimationDirection,
        fade_in: bool,
    ) -> AnimationElement<Self> {
        let animation_name = match animation_type {
            AnimationDirection::FromBottom => "animate_from_bottom",
            AnimationDirection::FromLeft => "animate_from_left",
            AnimationDirection::FromRight => "animate_from_right",
            AnimationDirection::FromTop => "animate_from_top",
        };

        let animation_id = self.id().map_or_else(
            || ElementId::from(animation_name),
            |id| (id, animation_name).into(),
        );

        self.with_animation(
            animation_id,
            gpui::Animation::new(AnimationDuration::Fast.into()).with_easing(ease_out_quint()),
            move |mut this, delta| {
                let start_opacity = 0.4;
                let start_pos = 0.0;
                let end_pos = 40.0;

                if fade_in {
                    this = this.opacity(start_opacity + delta * (1.0 - start_opacity));
                }

                match animation_type {
                    AnimationDirection::FromBottom => {
                        this.bottom(px(start_pos + delta * (end_pos - start_pos)))
                    }
                    AnimationDirection::FromLeft => {
                        this.left(px(start_pos + delta * (end_pos - start_pos)))
                    }
                    AnimationDirection::FromRight => {
                        this.right(px(start_pos + delta * (end_pos - start_pos)))
                    }
                    AnimationDirection::FromTop => {
                        this.top(px(start_pos + delta * (end_pos - start_pos)))
                    }
                }
            },
        )
    }

    fn animate_in_from_bottom(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromBottom, fade)
    }

    fn animate_in_from_left(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromLeft, fade)
    }

    fn animate_in_from_right(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromRight, fade)
    }

    fn animate_in_from_top(self, fade: bool) -> AnimationElement<Self> {
        self.animate_in(AnimationDirection::FromTop, fade)
    }
}

impl<E: Styled + Element> DefaultAnimations for E {}
