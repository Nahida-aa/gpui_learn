//! schema:主题 JSON 的公开 schema 与颜色解析,对齐 zed `crates/theme/src/schema.rs`。
//!
//! zed 这个文件只有两样东西:一个「序列化形态的明暗枚举」和一个颜色解析
//! 函数——它们的共同点是**属于外部契约**(主题扩展 JSON 的作者要照着写)。
//!
//! 注意本包**只管这一层契约**,不管 JSON 的具体结构与装载:那些在
//! `theme-settings` 包(`content` / `loaders` / `init`),依赖方向是
//! `theme-settings → theme`。zed 同样是 `theme_settings` 引用
//! `theme::AppearanceContent` / `theme::try_parse_color`。
//!
//! 与 zed 的差异:zed 用 `schemars` 的 `JsonSchema` derive 生成 JSON Schema
//! 供编辑器做主题文件补全;我们暂无这个需求,不引入 schemars,该 derive 从略。

use gpui::Hsla;
use palette::FromColor as _;

/// 主题 JSON 里的明暗形态(对齐 zed `schema::AppearanceContent`)。
///
/// 与 [`Appearance`](crate::Appearance) 的区别:那个是**运行时**形态(主题
/// 被装入后用的),这个是**序列化**形态(JSON 里 `"appearance": "dark"`)。
/// 两者字段一一对应,由 `theme-settings` 包的 loaders 负责转换。
#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceContent {
    Light,
    Dark,
}

/// 把颜色字符串解析成 [`Hsla`](对齐 zed `schema::try_parse_color`)。
///
/// 先交给 gpui 的 `Rgba::try_from` 处理它认识的形式(`#rgb` / `#rrggbb` /
/// `#rrggbbaa` 等),再经 `palette` 做 sRGB → HSL 转换、把色相归一化到
/// `[0, 1)`——gpui 的 `hsla()` 要的正是这个区间。
///
/// 与 zed 的差异:zed 的 `anyhow::Result` 由底层错误直接冒泡;我们包一层
/// 带上原始字符串,便于定位 JSON 里写坏的颜色值。
pub fn try_parse_color(color: &str) -> anyhow::Result<Hsla> {
    let rgba = gpui::Rgba::try_from(color)
        .map_err(|err| anyhow::anyhow!("invalid color {color:?}: {err}"))?;
    let rgba = palette::rgb::Srgba::from_components((rgba.r, rgba.g, rgba.b, rgba.a));
    let hsla = palette::Hsla::from_color(rgba);

    Ok(gpui::hsla(
        hsla.hue.into_positive_degrees() / 360.,
        hsla.saturation,
        hsla.lightness,
        hsla.alpha,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Rgba;

    #[test]
    fn parses_hex_colors() {
        // 与 gpui 自带的 Rgba→Hsla 转换结果一致(说明 palette 这条链路没跑偏)。
        // 用容差比较:两条路径的浮点舍入在末位会差 1e-7 量级,
        // 例如 #cad3f5 的 h 是 0.6317829 vs 0.63178295。
        for hex in ["#24273a", "#cad3f5", "#ed8796"] {
            let ours = try_parse_color(hex).expect("parse hex");
            let expected = Hsla::from(Rgba::try_from(hex).unwrap());
            let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
            assert!(
                close(ours.h, expected.h)
                    && close(ours.s, expected.s)
                    && close(ours.l, expected.l)
                    && close(ours.a, expected.a),
                "{hex} 应与其 gpui 直转结果一致:\n  ours     = {ours:?}\n  expected = {expected:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_colors_with_context() {
        let err = try_parse_color("not-a-color").unwrap_err();
        assert!(
            err.to_string().contains("not-a-color"),
            "错误信息应带上原始字符串,便于定位 JSON: {err}"
        );
    }
}
