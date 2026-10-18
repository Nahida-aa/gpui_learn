use std::borrow::Cow;

// 只被 dev 分支的 `__fs_embed_*` 用到；release（编译期内嵌）下它们不存在。
#[cfg(all(debug_assertions, not(feature = "debug-embed")))]
use crate::dev::dev_repo_root;

/// Get an embedded file as a string.
pub fn asset_str<A: rust_embed::RustEmbed>(path: &str) -> Cow<'static, str> {
    match A::get(path).expect(path).data {
        Cow::Borrowed(bytes) => Cow::Borrowed(std::str::from_utf8(bytes).unwrap()),
        Cow::Owned(bytes) => Cow::Owned(String::from_utf8(bytes).unwrap()),
    }
}

/// Re-exports that back [`fs_embed!`] so a caller only needs to depend on `util`.
#[doc(hidden)]
pub mod __rust_embed {
    pub use rust_embed::{EmbeddedFile, Filenames, Metadata, RustEmbed, flate, utils};
}

#[cfg(all(debug_assertions, not(feature = "debug-embed")))]
#[doc(hidden)]
pub fn __fs_embed_iter(
    root_relative: &str,
    includes: &[&str],
    excludes: &[&str],
) -> impl Iterator<Item = std::borrow::Cow<'static, str>> + 'static {
    fs_embed_file_names(root_relative, includes, excludes)
        .into_iter()
        .map(std::borrow::Cow::Owned)
}

#[cfg(all(debug_assertions, not(feature = "debug-embed")))]
fn fs_embed_file_names(root_relative: &str, includes: &[&str], excludes: &[&str]) -> Vec<String> {
    let Some(root) = dev_repo_root().map(|root| root.join(root_relative)) else {
        return Vec::new();
    };
    let matcher = rust_embed::utils::PathMatcher::new(includes, excludes);
    rust_embed::utils::get_files(root.to_string_lossy().into_owned(), matcher)
        .map(|entry| entry.rel_path)
        .collect()
}

#[cfg(all(debug_assertions, not(feature = "debug-embed")))]
#[doc(hidden)]
pub fn __fs_embed_get(
    root_relative: &str,
    file_path: &str,
    includes: &[&str],
    excludes: &[&str],
) -> Option<rust_embed::EmbeddedFile> {
    let matcher = rust_embed::utils::PathMatcher::new(includes, excludes);
    if !matcher.is_path_included(file_path) {
        return None;
    }
    let root = dev_repo_root()
        .expect("dev asset loading requires running from within the checkout")
        .join(root_relative);
    rust_embed::utils::read_file_from_fs(&root.join(file_path)).ok()
}

#[cfg(feature = "debug-embed")]
#[doc(hidden)]
#[macro_export]
macro_rules! __fs_embed {
    (
        $vis:vis struct $name:ident,
        crate_relative = $crate_relative:literal,
        root_relative = $root_relative:literal
        $(, include = [$($include:literal),* $(,)?])?
        $(, exclude = [$($exclude:literal),* $(,)?])?
        $(,)?
    ) => {
        #[derive($crate::__rust_embed::RustEmbed)]
        #[crate_path = "::util::__rust_embed"]
        #[folder = $crate_relative]
        $($(#[include = $include])*)?
        $($(#[exclude = $exclude])*)?
        $vis struct $name;
    };
}

#[cfg(not(feature = "debug-embed"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __fs_embed {
    (
        $vis:vis struct $name:ident,
        crate_relative = $crate_relative:literal,
        root_relative = $root_relative:literal
        $(, include = [$($include:literal),* $(,)?])?
        $(, exclude = [$($exclude:literal),* $(,)?])?
        $(,)?
    ) => {
        // `crate_path` points the derive's generated code at util's re-export so
        // the caller needs no direct `rust_embed` dependency.
        #[cfg(not(debug_assertions))]
        #[derive($crate::__rust_embed::RustEmbed)]
        #[crate_path = "::util::__rust_embed"]
        #[folder = $crate_relative]
        $($(#[include = $include])*)?
        $($(#[exclude = $exclude])*)?
        $vis struct $name;

        #[cfg(debug_assertions)]
        $vis struct $name;

        // Mirror the derive's public surface: inherent `get`/`iter` (callable
        // without the trait in scope) plus the trait impl (for generic bounds
        // like `util::asset_str` and `handlebars::register_embed_templates`), so
        // the two arms are interchangeable at call sites.
        #[cfg(debug_assertions)]
        impl $name {
            pub fn get(
                file_path: &str,
            ) -> ::core::option::Option<$crate::__rust_embed::EmbeddedFile> {
                $crate::__fs_embed_get(
                    $root_relative,
                    file_path,
                    &[$($($include),*)?],
                    &[$($($exclude),*)?],
                )
            }

            pub fn iter(
            ) -> impl ::core::iter::Iterator<Item = ::std::borrow::Cow<'static, str>> + 'static
            {
                $crate::__fs_embed_iter(
                    $root_relative,
                    &[$($($include),*)?],
                    &[$($($exclude),*)?],
                )
            }
        }

        #[cfg(debug_assertions)]
        impl $crate::__rust_embed::RustEmbed for $name {
            fn get(
                file_path: &str,
            ) -> ::core::option::Option<$crate::__rust_embed::EmbeddedFile> {
                <$name>::get(file_path)
            }

            fn iter(
            ) -> impl ::core::iter::Iterator<Item = ::std::borrow::Cow<'static, str>> + 'static
            {
                $crate::__fs_embed_iter(
                    $root_relative,
                    &[$($($include),*)?],
                    &[$($($exclude),*)?],
                )
            }
        }
    };
}

/// A `rust_embed` asset source that embeds files in release builds. Dev builds
/// read from the checkout at runtime unless the `debug-embed` feature is enabled,
/// in which case they embed the files too.
///
/// It expands to one of these arms:
/// * Release (`not(debug_assertions)`): `#[derive(RustEmbed)]` embedding
///   `crate_relative` at build time, with the given `include`/`exclude` globs.
/// * Dev with `debug-embed`: the same compile-time embedding as release.
/// * Dev without `debug-embed`: a runtime filesystem source rooted at
///   `root_relative`; edits appear on the next launch without a rebuild.
///
/// Two paths are required because the arms resolve from different bases: the
/// derive reads `crate_relative` relative to the crate's `Cargo.toml`, while the
/// runtime dev arm resolves `root_relative` relative to the repository root via
/// [`dev_repo_root`]. Baking the build-time path into that arm would point at the
/// wrong checkout from another worktree and is rejected by corgi, whose sandbox
/// requires checkout-independent output.
///
/// # Choosing the number of `../` — read this before adding a new `fs_embed!`
///
/// **`root_relative` is always easy: one segment, relative to the repository root.**
/// Write `"assets"` and it is done.
///
/// **`crate_relative` is the trap.** It is resolved against the *consuming crate's*
/// `CARGO_MANIFEST_DIR`, so its `../` depth must equal the crate's own nesting depth
/// inside the repository. There is no compile-time check for this, and the failure
/// is silent and asymmetric:
///
/// | build | which path is used | wrong depth ⇒ |
/// |---|---|---|
/// | dev (`cargo run`) | `root_relative` | **works** — reads the checkout |
/// | release (`cargo build --release`) | `crate_relative` | **embeds nothing** |
///
/// The embed then fails at *runtime*, not build time, via
/// [`asset_str`] → `.expect(path)`:
///
/// ```text
/// thread 'main' panicked at util/src/embed.rs:9:24:
/// settings/default.json
/// ```
///
/// Because dev builds never touch `crate_relative`, such a mistake survives
/// however long you only ever run `cargo run` — it surfaces the first time
/// someone runs an installed release build.
///
/// To get the depth right, count from the crate directory up to the repository
/// root. This repository's assets live at `<repo-root>/assets/`:
///
/// | crate location | `crate_relative` |
/// |---|---|
/// | `packages/aa/aa_assets/aa_assets_core/` (depth 3) | `"../../../../assets"` |
/// | `crates/settings/` (depth 1) | `"../../assets"` |
///
/// 注意第一行用的是 `RustEmbed` 的 `#[folder]` 而不是 `crate_relative`；
/// 两者都按 crate 目录起算，但 `#[folder]` 不在 dev/release 间切换，
/// 写错时两种构建都会静默内嵌出空目录 —— 靠 `cargo run` 发现不了。
///
/// **Do not copy another call site's `crate_relative` verbatim** — a sibling crate
/// one directory deeper needs one more `../` for the same target.
///
/// If you are unsure, verify the embed actually contains files rather than
/// assuming it. `debug-embed` is a default feature of this crate, so a plain
/// `cargo build` embeds exactly what a release build does; a test like
/// `assert!(!default_settings().is_empty())` covers the release path even in
/// debug. 触发本条目的 settings 包（曾在本仓库 `packages/zed/zed_settings/`，
/// depth 2，用 `"../../../assets"`）已于 2026-10 搬去 aacode 的
/// `packages/setting/settings/`（同为 depth 2，层级数不变），那边的
/// `src/lib.rs` 保留着三个此类回归测试，都是这个 bug 上线后才补的。
///
/// ```ignore
/// // In aacode's packages/setting/settings/ (two levels below the repo root):
/// util::fs_embed! {
///     pub struct SettingsAssets,
///     crate_relative = "../../../assets", // ← three levels, not two
///     root_relative = "assets",
///     include = ["settings/*", "keymaps/*"],
///     exclude = ["*.DS_Store"],
/// }
/// ```
#[macro_export]
macro_rules! fs_embed {
    ($($tokens:tt)*) => {
        $crate::__fs_embed!($($tokens)*);
    };
}
