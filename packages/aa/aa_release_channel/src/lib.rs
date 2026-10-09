//! Provides constructs for the Zed app version and release channel.

#![deny(missing_docs)]

use std::{env, str::FromStr, sync::LazyLock};

use gpui::{App, Global};
use semver::Version;

/// stable | dev | nightly | preview
pub static RELEASE_CHANNEL_NAME: LazyLock<String> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        env::var("ZED_RELEASE_CHANNEL")
            .or_else(|_| env::var("RELEASE_CHANNEL"))
            .unwrap_or_else(|_| compile_time_release_channel_name())
    } else {
        compile_time_release_channel_name()
    }
});

/// 编译期回退的频道名。
///
/// 【有意偏离 Zed】Zed 在这里用 `#[cfg]` 在两条路之间二选一：
/// 构建期有 `ZED_RELEASE_CHANNEL` 就走 `env!`（且运行期缺它会 panic），
/// 否则走 `include_str!("../../zed/RELEASE_CHANNEL")` 读仓库里那个文件。
/// 它需要文件是因为 crane/Nix 会把每个 crate 单独 vendor 到独立路径编译，
/// `include_str!` 的相对路径在那里不存在（见 zed 原注释）。
///
/// aacode 不用 crane，且直接规定「设置环境变量，否则默认 dev」，故两条路合并成
/// `option_env!`：
///   - 有 env → 用它（等价 zed 的 env! 分支，但不会 panic）
///   - 无 env → "dev"
///
/// 顺带取消了 `__do_not_set_zed_release_channel` 这个 cfg：它原本只为让两个分支
/// 互斥而存在，现在只有一个函数，cfg 没有意义了（build.rs 里对应的 rustc-cfg 也一并删）。
///
/// 代价：没有 env 时编译产物固定是 dev。若要出非 dev 的正式版，必须构建期注入
/// ZED_RELEASE_CHANNEL —— `just install` 就是这么做的（默认 stable）。
fn compile_time_release_channel_name() -> String {
    option_env!("ZED_RELEASE_CHANNEL")
        .or_else(|| option_env!("RELEASE_CHANNEL"))
        .unwrap_or("dev")
        .trim()
        .to_string()
}

#[doc(hidden)]
pub static RELEASE_CHANNEL: LazyLock<ReleaseChannel> =
    LazyLock::new(|| match ReleaseChannel::from_str(&RELEASE_CHANNEL_NAME) {
        Ok(channel) => channel,
        _ => panic!("invalid release channel {}", *RELEASE_CHANNEL_NAME),
    });

/// The Git commit SHA that Zed was built at.
#[derive(Clone, Eq, Debug, PartialEq)]
pub struct AppCommitSha(String);

struct GlobalAppCommitSha(AppCommitSha);

impl Global for GlobalAppCommitSha {}

impl AppCommitSha {
    /// Creates a new [`AppCommitSha`].
    pub fn new(sha: String) -> Self {
        AppCommitSha(sha)
    }

    /// Returns the global [`AppCommitSha`], if one is set.
    pub fn try_global(cx: &App) -> Option<AppCommitSha> {
        cx.try_global::<GlobalAppCommitSha>()
            .map(|sha| sha.0.clone())
    }

    /// Sets the global [`AppCommitSha`].
    pub fn set_global(sha: AppCommitSha, cx: &mut App) {
        cx.set_global(GlobalAppCommitSha(sha))
    }

    /// Returns the full commit SHA.
    pub fn full(&self) -> String {
        self.0.to_string()
    }

    /// Returns the short (7 character) commit SHA.
    pub fn short(&self) -> String {
        self.0.chars().take(7).collect()
    }
}

struct GlobalAppVersion(Version);

impl Global for GlobalAppVersion {}

/// The version of Zed.
pub struct AppVersion;

impl AppVersion {
    /// Load the app version from env.
    pub fn load(
        pkg_version: &str,
        build_id: Option<&str>,
        commit_sha: Option<AppCommitSha>,
    ) -> Version {
        let mut version: Version = if let Ok(from_env) = env::var("APP_VERSION") {
            from_env.parse().expect("invalid APP_VERSION")
        } else {
            pkg_version.parse().expect("invalid version in Cargo.toml")
        };
        let mut pre = String::from(RELEASE_CHANNEL.dev_name());

        if let Some(build_id) = build_id {
            pre.push('.');
            pre.push_str(&build_id);
        }

        if let Some(sha) = commit_sha {
            pre.push('.');
            pre.push_str(&sha.0);
        }
        if let Ok(build) = semver::BuildMetadata::new(&pre) {
            version.build = build;
        }

        version
    }

    /// Returns the global version number.
    pub fn global(cx: &App) -> Version {
        if cx.has_global::<GlobalAppVersion>() {
            cx.global::<GlobalAppVersion>().0.clone()
        } else {
            Version::new(0, 0, 0)
        }
    }
}

/// A Zed release channel.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ReleaseChannel {
    /// The development release channel.
    ///
    /// Used for local debug builds of Zed.
    #[default]
    Dev,

    /// The Nightly release channel.
    Nightly,

    /// The Preview release channel.
    Preview,

    /// The Stable release channel.
    Stable,
}

struct GlobalReleaseChannel(ReleaseChannel);

impl Global for GlobalReleaseChannel {}

/// Initializes the release channel.
pub fn init(app_version: Version, cx: &mut App) {
    cx.set_global(GlobalAppVersion(app_version));
    cx.set_global(GlobalReleaseChannel(*RELEASE_CHANNEL))
}

/// Initializes the release channel for tests that rely on fake release channel.
pub fn init_test(app_version: Version, release_channel: ReleaseChannel, cx: &mut App) {
    cx.set_global(GlobalAppVersion(app_version));
    cx.set_global(GlobalReleaseChannel(release_channel))
}

/// Returns the docs URL for the current release channel given `base_url`
/// (e.g. `"https://zed.dev/docs"`) and `slug`.
pub fn docs_url(base_url: &str, slug: &str, cx: &App) -> String {
    ReleaseChannel::try_global(cx)
        .unwrap_or(*RELEASE_CHANNEL)
        .docs_url(base_url, slug)
}

/// Windows app identifier：`{prefix}-{channel_suffix}`，如 `"Zed-Editor-Dev"`。
///
/// 读编译期确定的 [`RELEASE_CHANNEL`]，不需要 `cx`。
/// aacode 消费方传自己的前缀；Zed 传 `"Zed-Editor"`。
#[cfg(target_os = "windows")]
pub fn app_identifier(prefix: &str) -> String {
    let suffix = match *RELEASE_CHANNEL {
        ReleaseChannel::Dev => "Dev",
        ReleaseChannel::Nightly => "Nightly",
        ReleaseChannel::Preview => "Preview",
        ReleaseChannel::Stable => "Stable",
    };
    format!("{prefix}-{suffix}")
}

impl ReleaseChannel {
    /// All release channels.
    pub const ALL: [ReleaseChannel; 4] = [
        ReleaseChannel::Dev,
        ReleaseChannel::Nightly,
        ReleaseChannel::Preview,
        ReleaseChannel::Stable,
    ];

    /// Returns the global [`ReleaseChannel`].
    pub fn global(cx: &App) -> Self {
        cx.global::<GlobalReleaseChannel>().0
    }

    /// Returns the global [`ReleaseChannel`], if one is set.
    pub fn try_global(cx: &App) -> Option<Self> {
        cx.try_global::<GlobalReleaseChannel>()
            .map(|channel| channel.0)
    }

    /// Returns whether we want to poll for updates for this [`ReleaseChannel`]
    pub fn poll_for_updates(&self) -> bool {
        !matches!(self, ReleaseChannel::Dev)
    }

    /// Returns the display name for this [`ReleaseChannel`].
    ///
    /// `brand` is the app name prefix, e.g. `"Zed"` → `"Zed Dev"`.
    /// Stable 不拼 channel，直接返回 brand。
    pub fn display_name(&self, brand: &str) -> String {
        let channel = match self {
            ReleaseChannel::Dev => " Dev",
            ReleaseChannel::Nightly => " Nightly",
            ReleaseChannel::Preview => " Preview",
            ReleaseChannel::Stable => "",
        };
        format!("{brand}{channel}")
    }

    /// Returns the programmatic name for this [`ReleaseChannel`].
    pub fn dev_name(&self) -> &'static str {
        match self {
            ReleaseChannel::Dev => "dev",
            ReleaseChannel::Nightly => "nightly",
            ReleaseChannel::Preview => "preview",
            ReleaseChannel::Stable => "stable",
        }
    }

    /// Returns the application ID that's used by Wayland as application ID
    /// and WM_CLASS on X11.
    /// This also has to match the bundle identifier for Zed on macOS.
    ///
    /// `app_id` e.g. `"dev.zed.Zed"` → `"dev.zed.Zed-Dev"` / `"dev.zed.Zed"`.
    /// Stable 不拼 channel，直接返回 `app_id`。
    pub fn app_id(&self, app_id: &str) -> String {
        let channel_suffix = match self {
            ReleaseChannel::Dev => "-Dev",
            ReleaseChannel::Nightly => "-Nightly",
            ReleaseChannel::Preview => "-Preview",
            ReleaseChannel::Stable => "",
        };
        format!("{app_id}{channel_suffix}")
    }

    /// Returns the query parameter for this [`ReleaseChannel`].
    pub fn release_query_param(&self) -> Option<&'static str> {
        match self {
            Self::Dev => None,
            Self::Nightly => Some("nightly=1"),
            Self::Preview => Some("preview=1"),
            Self::Stable => None,
        }
    }

    /// Returns the docs URL for this [`ReleaseChannel`] given `base_url`
    /// (e.g. `"https://zed.dev/docs"`) and `slug`.
    pub fn docs_url(&self, base_url: &str, slug: &str) -> String {
        let channel_path_segment = match self {
            Self::Dev | Self::Nightly => Some("nightly"),
            Self::Preview => Some("preview"),
            Self::Stable => None,
        };
        match channel_path_segment {
            Some(seg) if slug.is_empty() => format!("{base_url}/{seg}"),
            Some(seg) => format!("{base_url}/{seg}/{slug}"),
            None if slug.is_empty() => base_url.to_string(),
            None => format!("{base_url}/{slug}"),
        }
    }
}

/// Error indicating that release channel string does not match any known release channel names.
#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub struct InvalidReleaseChannel;

impl FromStr for ReleaseChannel {
    type Err = InvalidReleaseChannel;

    fn from_str(channel: &str) -> Result<Self, Self::Err> {
        Ok(match channel {
            "dev" => ReleaseChannel::Dev,
            "nightly" => ReleaseChannel::Nightly,
            "preview" => ReleaseChannel::Preview,
            "stable" => ReleaseChannel::Stable,
            _ => return Err(InvalidReleaseChannel),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ReleaseChannel;

    #[test]
    fn test_docs_url_for_release_channel() {
        let base = "https://zed.dev/docs";
        assert_eq!(
            ReleaseChannel::Dev.docs_url(base, "settings"),
            "https://zed.dev/docs/nightly/settings"
        );
        assert_eq!(
            ReleaseChannel::Nightly.docs_url(base, "settings"),
            "https://zed.dev/docs/nightly/settings"
        );
        assert_eq!(
            ReleaseChannel::Preview.docs_url(base, "settings"),
            "https://zed.dev/docs/preview/settings"
        );
        assert_eq!(
            ReleaseChannel::Stable.docs_url(base, "settings"),
            "https://zed.dev/docs/settings"
        );
    }
}
