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
    // rust-embed 的 derive（rust-embed-impl）在 compression 开启时会生成对
    // EmbeddedCompressedFile 的引用，并经 #[crate_path] 解析到本模块，故必须转出。
    //
    // 不做 #[cfg(feature = "compression")] 门控：那个 cfg 判的是 **util 自己的**
    // feature，而 util 并没有 compression 这个 feature（compression 是开在
    // rust-embed 上的），门控后恒为 false、类型永远导不出。
    //
    // 无条件转出的前提是 rust-embed >= 8.12（该类型 8.12 才加回来）。本 workspace 的
    // rust-embed 约束已从 "~8.11" 放宽到 "8.11"（caret，<9.0）以便与
    // rust-embed-impl 同版本，实际解析到 8.13.x。
    pub use rust_embed::EmbeddedCompressedFile;
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
/// ```ignore
/// util::fs_embed! {
///     pub struct Assets,
///     crate_relative = "../../assets",
///     root_relative = "assets",
///     include = ["fonts/**/*", "themes/**/*", "*.md"],
///     exclude = ["themes/src/*", "*.DS_Store"],
/// }
/// ```
#[macro_export]
macro_rules! fs_embed {
    ($($tokens:tt)*) => {
        $crate::__fs_embed!($($tokens)*);
    };
}
