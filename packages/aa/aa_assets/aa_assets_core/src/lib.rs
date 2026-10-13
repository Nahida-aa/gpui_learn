// This crate was essentially pulled out verbatim from main `zed` crate to avoid having to run RustEmbed macro whenever zed has to be rebuilt. It saves a second or two on an incremental build.
use anyhow::{Context as _, Result};
use rust_embed::RustEmbed;
use std::borrow::Cow;
// IconName 不在这里：全仓库唯一的枚举在 `aa_icons`（zed crates/icons 原样），
// 由它的 `path()` + 双向测试负责「枚举 ↔ SVG 文件」的对应关系。
#[derive(RustEmbed)]
#[folder = "../../../../assets"]
#[include = "fonts/**/*"]
#[include = "icons/**/*"]
// 图标主题的多彩 SVG。只收 `*.svg`：`catppuccin-icons.json` 由 theme crate
// `include_str!` 引入，别在二进制里存两份。
#[include = "icon_themes/**/*.svg"]
#[include = "images/**/*"]
#[include = "themes/**/*"]
#[exclude = "themes/src/*"]
#[include = "sounds/**/*"]
#[include = "prompts/**/*"]
#[include = "*.md"]
#[exclude = "*.DS_Store"]
pub struct Assets;

pub trait AssetProvider {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>>;
    fn list(&self, path: &str) -> Result<Vec<String>>;
}

impl AssetProvider for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        self.load_asset(path)
    }

    fn list(&self, path: &str) -> Result<Vec<String>> {
        self.list_assets(path)
    }
}

impl Assets {
    pub fn load_asset(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .with_context(|| format!("loading asset at path {path:?}"))
    }

    pub fn list_assets(&self, path: &str) -> Result<Vec<String>> {
        Ok(Self::iter()
            .filter_map(|p| {
                if p.starts_with(path) {
                    // Self::iter() 产出 Cow<'_, str>；要 String 必须用 into_owned()
                    // 而不是 to_owned()（后者作用在 Cow 上仍是 Cow）。
                    Some(p.into_owned())
                } else {
                    None
                }
            })
            .collect())
    }
    pub fn load_font_bytes(&self) -> Result<Vec<Cow<'static, [u8]>>> {
        self.list_assets("fonts")?
            .into_iter()
            .filter(|p| p.ends_with(".ttf"))
            .map(|p| {
                self.load_asset(&p)?
                    .with_context(|| format!("missing embedded font: {p}"))
            })
            .collect()
    }

    pub fn test_font_bytes(&self) -> Vec<Cow<'static, [u8]>> {
        vec![
            self.load_asset("fonts/lilex/Lilex-Regular.ttf")
                .unwrap()
                .unwrap(),
        ]
    }
}
