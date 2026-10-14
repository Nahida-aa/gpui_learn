//! 颜色类型 —— 从 `gpui` 抽出，供 gpui 与 iced 两个后端共享。
//!
//! ## 为什么不直接用 `iced::Color`
//!
//! `iced::Color` 是 sRGB RGBA（4×`f32`），而本仓库的主题体系以 HSL 为内部
//! 表示：`default_colors.rs` 的整套调色板、`fallback_themes.rs` 的兜底家族、
//! 以及 `color_space.rs` 的 OKLab 派生色，全都是按 HSL 组织的字面量。
//! 若中途换成 RGB，调「降 5% 明度」「往 hue 方向推」这类微调就只能手算三通道。
//!
//! ## 为什么逐字照搬 gpui 的数学
//!
//! 下面每个转换函数都是 `zed/crates/gpui/src/color.rs` 的逐字移植，包括几处
//! 看起来可疑但**必须保留**的行为：
//!
//! - [`Hsla::blend`] 内部走 `Rgba::blend`，而后者的 alpha 取 `self.a` 而非
//!   合成后的 alpha。这不是理想的结果，但 gpui 侧长期如此；两端算法一旦不同，
//!   同一个半透明叠加在 gpui 下和 iced 下就会渲出不同颜色。
//! - `hsla()` 对四个通道一律 `clamp(0., 1.)`，包括色相 `h`。
//!
//! 迁移 gpui 侧（`aa_theme_gpui` 改为依赖本 crate）时，本文件是「正确」的那一份，
//! gpui 版本应被删除而非保留。

use anyhow::{Context as _, Result, bail};
use palette::FromColor as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// An RGBA color, non-premultiplied, each channel in the range `0.0..=1.0`.
#[derive(Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Rgba {
    /// The red component of the color, in the range 0.0 to 1.0
    pub r: f32,
    /// The green component of the color, in the range 0.0 to 1.0
    pub g: f32,
    /// The blue component of the color, in the range 0.0 to 1.0
    pub b: f32,
    /// The alpha component of the color, in the range 0.0 to 1.0
    pub a: f32,
}

impl fmt::Debug for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rgba({:#010x})", u32::from(*self))
    }
}

impl From<Rgba> for u32 {
    fn from(rgba: Rgba) -> Self {
        let r = (rgba.r * 255.0) as u32;
        let g = (rgba.g * 255.0) as u32;
        let b = (rgba.b * 255.0) as u32;
        let a = (rgba.a * 255.0) as u32;
        r << 24 | g << 16 | b << 8 | a
    }
}

impl Rgba {
    /// Blends `other` on top of `self` based on `other`'s alpha value.
    ///
    /// 注意 alpha 取 `self.a`（gpui 原样）。见 crate 文档的说明。
    pub fn blend(&self, other: Rgba) -> Self {
        if other.a >= 1.0 {
            other
        } else if other.a <= 0.0 {
            *self
        } else {
            Rgba {
                r: (self.r * (1.0 - other.a)) + (other.r * other.a),
                g: (self.g * (1.0 - other.a)) + (other.g * other.a),
                b: (self.b * (1.0 - other.a)) + (other.b * other.a),
                a: self.a,
            }
        }
    }

    /// Returns a new RGBA color with the same red, green and blue channels, but
    /// with a new alpha value.
    pub fn alpha(&self, a: f32) -> Self {
        Rgba {
            r: self.r,
            g: self.g,
            b: self.b,
            a: a.clamp(0., 1.),
        }
    }
}

impl From<[f32; 4]> for Rgba {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Rgba> for [u8; 4] {
    fn from(rgba: Rgba) -> Self {
        [
            (rgba.r * 255.0) as u8,
            (rgba.g * 255.0) as u8,
            (rgba.b * 255.0) as u8,
            (rgba.a * 255.0) as u8,
        ]
    }
}

impl From<Rgba> for Hsla {
    fn from(color: Rgba) -> Self {
        let r = color.r;
        let g = color.g;
        let b = color.b;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        let l = (max + min) / 2.0;
        let s = if l == 0.0 || l == 1.0 {
            0.0
        } else if l < 0.5 {
            delta / (2.0 * l)
        } else {
            delta / (2.0 - 2.0 * l)
        };

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if max == g {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };

        Hsla {
            h,
            s,
            l,
            a: color.a,
        }
    }
}

impl From<Hsla> for Rgba {
    fn from(color: Hsla) -> Self {
        let h = color.h;
        let s = color.s;
        let l = color.l;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let cm = c + m;
        let xm = x + m;

        let (r, g, b) = match (h * 6.0).floor() as i32 {
            0 | 6 => (cm, xm, m),
            1 => (xm, cm, m),
            2 => (m, cm, xm),
            3 => (m, xm, cm),
            4 => (xm, m, cm),
            _ => (cm, m, xm),
        };

        Rgba {
            r: r.clamp(0., 1.),
            g: g.clamp(0., 1.),
            b: b.clamp(0., 1.),
            a: color.a,
        }
    }
}

impl TryFrom<&'_ str> for Rgba {
    type Error = anyhow::Error;

    /// 接受 `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`（gpui 同款，不接受
    /// `rgb(...)` 函数式写法）。
    fn try_from(value: &'_ str) -> Result<Self> {
        const RGB: usize = "rgb".len();
        const RGBA: usize = "rgba".len();
        const RRGGBB: usize = "rrggbb".len();
        const RRGGBBAA: usize = "rrggbbaa".len();

        const EXPECTED_FORMATS: &str = "Expected #rgb, #rgba, #rrggbb, or #rrggbbaa";
        const INVALID_UNICODE: &str = "invalid unicode characters in color";

        let Some(("", hex)) = value.trim().split_once('#') else {
            bail!("invalid RGBA hex color: '{value}'. {EXPECTED_FORMATS}");
        };

        let (r, g, b, a) = match hex.len() {
            RGB | RGBA => {
                let r = u8::from_str_radix(
                    hex.get(0..1).with_context(|| {
                        format!("{INVALID_UNICODE}: r component of #rgb/#rgba for value: '{value}'")
                    })?,
                    16,
                )?;
                let g = u8::from_str_radix(
                    hex.get(1..2).with_context(|| {
                        format!("{INVALID_UNICODE}: g component of #rgb/#rgba for value: '{value}'")
                    })?,
                    16,
                )?;
                let b = u8::from_str_radix(
                    hex.get(2..3).with_context(|| {
                        format!("{INVALID_UNICODE}: b component of #rgb/#rgba for value: '{value}'")
                    })?,
                    16,
                )?;
                let a = if hex.len() == RGBA {
                    u8::from_str_radix(
                        hex.get(3..4).with_context(|| {
                            format!("{INVALID_UNICODE}: a component of #rgba for value: '{value}'")
                        })?,
                        16,
                    )?
                } else {
                    0xf
                };

                /// Duplicates a given hex digit.
                /// E.g., `0xf` -> `0xff`.
                const fn duplicate(value: u8) -> u8 {
                    (value << 4) | value
                }

                (duplicate(r), duplicate(g), duplicate(b), duplicate(a))
            }
            RRGGBB | RRGGBBAA => {
                let r = u8::from_str_radix(
                    hex.get(0..2).with_context(|| {
                        format!(
                            "{}: r component of #rrggbb/#rrggbbaa for value: '{}'",
                            INVALID_UNICODE, value
                        )
                    })?,
                    16,
                )?;
                let g = u8::from_str_radix(
                    hex.get(2..4).with_context(|| {
                        format!("{INVALID_UNICODE}: g component of #rrggbb/#rrggbbaa for value: '{value}'")
                    })?,
                    16,
                )?;
                let b = u8::from_str_radix(
                    hex.get(4..6).with_context(|| {
                        format!("{INVALID_UNICODE}: b component of #rrggbb/#rrggbbaa for value: '{value}'")
                    })?,
                    16,
                )?;
                let a = if hex.len() == RRGGBBAA {
                    u8::from_str_radix(
                        hex.get(6..8).with_context(|| {
                            format!(
                                "{}: a component of #rrggbb/#rrggbbaa for value: '{}'",
                                INVALID_UNICODE, value
                            )
                        })?,
                        16,
                    )?
                } else {
                    0xff
                };

                (r, g, b, a)
            }
            _ => bail!("invalid RGBA hex color: '{value}'. {EXPECTED_FORMATS}"),
        };

        Ok(Rgba {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
            a: a as f32 / 255.,
        })
    }
}

/// An HSLA color. Each channel is in the range `0.0..=1.0` (`h` included).
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Hsla {
    /// Hue, in a range from 0 to 1
    pub h: f32,
    /// Saturation, in a range from 0 to 1
    pub s: f32,
    /// Lightness, in a range from 0 to 1
    pub l: f32,
    /// Alpha, in a range from 0 to 1
    pub a: f32,
}

/// Creates an [`Hsla`], clamping all four channels to `0.0..=1.0`.
pub const fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla {
        h: h.clamp(0., 1.),
        s: s.clamp(0., 1.),
        l: l.clamp(0., 1.),
        a: a.clamp(0., 1.),
    }
}

/// Pure black in [`Hsla`]
pub const fn black() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 0.,
        a: 1.,
    }
}

/// Transparent black in [`Hsla`]
pub const fn transparent_black() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 0.,
        a: 0.,
    }
}

/// Transparent white in [`Hsla`]
pub const fn transparent_white() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 1.,
        a: 0.,
    }
}

/// An [`Hsla`] with the given lightness and alpha, and no saturation or hue.
pub const fn opaque_grey(lightness: f32, opacity: f32) -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: lightness.clamp(0., 1.),
        a: opacity.clamp(0., 1.),
    }
}

/// Pure white in [`Hsla`]
pub const fn white() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 1.,
        a: 1.,
    }
}

/// Pure red in [`Hsla`]
pub const fn red() -> Hsla {
    Hsla {
        h: 0.,
        s: 1.,
        l: 0.5,
        a: 1.,
    }
}

/// Pure blue in [`Hsla`]
pub const fn blue() -> Hsla {
    Hsla {
        h: 0.666_666_7,
        s: 1.,
        l: 0.5,
        a: 1.,
    }
}

/// Pure green in [`Hsla`]
pub const fn green() -> Hsla {
    Hsla {
        h: 0.333_333_34,
        s: 1.,
        // 注意 l 是 0.25 而不是 0.5 —— gpui 的 green 就是这个偏暗的值。
        l: 0.25,
        a: 1.,
    }
}

/// Pure yellow in [`Hsla`]
pub const fn yellow() -> Hsla {
    Hsla {
        h: 0.166_666_67,
        s: 1.,
        l: 0.5,
        a: 1.,
    }
}

impl Hsla {
    /// Converts this HSLA color to an RGBA color.
    pub fn to_rgb(self) -> Rgba {
        self.into()
    }

    /// Returns true if the color is fully transparent, false otherwise.
    pub fn is_transparent(&self) -> bool {
        self.a == 0.
    }

    /// Returns true if the color is fully opaque, false otherwise.
    pub fn is_opaque(&self) -> bool {
        self.a == 1.
    }

    /// Blends `other` on top of `self` based on `other`'s alpha value.
    ///
    /// Assumptions:
    /// - Alpha values are contained in the range [0, 1], with 1 as fully opaque and 0 as fully transparent.
    /// - The relative contributions of `self` and `other` is based on `self`'s alpha value
    ///   (`self.a`) and `other`'s alpha value (`other.a`), `self` contributing
    ///   `self.a * (1.0 - other.a)` and `other` contributing its own alpha value.
    pub fn blend(self, other: Hsla) -> Hsla {
        let alpha = other.a;

        if alpha >= 1.0 {
            other
        } else if alpha <= 0.0 {
            self
        } else {
            let converted_self = Rgba::from(self);
            let converted_other = Rgba::from(other);
            let blended_rgb = converted_self.blend(converted_other);
            Hsla::from(blended_rgb)
        }
    }

    /// Returns a new HSLA color with the same hue and lightness, but with no saturation.
    pub fn grayscale(&self) -> Self {
        Hsla {
            h: self.h,
            s: 0.,
            l: self.l,
            a: self.a,
        }
    }

    /// Fade out the color by a given factor. This factor should be between 0.0 and 1.0.
    /// Where 0.0 will leave the color unchanged, and 1.0 will completely fade out the color.
    pub fn fade_out(&mut self, factor: f32) {
        self.a *= 1.0 - factor.clamp(0., 1.);
    }

    /// Multiplies the alpha value of the color by a given factor
    /// and returns a new HSLA color.
    ///
    /// Useful for transforming colors with dynamic opacity,
    /// like a color from an external source.
    pub fn opacity(&self, factor: f32) -> Self {
        Hsla {
            h: self.h,
            s: self.s,
            l: self.l,
            a: self.a * factor.clamp(0., 1.),
        }
    }

    /// Returns a new HSLA color with the same hue, saturation,
    /// and lightness, but with a new alpha value.
    pub fn alpha(&self, a: f32) -> Self {
        Hsla {
            h: self.h,
            s: self.s,
            l: self.l,
            a: a.clamp(0., 1.),
        }
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RgbaVisitor;

        impl serde::de::Visitor<'_> for RgbaVisitor {
            type Value = Rgba;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in the format #rrggbb or #rrggbbaa")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> std::result::Result<Rgba, E> {
                Rgba::try_from(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(RgbaVisitor)
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        let a = (self.a * 255.0).round() as u8;

        let s = format!("#{r:02x}{g:02x}{b:02x}{a:02x}");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Hsla {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Rgba::deserialize(deserializer)?.into())
    }
}

impl Serialize for Hsla {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        Rgba::from(*self).serialize(serializer)
    }
}

impl schemars::JsonSchema for Rgba {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Rgba".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"
        })
    }
}

impl schemars::JsonSchema for Hsla {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        Rgba::schema_name()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        Rgba::json_schema(generator)
    }
}

/// 主题 JSON / VSCode 设置里的十六进制颜色 → [`Hsla`]。
///
/// 与 `aa_theme_gpui::schema::try_parse_color` 的区别：那条链路走
/// `palette::Hsla::from_color` 做 sRGB→HSL，本函数走 gpui 的
/// [`From<Rgba> for Hsla`]。两者结果差异在 1e-7 量级（见 gpui 侧同名测试），
/// 但后者与 gpui 渲染路径**逐位一致**，因此取后者 —— 共享同一份色板时，
/// 两端不该因为走不同公式而出现肉眼不可见但确实存在的偏差。
/// 解析 `#rgb` / `#rrggbb` / `#rrggbbaa` 成 [`Hsla`]。
///
/// RGB→HSL 走 `palette` 而不是本 crate 的 `Hsla::from(Rgba)`：后者是 gpui
/// 手写的 max/min 公式，与 **zed `crates/theme/src/schema.rs:17` 的 palette
/// 路径**在 hue 上差 1 ulp（如 `#cad3f5` 的 h 差 6e-8）。本函数属于 theme 而非
/// gpui，语义上应与上游一致，故取 palette 路径。palette 是纯计算库，不引入
/// 任何 UI 框架依赖，符合本 crate 的边界。
///
/// 之所以要精确到 ulp：gpui 后端切换到本 crate 之前，两份实现会同时存在并
/// 分别解析同一份调色板；若解析结果不一致，同一个色值在两个后端下会漂移。
/// `aa_theme_gpui` 侧的 `color_parity` 测试用 `to_bits()` 比对来盯这件事。
pub fn try_parse_color(color: &str) -> Result<Hsla> {
    let rgba =
        Rgba::try_from(color).map_err(|err| anyhow::anyhow!("invalid color {color:?}: {err}"))?;
    let srgba = palette::rgb::Srgba::from_components((rgba.r, rgba.g, rgba.b, rgba.a));
    // 局部变量不能也叫 hsla —— 本模块导出了同名函数 `hsla()`，会遮蔽它。
    let converted = palette::Hsla::from_color(srgba);
    Ok(hsla(
        converted.hue.into_positive_degrees() / 360.,
        converted.saturation,
        converted.lightness,
        converted.alpha,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() < tolerance
    }

    #[test]
    fn parses_six_and_eight_digit_hex() {
        let rgb = Rgba::try_from("#ff8000").unwrap();
        assert_eq!(
            rgb,
            Rgba {
                r: 1.0,
                g: 128.0 / 255.,
                b: 0.0,
                a: 1.0
            }
        );

        let rgba = Rgba::try_from("#ff800080").unwrap();
        assert_eq!(
            rgba,
            Rgba {
                r: 1.0,
                g: 128.0 / 255.,
                b: 0.0,
                a: 128.0 / 255.
            }
        );
    }

    #[test]
    fn parses_short_hex_by_duplicating_digits() {
        // #abc → #aabbcc，且 #abcd 的末位是 alpha 而非第四个 RGB 通道。
        let rgb = Rgba::try_from("#abc").unwrap();
        assert_eq!(
            rgb,
            Rgba {
                r: 0xaa as f32 / 255.,
                g: 0xbb as f32 / 255.,
                b: 0xcc as f32 / 255.,
                a: 1.0
            }
        );

        let rgba = Rgba::try_from("#abcd").unwrap();
        assert_eq!(
            rgba,
            Rgba {
                r: 0xaa as f32 / 255.,
                g: 0xbb as f32 / 255.,
                b: 0xcc as f32 / 255.,
                a: 0xdd as f32 / 255.
            }
        );
    }

    #[test]
    fn rejects_malformed_colors() {
        // 注意 "#ff80" 不在此列：4 位是合法的 #rgba。
        for bad in ["", "ff8000", "#ff80000", "#ggg", "rgb(1,2,3)"] {
            assert!(Rgba::try_from(bad).is_err(), "{bad:?} 应解析失败");
        }
    }

    #[test]
    fn accepts_the_formats_the_error_message_claims() {
        // 错误文案承诺了 4 种格式，逐个确认没有在文案上撒谎。
        for good in ["#f00", "#f00f", "#ff0000", "#ff0000ff"] {
            assert!(Rgba::try_from(good).is_ok(), "{good:?} 应解析成功");
        }
    }

    #[test]
    fn hsla_round_trips_through_rgb() {
        for color in [
            hsla(0.6317829, 0.6971, 0.7805, 1.0),
            hsla(0.0, 0.0, 0.0, 1.0),
            hsla(0.5, 0.5, 0.5, 0.5),
        ] {
            let back = Hsla::from(color.to_rgb());
            assert!(close(color.h, back.h, 1e-5), "h: {color:?} -> {back:?}");
            assert!(close(color.s, back.s, 1e-5), "s: {color:?} -> {back:?}");
            assert!(close(color.l, back.l, 1e-5), "l: {color:?} -> {back:?}");
            assert_eq!(color.a, back.a);
        }
    }

    #[test]
    fn hue_is_taken_modulo_one() {
        // HSL 允许 h 超出 [0,1) 表示绕圈；实测 gpui 的取模行为是
        // rem_euclid(6)/6，而不是先 clamp。这里锁住该行为。
        let wrapped = Hsla::from(Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        assert_eq!(wrapped.h, 0.0);
    }

    #[test]
    fn hsla_constructor_clamps_every_channel_including_hue() {
        let color = hsla(1.5, 2.0, -0.5, 3.0);
        assert_eq!(
            color,
            Hsla {
                h: 1.0,
                s: 1.0,
                l: 0.0,
                a: 1.0
            }
        );
    }

    #[test]
    fn opacity_and_alpha_respect_input_range() {
        assert_eq!(red().opacity(0.5).a, 0.5);
        assert_eq!(red().alpha(2.0).a, 1.0, "alpha 会被 clamp 到 1");
        assert_eq!(red().alpha(-1.0).a, 0.0);

        let mut faded = red();
        faded.fade_out(1.0);
        assert_eq!(faded.a, 0.0, "fade_out(1.0) 应完全透明");
    }

    #[test]
    fn blend_short_circuits_on_opaque_and_transparent() {
        let base = hsla(0.6, 0.7, 0.5, 1.0);
        assert_eq!(base.blend(white()), white());
        assert_eq!(base.blend(transparent_black()), base);
    }

    #[test]
    fn blend_keeps_the_base_alpha() {
        // 锁住 gpui 的既有行为：blend 后的 alpha 取 self 而非合成值。
        // 见 crate 文档 —— 若将来两边都改成「正确的」合成语义，
        // 这条测试和 gpui 侧必须同步改。
        let base = hsla(0.6, 0.7, 0.5, 0.3);
        // other 也必须是半透明：a>=1.0 时 blend 直接返回 other，走不到这段。
        let blended = base.blend(hsla(0.0, 0.0, 0.0, 0.5));
        assert_eq!(blended.a, 0.3);
        assert_ne!(blended.to_rgb(), base.to_rgb(), "RGB 通道应确实被混合了");
    }

    #[test]
    fn serializes_back_to_hex() {
        let rgba = Rgba {
            r: 1.0,
            g: 128.0 / 255.,
            b: 0.0,
            a: 1.0,
        };
        assert_eq!(serde_json::to_string(&rgba).unwrap(), "\"#ff8000ff\"");

        // Hsla 走 Rgba 序列化，因此往返是 无损 的十六进制。
        let parsed: Hsla = serde_json::from_str("\"#ff8000ff\"").unwrap();
        // h = (g-b)/delta/6，delta = 1.0 - 0.0 = 1.0 → 0.50196/6 ≈ 0.08366。
        assert!(close(parsed.h, 0.50196 / 6.0, 1e-5), "{parsed:?}");
        assert!(close(parsed.s, 1.0, 1e-5), "{parsed:?}");
        assert!(close(parsed.l, 0.5, 1e-5), "{parsed:?}");
    }

    #[test]
    fn try_parse_color_matches_gpui_conversion() {
        // 与 aa_theme_gpui::schema::tests 里那几个十六进制对应，
        // 便于迁移时直接比对两边结果。
        for hex in ["#24273a", "#cad3f5", "#ed8796"] {
            let parsed = try_parse_color(hex).unwrap();
            let from_rgb = Hsla::from(Rgba::try_from(hex).unwrap());
            assert!(close(parsed.h, from_rgb.h, 1e-6));
            assert!(close(parsed.s, from_rgb.s, 1e-6));
            assert!(close(parsed.l, from_rgb.l, 1e-6));
        }
    }
}
