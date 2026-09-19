use gpui::Transformation;

/// A trait for components that can be transformed.
///
/// 对齐 zed `crates/ui/src/traits/transformable.rs`。
pub trait Transformable {
    /// Sets the transformation for the element.
    fn transform(self, transformation: Transformation) -> Self;
}
