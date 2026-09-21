use gpui::Transformation;

/// A trait for components that can be transformed.
///
/// 对齐 zed `crates/ui/src/traits/transformable.rs`。
///
/// **为什么定义在 `aa_gpui_base` 而不是 `ui`**：`Icon` 住在 base，而 trait 与
/// 类型的 impl 受孤儿规则约束 —— trait 必须在类型所在 crate 才能为它实现。
/// `ui` 通过 `pub use aa_gpui_base::Transformable` 原样转出，外部感知不到差别。
pub trait Transformable {
    /// Sets the transformation for the element.
    fn transform(self, transformation: Transformation) -> Self;
}
