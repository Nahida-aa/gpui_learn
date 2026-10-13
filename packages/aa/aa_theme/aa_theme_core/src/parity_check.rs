use crate::color::{
    Hsla as C, Rgba as CRgba, black, blue, green, hsla, opaque_grey, red, transparent_black,
    transparent_white, white, yellow,
};

fn assert_same(name: &str, g: gpui::Hsla, c: C) {
    assert_eq!(g.h.to_bits(), c.h.to_bits(), "{name}.h");
    assert_eq!(g.s.to_bits(), c.s.to_bits(), "{name}.s");
    assert_eq!(g.l.to_bits(), c.l.to_bits(), "{name}.l");
    assert_eq!(g.a.to_bits(), c.a.to_bits(), "{name}.a");
    let g_rgb: gpui::Rgba = g.into();
    let c_rgb: CRgba = c.into();
    assert_eq!(g_rgb.r.to_bits(), c_rgb.r.to_bits(), "{name}->r");
    assert_eq!(g_rgb.g.to_bits(), c_rgb.g.to_bits(), "{name}->g");
    assert_eq!(g_rgb.b.to_bits(), c_rgb.b.to_bits(), "{name}->b");
    assert_eq!(g_rgb.a.to_bits(), c_rgb.a.to_bits(), "{name}->a");
    let gb: gpui::Hsla = g_rgb.into();
    let cb: C = c_rgb.into();
    assert_eq!(gb.h.to_bits(), cb.h.to_bits(), "{name} back h");
    assert_eq!(gb.s.to_bits(), cb.s.to_bits(), "{name} back s");
    assert_eq!(gb.l.to_bits(), cb.l.to_bits(), "{name} back l");
}

/// 9 个命名颜色全部纳入：上一轮漏了 green/yellow，正是漏检的那两个。
#[test]
fn named_colors_match_gpui() {
    let pairs: Vec<(&str, gpui::Hsla, C)> = vec![
        ("black", gpui::black(), black()),
        ("white", gpui::white(), white()),
        ("red", gpui::red(), red()),
        ("blue", gpui::blue(), blue()),
        ("green", gpui::green(), green()),
        ("yellow", gpui::yellow(), yellow()),
        (
            "transparent_black",
            gpui::transparent_black(),
            transparent_black(),
        ),
        (
            "transparent_white",
            gpui::transparent_white(),
            transparent_white(),
        ),
    ];
    for (n, g, c) in pairs {
        assert_same(n, g, c);
    }
    assert_eq!(
        opaque_grey(0.42, 0.75),
        C {
            h: 0.,
            s: 0.,
            l: 0.42,
            a: 0.75
        }
    );
    assert_same(
        "opaque_grey",
        gpui::opaque_grey(0.42, 0.75),
        opaque_grey(0.42, 0.75),
    );
    // clamp 行为也要一致
    assert_same(
        "opaque_grey clamped",
        gpui::opaque_grey(2.0, -1.0),
        opaque_grey(2.0, -1.0),
    );
    assert_same(
        "hsla clamped",
        gpui::hsla(1.5, 2.0, -0.5, 3.0),
        hsla(1.5, 2.0, -0.5, 3.0),
    );
}

#[test]
fn sampled_and_method_colors_match_gpui() {
    for g in [
        gpui::hsla(0.6317829, 0.6971, 0.7805, 1.0),
        gpui::hsla(0.3333, 0.5, 0.5, 0.25),
        gpui::hsla(0.8333, 0.9, 0.2, 0.6),
        gpui::hsla(0.6, 0.7, 0.5, 0.4),
    ] {
        assert_same(
            "sample",
            g,
            C {
                h: g.h,
                s: g.s,
                l: g.l,
                a: g.a,
            },
        );
    }

    let bg = gpui::hsla(0.6, 0.7, 0.5, 0.4);
    let bc = C {
        h: 0.6,
        s: 0.7,
        l: 0.5,
        a: 0.4,
    };
    let other_g = gpui::hsla(0.1, 0.2, 0.3, 0.5);
    let other_c = C {
        h: 0.1,
        s: 0.2,
        l: 0.3,
        a: 0.5,
    };
    assert_same("blend", bg.blend(other_g), bc.blend(other_c));
    assert_same("grayscale", bg.grayscale(), bc.grayscale());
    assert_same("opacity", bg.opacity(0.33), bc.opacity(0.33));
    assert_same("alpha", bg.alpha(0.77), bc.alpha(0.77));

    let mut g = bg;
    let mut c = bc;
    g.fade_out(0.4);
    c.fade_out(0.4);
    assert_same("fade_out", g, c);
}
