//! Conversions between gpui's [`Hsla`] and the OKLab / OKLCh perceptual color
//! spaces, backed by the `palette` crate.
//!
//! These are exposed so consumers can reason about perceptual color distance
//! (e.g. bracket colorization) without taking a direct dependency on `palette`.

use gpui::{Hsla, Rgba};
use palette::{
    FromColor, OklabHue,
    rgb::{LinSrgba, Srgba},
};

/// A color in the OKLab perceptual color space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklab {
    /// Perceptual lightness, in `0.0..=1.0`.
    pub l: f32,
    /// Green/red opponent axis.
    pub a: f32,
    /// Blue/yellow opponent axis.
    pub b: f32,
}

/// A color in the OKLCh perceptual color space (the cylindrical form of OKLab).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    /// Perceptual lightness, in `0.0..=1.0`.
    pub l: f32,
    /// Chroma (colorfulness).
    pub chroma: f32,
    /// Hue, in degrees (`0.0..360.0`).
    pub hue: f32,
}

/// Converts an [`Hsla`] color into the OKLab color space.
pub fn hsla_to_oklab(color: Hsla) -> Oklab {
    let oklab = palette::Oklab::from_color(hsla_to_linear(color));
    Oklab {
        l: oklab.l,
        a: oklab.a,
        b: oklab.b,
    }
}

/// Converts an [`Hsla`] color into the OKLCh color space.
pub fn hsla_to_oklch(color: Hsla) -> Oklch {
    let oklch = palette::Oklch::from_color(hsla_to_linear(color));
    Oklch {
        l: oklch.l,
        chroma: oklch.chroma,
        hue: oklch.hue.into_positive_degrees(),
    }
}

/// Converts an [`Oklch`] color back into [`Hsla`], using `alpha` for the
/// resulting alpha channel. Channels outside the sRGB gamut are clamped.
pub fn oklch_to_hsla(color: Oklch, alpha: f32) -> Hsla {
    let oklch = palette::Oklch {
        l: color.l,
        chroma: color.chroma,
        hue: OklabHue::from_degrees(color.hue),
    };
    let rgba: Srgba = Srgba::from_linear(LinSrgba::from_color(oklch));
    let (red, green, blue, _) = rgba.into_components();
    Hsla::from(Rgba {
        r: red.clamp(0.0, 1.0),
        g: green.clamp(0.0, 1.0),
        b: blue.clamp(0.0, 1.0),
        a: alpha,
    })
}

fn hsla_to_linear(color: Hsla) -> LinSrgba {
    let rgba = Rgba::from(color);
    Srgba::new(rgba.r, rgba.g, rgba.b, rgba.a).into_linear()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::hsla;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() < tolerance
    }

    #[test]
    fn white_is_max_lightness_and_black_is_zero() {
        let white = hsla_to_oklab(hsla(0., 0., 1., 1.));
        let black = hsla_to_oklab(hsla(0., 0., 0., 1.));
        assert!(close(white.l, 1.0, 1e-3), "白色应接近 L=1: {white:?}");
        assert!(close(black.l, 0.0, 1e-3), "黑色应接近 L=0: {black:?}");
        // 无彩色时 a/b 都接近 0
        assert!(close(white.a, 0.0, 1e-3) && close(white.b, 0.0, 1e-3));
    }

    #[test]
    fn oklch_round_trips_through_hsla() {
        // 取一个饱和度中等的颜色,往返后应回到接近原值
        let original = hsla(0.6, 0.7, 0.5, 1.0);
        let oklch = hsla_to_oklch(original);
        let back = oklch_to_hsla(oklch, 1.0);
        assert!(close(original.h, back.h, 1e-2), "色相: {original:?} vs {back:?}");
        assert!(close(original.s, back.s, 1e-2), "饱和度: {original:?} vs {back:?}");
        assert!(close(original.l, back.l, 1e-2), "明度: {original:?} vs {back:?}");
    }

    #[test]
    fn oklch_alpha_is_taken_from_argument() {
        let oklch = hsla_to_oklch(hsla(0.3, 0.5, 0.5, 1.0));
        let transparent = oklch_to_hsla(oklch, 0.25);
        assert_eq!(transparent.a, 0.25, "alpha 应由参数决定,而非原色");
    }

    #[test]
    fn oklch_hue_is_in_degrees() {
        let oklch = hsla_to_oklch(hsla(0.0, 0.8, 0.5, 1.0));
        assert!(
            (0.0..360.0).contains(&oklch.hue),
            "色相应在 [0,360) 度: {}",
            oklch.hue
        );
    }
}
