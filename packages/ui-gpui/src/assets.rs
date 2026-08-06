//! ui-gpui 自带的内嵌资源。
//!
//! 用 `rust-embed` 在编译期把 `assets/` 目录内嵌进二进制，运行时通过 gpui 的
//! `AssetSource` 接口按路径读取（如 `icons/play_filled.svg`）。
//!
//! 注意：gpui 的 `svg().path(...)` 只认 **app 注册的 `AssetSource`**（即
//! `Application::with_assets(...)` 传入的那个）。所以使用 ui-gpui 的 app 必须把
//! 本 crate 的 [`Assets`] 组合进自己的 asset source，否则图标路径解析不到。
//!
//! 资源放在本 crate 的 `assets/` 子目录（与 `src/` 同级），由
//! `#[folder = "assets"]` 指定为 embed 根。

use anyhow::Context as _;
use gpui::{App, AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*"]
#[include = "*.md"]
#[exclude = "*.DS_Store"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .with_context(|| format!("loading asset at path {path:?}"))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| {
                if p.starts_with(path) {
                    Some(p.into())
                } else {
                    None
                }
            })
            .collect())
    }
}

impl Assets {
    /// Populate the [`TextSystem`] of the given [`AppContext`] with all `.ttf` fonts in the `fonts` directory.
    pub fn load_fonts(&self, cx: &App) -> anyhow::Result<()> {
        let font_paths = self.list("fonts")?;
        let mut embedded_fonts = Vec::new();
        for font_path in font_paths {
            if font_path.ends_with(".ttf") {
                let font_bytes = cx
                    .asset_source()
                    .load(&font_path)?
                    .expect("Assets should never return None");
                embedded_fonts.push(font_bytes);
            }
        }

        cx.text_system().add_fonts(embedded_fonts)
    }
}
