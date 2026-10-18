//! 平台相关的常量与尺寸（对齐 zed `crates/ui/src/utils/constants.rs`）。
//!
//! 这里都是「跟随宿主平台、不跟随主题」的值：macOS 红绿灯的内边距、
//! 标题栏高度。UI 尺寸类（字号 / 间距 / 圆角）不要放这里，去 `styles::units`。

use gpui::{Pixels, Window, px};

pub const MACOS_SDK_26_OR_LATER: bool = cfg!(macos_sdk_26_or_later);

// Use pixels here instead of a rem-based size because the macOS traffic
// lights are a static size, and don't scale with the rest of the UI.
//
// Magic number: There is one extra pixel of padding on the left side due to
// the 1px border around the window on macOS apps.
pub const TRAFFIC_LIGHT_PADDING: f32 = if MACOS_SDK_26_OR_LATER { 78. } else { 71. };

/// Returns the platform-appropriate title bar height.
///
/// On Windows, this returns a fixed height of 32px.
/// On other platforms, it scales with the window's rem size (1.75x) with a minimum of 34px.
#[cfg(not(target_os = "windows"))]
pub fn platform_title_bar_height(window: &Window) -> Pixels {
    (1.75 * window.rem_size()).max(px(34.))
}

#[cfg(target_os = "windows")]
pub fn platform_title_bar_height(_window: &Window) -> Pixels {
    // todo(windows) instead of hard coded size report the actual size to the Windows platform API
    px(32.)
}
