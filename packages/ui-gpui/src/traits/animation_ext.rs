use std::time::Duration;

use gpui::{Animation, AnimationElement, AnimationExt, ElementId, Transformation, percentage};

use super::transformable::Transformable;

/// An extension trait for adding common animations to animatable components.
///
/// 对齐 zed `crates/ui/src/traits/animation_ext.rs`。blanket impl 挂在
/// gpui 的 [`AnimationExt`] 上 —— 任何可动画元素自动获得，组件无需自己 impl；
/// 旋转的具体应用通过 [`Transformable`] 完成。
pub trait CommonAnimationExt: AnimationExt {
    /// Render this component as rotating over the given duration.
    ///
    /// NOTE: This method uses the location of the caller to generate an ID for this state.
    ///       If this is not sufficient to identify your state (e.g. you're rendering a list item),
    ///       you can provide a custom ElementID using the `use_keyed_rotate_animation` method.
    #[track_caller]
    fn with_rotate_animation(self, duration: u64) -> AnimationElement<Self>
    where
        Self: Transformable + Sized,
    {
        self.with_keyed_rotate_animation(
            ElementId::CodeLocation(*std::panic::Location::caller()),
            duration,
        )
    }

    /// Render this component as rotating with the given element ID over the given duration.
    fn with_keyed_rotate_animation(
        self,
        id: impl Into<ElementId>,
        duration: u64,
    ) -> AnimationElement<Self>
    where
        Self: Transformable + Sized,
    {
        self.with_animation(
            id,
            Animation::new(Duration::from_secs(duration)).repeat_synced(),
            |component, delta| component.transform(Transformation::rotate(percentage(delta))),
        )
    }
}

impl<T: AnimationExt> CommonAnimationExt for T {}
