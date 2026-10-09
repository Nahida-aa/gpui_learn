//! Zed 专有 release channel 薄壳层。
//!
//! 通用逻辑（频道枚举、版本管理、拼接函数）全在 `aa_release_channel`，
//! 这里只做两件事：
//!   1. `pub use aa_release_channel::*` 让 Zed 消费方零改动
//!   2. 几个把 Zed 专有硬编码（brand / bundle prefix / docs URL）传给
//!      aa_ 通用方法的薄壳函数
//!
//! aacode 不依赖此 crate——直接依赖 `aa_release_channel`。

#![deny(missing_docs)]

pub use aa_release_channel::*;

const ZED_DOCS_URL: &str = "https://zed.dev/docs";
const ZED_BRAND: &str = "Zed";
const ZED_BUNDLE_PREFIX: &str = "dev.zed.Zed";
const ZED_APP_IDENTIFIER_PREFIX: &str = "Zed-Editor";

/// 编译期回退时读 Zed 专有 env var（aa_ 已做了 ZED_RELEASE_CHANNEL → RELEASE_CHANNEL 回退）。
fn compile_time_release_channel_name() -> String {
    option_env!("ZED_RELEASE_CHANNEL").unwrap_or("dev").trim().to_string()
}

// ---- Zed 专有薄壳函数 ----
//
// 这些函数只是把 Zed 硬编码传给 aa_ 的通用方法，方便 Zed 消费方零改动。
// aacode 不走这里——直接调 aa_ 的方法 + 传自己的品牌值。

/// Zed display name：`"Zed"` + channel suffix。
pub fn display_name(cx: &gpui::App) -> String {
    ReleaseChannel::try_global(cx)
        .unwrap_or(*RELEASE_CHANNEL)
        .display_name(ZED_BRAND)
}

/// Zed docs URL。
pub fn docs_url(slug: &str, cx: &gpui::App) -> String {
    aa_release_channel::docs_url(ZED_DOCS_URL, slug, cx)
}

/// Zed Wayland app_id / macOS bundle identifier。
pub fn app_id(cx: &gpui::App) -> String {
    ReleaseChannel::try_global(cx)
        .unwrap_or(*RELEASE_CHANNEL)
        .app_id(ZED_BUNDLE_PREFIX)
}

/// Windows app identifier。
#[cfg(target_os = "windows")]
pub fn app_identifier(cx: &gpui::App) -> String {
    let channel = ReleaseChannel::try_global(cx)
        .unwrap_or(*RELEASE_CHANNEL);
    let suffix = match channel {
        ReleaseChannel::Dev => "Dev",
        ReleaseChannel::Nightly => "Nightly",
        ReleaseChannel::Preview => "Preview",
        ReleaseChannel::Stable => "Stable",
    };
    format!("{ZED_APP_IDENTIFIER_PREFIX}-{suffix}")
}
