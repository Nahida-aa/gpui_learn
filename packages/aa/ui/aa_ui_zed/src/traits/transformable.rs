use gpui::Transformation;

/// A trait for components that can be transformed.
///
/// 对齐 zed `crates/ui/src/traits/transformable.rs`。
///
/// **为什么定义在 `ui`**：`Icon` 住在 `ui`（zed 也是），而 trait 与类型的
/// impl 受孤儿规则约束 —— trait 必须在类型所在 crate 才能为它实现。
pub trait Transformable {
    /// Sets the transformation for the element.
    fn transform(self, transformation: Transformation) -> Self;
}
