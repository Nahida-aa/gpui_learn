//! gpui 侧适配层：把 [`aa_assets_core::Assets`] 的内嵌资源暴露成 gpui 的 [`AssetSource`]。
//!
//! ## 为什么必须是本 crate 的类型（不能省）
//!
//! 直觉上想让 core 直接 `impl gpui::AssetSource`，但那会撞孤儿规则（E0117）：
//! trait 来自外部 crate（gpui），类型也来自外部 crate（aa_assets_core），
//! 两者都不是本 crate 的。`pub use` 也绕不开 —— 重新导出的仍是「外部类型」。
//! 所以必须有一个本地类型承载这个 impl。本仓库既有的
//! `theme_settings::GlobalAssets` 是同一模式。
//!
//! ## 为什么是 unit struct 而不是 newtype
//!
//! 内嵌资源全是编译期静态数据，没有实例状态可存，包一层字段只会：
//! 多出 `new()` / `Default` 等构造入口，以及 `load_asset` / `list_assets`
//! 这类无人调用的转发方法；还让调用处能写出 `.0` 摸到内部结构。
//! unit struct 本身就是值表达式，构造直接写 `GpuiAssets`。
//!
//! 需要 RustEmbed 关联函数（`asset_str!` 等）时直接用
//! [`aa_assets_core::Assets`]，不必经过本层。

use std::borrow::Cow;

use aa_assets_core::Assets as CoreAssets;
use anyhow::Result;
use gpui::{App, AssetSource};
use gpui_shared_string::SharedString;

pub struct GpuiAssets;

impl GpuiAssets {
    /// 把 `fonts/` 下所有 `.ttf` 装进 [`App`] 的 TextSystem。
    ///
    /// 刻意经 `cx.asset_source()` 取字节而不是直接 `CoreAssets::get`：测试
    /// 上下文会注入自己的 AssetSource，直接读内嵌表会绕过它。
    pub fn load_fonts(&self, cx: &App) -> Result<()> {
        let mut embedded_fonts = Vec::new();
        for font_path in CoreAssets::iter() {
            let font_path = font_path.into_owned();
            if font_path.starts_with("fonts") && font_path.ends_with(".ttf") {
                let font_bytes = cx
                    .asset_source()
                    .load(&font_path)?
                    .expect("Assets should never return None");
                embedded_fonts.push(font_bytes);
            }
        }
        cx.text_system().add_fonts(embedded_fonts)
    }

    pub fn load_test_fonts(&self, cx: &App) {
        cx.text_system()
            .add_fonts(vec![
                CoreAssets::get("fonts/lilex/Lilex-Regular.ttf")
                    .expect("test font must be embedded")
                    .data,
            ])
            .unwrap()
    }
}

impl AssetSource for GpuiAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(CoreAssets::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(CoreAssets::iter()
            .filter(|p| p.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// unit struct 的前提是「资源全是编译期静态、没有实例状态」。
    /// 这条测试锁住该前提：万一 core 将来改成持有状态，这里会立刻失败，
    /// 提醒把 GpuiAssets 换回 newtype。
    #[test]
    fn gpui_assets_carries_no_state() {
        assert_eq!(std::mem::size_of::<GpuiAssets>(), 0);
    }

    /// `#[folder]` 深路径写错时 core 会静默内嵌出空目录（两种构建都崩，
    /// 靠 cargo run 发现不了）。这里直接验内嵌非空。
    #[test]
    fn embedded_assets_are_non_empty() {
        assert!(
            CoreAssets::get("fonts/lilex/Lilex-Regular.ttf").is_some(),
            "内嵌字体缺失：#[folder] 路径可能不对"
        );
    }

    /// AssetSource 两条路径都要能读到同一份资源。
    #[test]
    fn asset_source_reads_embedded_files() {
        let assets = GpuiAssets;
        assert!(
            AssetSource::load(&assets, "fonts/lilex/Lilex-Regular.ttf")
                .unwrap()
                .is_some()
        );
        assert!(!AssetSource::list(&assets, "icons").unwrap().is_empty());
    }

    /// 记录当前实际行为：缺失路径返回 `Ok(None)`。
    ///
    /// 注意这与旧的 `aa_gpui_kit_assets` 不同 —— 那份的 `load_asset` 对
    /// `Option` 调了 `.with_context()`，会把 `None` 变成 `Err`，违反了
    /// `AssetSource::load` 的签名约定（gpui 自己的 `impl for ()` 返回 `Ok(None)`）。
    /// unit struct 版顺手对齐到了 trait 契约。
    #[test]
    fn missing_asset_is_none_not_err() {
        assert!(
            AssetSource::load(&GpuiAssets, "fonts/does-not-exist.ttf")
                .unwrap()
                .is_none()
        );
    }
}
