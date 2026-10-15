//! APCA 对比度与对比度兜底。
//!
//! 逐字移植 zed 的 `crates/ui/src/utils/apca_contrast.rs`，包括它的测试。
//! 放在 `aa_ui_core` 是因为它**框架无关**：只依赖颜色类型，gpui 与 iced
//! 两侧都能用（zed 里它同样同时服务 editor 与 terminal_view）。
//!
//! ## 为什么不能「取个近似」
//!
//! APCA 的常数表和指数是配套的一套（`main_trc=2.4` 配 `norm_bg=0.56`
//! 之类），动任何一个数都会让输出偏离标准，而表现为「颜色有点怪」—— 看不
//! 出来是错。所以整个文件照搬，只把类型换成我们的 `Hsla`。

use aa_theme_core::{Hsla, black, white};

/// APCA (Accessible Perceptual Contrast Algorithm) constants.
///
/// 基于 APCA 0.0.98G-4g W3 兼容常数，见 <https://github.com/Myndex/apca-w3>
struct APCAConstants {
    /// 显示器感知的主 TRC 指数。
    main_trc: f32,
    /// sRGB 系数。
    s_rco: f32,
    s_gco: f32,
    s_bco: f32,
    /// G-4g 常数，配 2.4 指数使用。
    norm_bg: f32,
    norm_txt: f32,
    rev_txt: f32,
    rev_bg: f32,
    /// G-4g 的夹取与缩放。
    blk_thrs: f32,
    blk_clmp: f32,
    scale_bow: f32,
    scale_wob: f32,
    lo_bow_offset: f32,
    lo_wob_offset: f32,
    delta_y_min: f32,
    lo_clip: f32,
}

impl Default for APCAConstants {
    fn default() -> Self {
        Self {
            main_trc: 2.4,
            s_rco: 0.2126729,
            s_gco: 0.7151522,
            s_bco: 0.0721750,
            norm_bg: 0.56,
            norm_txt: 0.57,
            rev_txt: 0.62,
            rev_bg: 0.65,
            blk_thrs: 0.022,
            blk_clmp: 1.414,
            scale_bow: 1.14,
            scale_wob: 1.14,
            lo_bow_offset: 0.027,
            lo_wob_offset: 0.027,
            delta_y_min: 0.0005,
            lo_clip: 0.1,
        }
    }
}

/// 计算感知明度对比度（APCA），返回约 -108..=106。
///
/// 负值表示「亮字暗底」，正值相反。APCA 比 WCAG 2.x 在深色界面上准得多，
/// 好处是：对暗背景更准、**方向敏感**、整个区间感知均匀。
///
/// ARC Bronze Simple Mode 的常用阈值：
/// - Lc 45：大号流畅文本（36px+）的最低要求
/// - Lc 60：其他内容文本的最低要求
/// - Lc 75：正文最低要求
/// - Lc 90：正文推荐值
///
/// 多数终端主题的 APCA 值在 40..=70。
pub fn apca_contrast(text_color: Hsla, background_color: Hsla) -> f32 {
    let constants = APCAConstants::default();

    let text_y = srgb_to_y(text_color, &constants);
    let bg_y = srgb_to_y(background_color, &constants);

    // 对接近黑色的颜色做软夹取
    let text_y_clamped = if text_y > constants.blk_thrs {
        text_y
    } else {
        text_y + (constants.blk_thrs - text_y).powf(constants.blk_clmp)
    };

    let bg_y_clamped = if bg_y > constants.blk_thrs {
        bg_y
    } else {
        bg_y + (constants.blk_thrs - bg_y).powf(constants.blk_clmp)
    };

    // Y 差极小时返回 0
    if (bg_y_clamped - text_y_clamped).abs() < constants.delta_y_min {
        return 0.0;
    }

    // zed 原样是「先声明后赋值」，clippy 会报 late initialization。改成表达式
    // 不影响数值：sapc 在这里只被用一次。
    let (sapc, output_contrast) = if bg_y_clamped > text_y_clamped {
        // 正常极性：暗字亮底
        let sapc = (bg_y_clamped.powf(constants.norm_bg) - text_y_clamped.powf(constants.norm_txt))
            * constants.scale_bow;
        // 低对比度平滑滚出，防止极性反转
        let output = if sapc < constants.lo_clip {
            0.0
        } else {
            sapc - constants.lo_bow_offset
        };
        (sapc, output)
    } else {
        // 反极性：亮字暗底
        let sapc = (bg_y_clamped.powf(constants.rev_bg) - text_y_clamped.powf(constants.rev_txt))
            * constants.scale_wob;
        let output = if sapc > -constants.lo_clip {
            0.0
        } else {
            sapc + constants.lo_wob_offset
        };
        (sapc, output)
    };
    let _ = sapc;

    output_contrast * 100.0
}

/// sRGB → APCA 的 Y（相对亮度）。
fn srgb_to_y(color: Hsla, constants: &APCAConstants) -> f32 {
    let rgba = aa_theme_core::Rgba::from(color);

    // 线性化并套用系数
    let r_linear = (rgba.r).powf(constants.main_trc);
    let g_linear = (rgba.g).powf(constants.main_trc);
    let b_linear = (rgba.b).powf(constants.main_trc);

    constants.s_rco * r_linear + constants.s_gco * g_linear + constants.s_bco * b_linear
}

/// 把前景色调到与背景的最低 APCA 对比度之上。
///
/// `minimum_apca_contrast` 传绝对值（例如 75 表示 Lc 75）。
///
/// 三级策略，逐级放宽，尽量保住色相和饱和度：
/// 1. 只调明度（保住 hue/saturation）
/// 2. 同时降饱和度
/// 3. 退回黑或白，取对比度高的那个
pub fn ensure_minimum_contrast(
    foreground: Hsla,
    background: Hsla,
    minimum_apca_contrast: f32,
) -> Hsla {
    if minimum_apca_contrast <= 0.0 {
        return foreground;
    }

    let current_contrast = apca_contrast(foreground, background).abs();

    if current_contrast >= minimum_apca_contrast {
        return foreground;
    }

    // 第一级：只调明度
    let adjusted = adjust_lightness_for_contrast(foreground, background, minimum_apca_contrast);

    let adjusted_contrast = apca_contrast(adjusted, background).abs();
    if adjusted_contrast >= minimum_apca_contrast {
        return adjusted;
    }

    // 第二级：同时降饱和度
    let desaturated =
        adjust_lightness_and_saturation_for_contrast(foreground, background, minimum_apca_contrast);

    let desaturated_contrast = apca_contrast(desaturated, background).abs();
    if desaturated_contrast >= minimum_apca_contrast {
        return desaturated;
    }

    // 最后手段：黑或白
    let black = Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: foreground.a,
    };

    let white = Hsla {
        h: 0.0,
        s: 0.0,
        l: 1.0,
        a: foreground.a,
    };

    let black_contrast = apca_contrast(black, background).abs();
    let white_contrast = apca_contrast(white, background).abs();

    if white_contrast > black_contrast {
        white
    } else {
        black
    }
}

/// 只调明度，保住色相与饱和度。
fn adjust_lightness_for_contrast(
    foreground: Hsla,
    background: Hsla,
    minimum_apca_contrast: f32,
) -> Hsla {
    // 先定方向：背景亮就该往暗调，反之往亮调
    let bg_luminance = srgb_to_y(background, &APCAConstants::default());
    let should_go_darker = bg_luminance > 0.5;

    // 二分查找最优明度
    let mut low = if should_go_darker { 0.0 } else { foreground.l };
    let mut high = if should_go_darker { foreground.l } else { 1.0 };
    let mut best_l = foreground.l;

    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        let test_color = Hsla {
            h: foreground.h,
            s: foreground.s,
            l: mid,
            a: foreground.a,
        };

        let contrast = apca_contrast(test_color, background).abs();

        if contrast >= minimum_apca_contrast {
            best_l = mid;
            // 尽量贴近最低要求，别一味拉满
            if should_go_darker {
                low = mid;
            } else {
                high = mid;
            }
        } else if should_go_darker {
            high = mid;
        } else {
            low = mid;
        }

        // 足够接近目标就停
        if (contrast - minimum_apca_contrast).abs() < 1.0 {
            best_l = mid;
            break;
        }
    }

    Hsla {
        h: foreground.h,
        s: foreground.s,
        l: best_l,
        a: foreground.a,
    }
}

/// 同时调明度与饱和度。
fn adjust_lightness_and_saturation_for_contrast(
    foreground: Hsla,
    background: Hsla,
    minimum_apca_contrast: f32,
) -> Hsla {
    // 逐级降饱和度
    let saturation_steps = [1.0, 0.8, 0.6, 0.4, 0.2, 0.0];

    for &sat_multiplier in &saturation_steps {
        let test_color = Hsla {
            h: foreground.h,
            s: foreground.s * sat_multiplier,
            l: foreground.l,
            a: foreground.a,
        };

        let adjusted = adjust_lightness_for_contrast(test_color, background, minimum_apca_contrast);
        let contrast = apca_contrast(adjusted, background).abs();

        if contrast >= minimum_apca_contrast {
            return adjusted;
        }
    }

    // 连灰阶都不行，返回灰阶那次的结果
    Hsla {
        h: foreground.h,
        s: 0.0,
        l: foreground.l,
        a: foreground.a,
    }
}

// 让 black()/white() 常量的 import 不报未使用：它们在 ensure_minimum_contrast
// 里被结构体字面量取代了，这里留作语义参照。
#[allow(dead_code)]
fn _palette_reference(h: f32, s: f32, l: f32) -> Hsla {
    let _ = (black(), white());
    Hsla { h, s, l, a: 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
        Hsla { h, s, l, a }
    }

    fn hsla_from_hex(hex: u32) -> Hsla {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if max == min {
            Hsla {
                h: 0.0,
                s: 0.0,
                l,
                a: 1.0,
            }
        } else {
            let d = max - min;
            let s = if l > 0.5 {
                d / (2.0 - max - min)
            } else {
                d / (max + min)
            };
            let h = if max == r {
                (g - b) / d + if g < b { 6.0 } else { 0.0 }
            } else if max == g {
                (b - r) / d + 2.0
            } else {
                (r - g) / d + 4.0
            } / 6.0;
            Hsla { h, s, l, a: 1.0 }
        }
    }

    /// 以下 5 个测试逐字搬自 zed 的 apca_contrast.rs：移植的正确性只能靠
    /// 同一套断言来证，自己另写一套等于自己给自己打分。
    #[test]
    fn test_apca_contrast() {
        let black = hsla(0.0, 0.0, 0.0, 1.0);
        let white = hsla(0.0, 0.0, 1.0, 1.0);
        let contrast = apca_contrast(black, white);
        assert!(
            contrast > 100.0,
            "Black on white should have high positive contrast, got {}",
            contrast
        );

        let contrast_reversed = apca_contrast(white, black);
        assert!(
            contrast_reversed < -100.0,
            "White on black should have high negative contrast, got {}",
            contrast_reversed
        );

        let gray = hsla(0.0, 0.0, 0.5, 1.0);
        let contrast_same = apca_contrast(gray, gray);
        assert!(
            contrast_same.abs() < 1.0,
            "Same color should have near-zero contrast, got {}",
            contrast_same
        );

        // APCA 不可交换：极性是算法的一部分
        assert!(
            (contrast + contrast_reversed).abs() > 1.0,
            "APCA should not be commutative"
        );
    }

    #[test]
    fn test_srgb_to_y() {
        let constants = APCAConstants::default();
        let black = hsla(0.0, 0.0, 0.0, 1.0);
        let y_black = srgb_to_y(black, &constants);
        assert!(
            y_black.abs() < 0.001,
            "Black should have Y near 0, got {}",
            y_black
        );

        let white = hsla(0.0, 0.0, 1.0, 1.0);
        let y_white = srgb_to_y(white, &constants);
        assert!(
            (y_white - 1.0).abs() < 0.001,
            "White should have Y near 1, got {}",
            y_white
        );
    }

    #[test]
    fn test_srgb_to_y_nan_issue() {
        // 深红在 2.4 指数下容易出 NaN（0 的负次幂），这条守住那个回归。
        let dark_red = hsla_from_hex(0x5f0000);
        let y_dark_red = srgb_to_y(dark_red, &APCAConstants::default());
        assert!(!y_dark_red.is_nan());
    }

    #[test]
    fn test_ensure_minimum_contrast() {
        let white_bg = hsla(0.0, 0.0, 1.0, 1.0);
        let light_gray = hsla(0.0, 0.0, 0.9, 1.0);

        let initial_contrast = apca_contrast(light_gray, white_bg).abs();
        assert!(
            initial_contrast < 15.0,
            "Initial contrast should be low, got {}",
            initial_contrast
        );

        let adjusted = ensure_minimum_contrast(light_gray, white_bg, 45.0);
        assert_eq!(adjusted.l, 0.0, "亮底上的浅灰应被调到黑");
        assert_eq!(adjusted.a, light_gray.a, "alpha 必须保留");

        let black_bg = hsla(0.0, 0.0, 0.0, 1.0);
        let dark_gray = hsla(0.0, 0.0, 0.1, 1.0);
        let initial_contrast = apca_contrast(dark_gray, black_bg).abs();
        assert!(
            initial_contrast < 15.0,
            "Initial contrast should be low, got {}",
            initial_contrast
        );

        let adjusted = ensure_minimum_contrast(dark_gray, black_bg, 45.0);
        assert_eq!(adjusted.l, 1.0, "暗底上的深灰应被调到白");

        // 对比度已够时不该动
        let black = hsla(0.0, 0.0, 0.0, 1.0);
        assert_eq!(ensure_minimum_contrast(black, white_bg, 45.0), black);
    }

    #[test]
    fn test_one_light_theme_exact_colors() {
        // One Light 主题里 background 与 ansi.white 都是 #fafafaff ——
        // 真实会撞车的配色，保证算法能把它救回来。
        let fafafa = hsla_from_hex(0xfafafa);

        let contrast = apca_contrast(fafafa, fafafa);
        assert!(
            contrast.abs() < 1.0,
            "Same color should have near-zero APCA contrast, got {}",
            contrast
        );

        let adjusted = ensure_minimum_contrast(fafafa, fafafa, 15.0);
        let new_contrast = apca_contrast(adjusted, fafafa).abs();
        assert!(
            new_contrast >= 15.0,
            "Adjusted contrast {} should be >= 15.0",
            new_contrast
        );
    }
}
