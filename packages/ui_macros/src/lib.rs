//! proc-macro 宏包（对齐 zed `crates/ui_macros` 的精简版）。
//!
//! 当前只含 [`derive_dynamic_spacing!`]：生成 `DynamicSpacing` 密度
//! 间距枚举（消费方在 `aa-gpui-kit-ui` 的 `styles/spacing`）。
//!
//! 与 zed 的差异：zed 另有 `#[derive(RegisterComponent)]`（把组件登记进
//! 其组件示例库，深度依赖 `component` crate 与 `inventory`），我们尚未
//! 建立该基建，故未实现；将来建组件示例库时再补。
//!
//! 授权说明：zed 的 `ui_macros` 为 GPL-3.0；本包是**等价功能的原创实现**
//! （宏的输入/输出契约与 zed 一致，便于调用方代码对齐），实现细节独立撰写。

mod dynamic_spacing;

use proc_macro::TokenStream;

/// 生成 zed 风格的密度间距枚举（`DynamicSpacing`）。
///
/// 输入为逗号分隔的档位列表，每项两种形式：
///
/// - **单值 `n`**：按标准公式展开三档 —— Compact `(n-4).max(0)`、
///   Default `n`、Comfortable `n+4`；
/// - **元组 `(a, b, c)`**：三档直接取值。
///
/// 变体名取 `BaseXX`（XX = Default 档像素值，两位补零）。
///
/// ```ignore
/// derive_dynamic_spacing![
///     (1, 2, 4),  // Compact: 1px | Default: 2px | Comfortable: 4px
///     24,         // Compact: 20px | Default: 24px | Comfortable: 28px
/// ];
/// ```
///
/// 生成项：枚举本体 + `spacing_ratio(density)` + `rems(density)` +
/// `px(density, rem_size)`。与 zed 的差异：zed 从 `theme_settings` 读
/// 当前密度与字体大小；我们尚无设置系统，密度由调用方显式传入。
#[proc_macro]
pub fn derive_dynamic_spacing(input: TokenStream) -> TokenStream {
    dynamic_spacing::expand(input)
}
