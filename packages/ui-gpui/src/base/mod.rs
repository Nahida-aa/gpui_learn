//! base：通用基础控件层。
//!
//! - [`geometry`]：通用数学（刻度换算 `Scale`、像素↔值换算、step 取整），
//!   不绑定某个具体控件，未来其他组件（button/input 等）也能复用。
//! - [`slider`]：滑块控件（单值 / Range 双 thumb）。
//! - [`icon`]：图标控件（`Icon` + 内置 [`icon::IconName`] 枚举），渲染内嵌 SVG。
//! - [`input`]：单行文本输入框（`Entity<InputState>`，含 IME 组字与选区）。
//! - [`button`]：按钮（一次性元素，无持久状态）。

pub mod button;
pub mod geometry;
pub mod icon;
pub mod input;
pub mod slider;
