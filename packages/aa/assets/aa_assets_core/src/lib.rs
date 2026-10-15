// This crate was essentially pulled out verbatim from main `zed` crate to avoid having to run RustEmbed macro whenever zed has to be rebuilt. It saves a second or two on an incremental build.
use aa_shared_string::SharedString;
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
    fn list(&self, path: &str) -> Result<Vec<SharedString>>;
}

impl AssetProvider for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        self.load_asset(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        self.list_assets(path)
    }
}

impl Assets {
    pub fn load_asset(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .with_context(|| format!("loading asset at path {path:?}"))
    }

    /// 列出 `path` 前缀下的资源路径。
    ///
    /// 返回 [`SharedString`] 而非 [`String`]：资源路径都是短字符串，
    /// `SharedString`（`SmolStr`）对短串是**内联存储**，不分配堆，而且克隆
    /// 廉价 —— 这份列表会被上层反复遍历、克隆。
    pub fn list_assets(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter(|p| p.starts_with(path))
            // `SharedString: From<Cow<str>>`；短路径走 SmolStr 内联存储。
            .map(SharedString::from)
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
