//! `aa_theme_core` 的颜色数学与 `gpui::color` 逐位一致。
//!
//! ## 为什么这份校验住在 aa_theme_gpui 而不是 aa_theme_core
//!
//! 一致性是「core 与 gpui 之间的关系」，而 gpui 侧才是依赖方 —— 把比较写在
//! core 里就得让 core 的 dev-dependencies 拖进 gpui 整棵树（编译代价），
//! 而且 `#[cfg(test)] mod` 里出现 `gpui::` 会打破「core 不认识任何后端」的
//! 边界：core 的编译期一旦依赖 gpui，它就再也无法被 iced 单独使用。
//!
//! 放在这里则 `gpui` 已是既有依赖，无需新增任何东西。
//!
//! ## 迁移完成后
//!
//! gpui 改为 `pub use aa_theme_core::{Hsla, Rgba}` 之后，两边是同一个类型，
//! 本文件会退化成恒真断言，应直接删除。

use aa_theme_core::{
    Hsla as CoreHsla, Rgba as CoreRgba, black, blue, green, hsla, opaque_grey, red,
    transparent_black, transparent_white, white, yellow,
};

fn core(color: gpui::Hsla) -> CoreHsla {
    CoreHsla {
        h: color.h,
        s: color.s,
        l: color.l,
        a: color.a,
    }
}

/// 双向 + 命名常量的逐位比对。`to_bits()` 而非 `==`：HSL→RGB→HSL 的往返在
/// 末位有 1e-7 量级浮点差异（gpui 自己的测试也这么写），`==` 会误报。
fn assert_same(name: &str, gpui_color: gpui::Hsla, core_color: CoreHsla) {
    assert_eq!(gpui_color.h.to_bits(), core_color.h.to_bits(), "{name}: h");
    assert_eq!(gpui_color.s.to_bits(), core_color.s.to_bits(), "{name}: s");
    assert_eq!(gpui_color.l.to_bits(), core_color.l.to_bits(), "{name}: l");
    assert_eq!(gpui_color.a.to_bits(), core_color.a.to_bits(), "{name}: a");

    // Hsla -> Rgba
    let gpui_rgb: gpui::Rgba = gpui_color.into();
    let core_rgb: CoreRgba = core_color.into();
    assert_eq!(gpui_rgb.r.to_bits(), core_rgb.r.to_bits(), "{name}: ->r");
    assert_eq!(gpui_rgb.g.to_bits(), core_rgb.g.to_bits(), "{name}: ->g");
    assert_eq!(gpui_rgb.b.to_bits(), core_rgb.b.to_bits(), "{name}: ->b");
    assert_eq!(gpui_rgb.a.to_bits(), core_rgb.a.to_bits(), "{name}: ->a");

    // Rgba -> Hsla（回程同样要逐位一致，不能只测去程）
    let gpui_back: gpui::Hsla = gpui_rgb.into();
    let core_back: CoreHsla = core_rgb.into();
    assert_eq!(
        gpui_back.h.to_bits(),
        core_back.h.to_bits(),
        "{name}: 回程 h"
    );
    assert_eq!(
        gpui_back.s.to_bits(),
        core_back.s.to_bits(),
        "{name}: 回程 s"
    );
    assert_eq!(
        gpui_back.l.to_bits(),
        core_back.l.to_bits(),
        "{name}: 回程 l"
    );
}

/// 9 个命名常量全部纳入 —— 早期版本只采样了其中 4 个，恰好漏掉了 `green`
/// 与 `yellow`，而正是这两个抄错了（green 的 lightness 应为 0.25 而非 0.5）。
#[test]
fn named_colors_match_gpui() {
    let pairs: Vec<(&str, gpui::Hsla, CoreHsla)> = vec![
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
        (
            "opaque_grey",
            gpui::opaque_grey(0.42, 0.75),
            opaque_grey(0.42, 0.75),
        ),
        // clamp 边界也要一致
        (
            "opaque_grey 超界",
            gpui::opaque_grey(2.0, -1.0),
            opaque_grey(2.0, -1.0),
        ),
        (
            "hsla 超界",
            gpui::hsla(1.5, 2.0, -0.5, 3.0),
            hsla(1.5, 2.0, -0.5, 3.0),
        ),
    ];
    for (name, gpui_color, core_color) in pairs {
        assert_same(name, gpui_color, core_color);
    }
}

#[test]
fn sampled_colors_match_gpui() {
    let samples = [
        gpui::hsla(0.6317829, 0.6971, 0.7805, 1.0),
        gpui::hsla(0.3333, 0.5, 0.5, 0.25),
        gpui::hsla(0.8333, 0.9, 0.2, 0.6),
        gpui::hsla(0.6, 0.7, 0.5, 0.4),
        gpui::hsla(0.99, 0.01, 0.99, 0.01),
        gpui::hsla(0.01, 0.99, 0.01, 0.99),
    ];
    for color in samples {
        assert_same("sample", color, core(color));
    }
}

#[test]
fn color_methods_match_gpui() {
    let base_gpui = gpui::hsla(0.6, 0.7, 0.5, 0.4);
    let base_core = core(base_gpui);
    let other_gpui = gpui::hsla(0.1, 0.2, 0.3, 0.5);
    let other_core = core(other_gpui);

    // other 保持半透明：a>=1.0 时 blend 直接短路返回 other，走不到混合逻辑。
    assert_same(
        "blend",
        base_gpui.blend(other_gpui),
        base_core.blend(other_core),
    );
    assert_same("grayscale", base_gpui.grayscale(), base_core.grayscale());
    assert_same("opacity", base_gpui.opacity(0.33), base_core.opacity(0.33));
    assert_same("alpha", base_gpui.alpha(0.77), base_core.alpha(0.77));

    // fade_out 是原地修改
    let mut gpui_faded = base_gpui;
    let mut core_faded = base_core;
    gpui_faded.fade_out(0.4);
    core_faded.fade_out(0.4);
    assert_same("fade_out", gpui_faded, core_faded);

    // blend 的短路分支同样要一致
    assert_same(
        "blend 不透明",
        base_gpui.blend(gpui::white()),
        base_core.blend(white()),
    );
    assert_same(
        "blend 全透明",
        base_gpui.blend(gpui::transparent_black()),
        base_core.blend(transparent_black()),
    );
}

#[test]
fn hex_parsing_agrees_on_accepted_and_rejected_input() {
    for input in [
        "#abc",
        "#abcd",
        "#ff8000",
        "#ff800080",
        "#00000000",
        "#ffffff",
        "#000",
    ] {
        let gpui_rgba = gpui::Rgba::try_from(input)
            .unwrap_or_else(|err| panic!("gpui 应接受 {input:?}: {err}"));
        let core_rgba: CoreRgba = input
            .try_into()
            .unwrap_or_else(|err| panic!("core 应接受 {input:?}: {err}"));
        assert_eq!(gpui_rgba.r.to_bits(), core_rgba.r.to_bits(), "{input}: r");
        assert_eq!(gpui_rgba.g.to_bits(), core_rgba.g.to_bits(), "{input}: g");
        assert_eq!(gpui_rgba.b.to_bits(), core_rgba.b.to_bits(), "{input}: b");
        assert_eq!(gpui_rgba.a.to_bits(), core_rgba.a.to_bits(), "{input}: a");
    }

    // 注意 "#ff80" 不在此列：4 位是合法的 #rgba。
    for input in ["", "ff8000", "#ff80000", "#ggg", "rgb(1,2,3)"] {
        assert!(
            gpui::Rgba::try_from(input).is_err(),
            "gpui 应拒绝 {input:?}"
        );
        assert!(CoreRgba::try_from(input).is_err(), "core 应拒绝 {input:?}");
    }
}

#[test]
fn serde_representation_agrees() {
    for input in ["#abc", "#ff8000", "#ff800080", "#cad3f5"] {
        let gpui_color: gpui::Hsla = serde_json::from_str(&format!("\"{input}\"")).unwrap();
        let core_color: CoreHsla = serde_json::from_str(&format!("\"{input}\"")).unwrap();
        assert_eq!(gpui_color.h.to_bits(), core_color.h.to_bits(), "{input}: h");
        assert_eq!(
            serde_json::to_string(&gpui_color).unwrap(),
            serde_json::to_string(&core_color).unwrap(),
            "{input}: 回写字符串"
        );
    }
}

/// 共享色板的前提：core 的 `try_parse_color` 与 gpui 侧
/// `aa_theme_gpui::try_parse_color` 必须给出同一个色值。
///
/// 这一条比类型层面的比对更重要 —— 它保证同一份调色板在两个后端下
/// 解析出的颜色不会出现 1e-7 的漂移。
#[test]
fn try_parse_color_agrees_with_gpui_side() {
    // 覆盖 gpui::Rgba::try_from 支持的全部格式（#rgb / #rgba / #rrggbb /
    // #rrggbbaa）+ 边界值。core 的 color.rs 是 gpui color.rs 的**重构副本而非
    // 逐字副本**（两份文件 diff 有 679 行差异），所以「Hsla::from 两边一样」
    // 推不出「Rgba::try_from 两边一样」—— 只能靠这份语料逐位验证。
    // 色值取 0x00 / 0x7f / 0x80 / 0xff 等易在整除与舍入上分叉的边界。
    let mut corpus: Vec<String> = [
        "#24273a",
        "#cad3f5",
        "#ed8796",
        "#1e1e2e",
        "#89b4fa",
        "#f38ba8",
        "#000000",
        "#ffffff",
        "#ff0000",
        "#00ff00",
        "#0000ff",
        "#010203",
        "#00000000",
        "#ffffffff",
        "#7f7f7f80",
        "#80808080",
        "#ff00007f",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // 只生成合法长度：#rgb(3) / #rgba(4) / #rrggbb(6) / #rrggbbaa(8)
    for d in ["0", "7", "8", "f"] {
        for n in [3usize, 4, 6, 8] {
            corpus.push(format!("#{}", d.repeat(n)));
        }
    }

    for hex in &corpus {
        let via_gpui = aa_theme_gpui::try_parse_color(hex).expect("gpui 侧解析");
        let via_core = aa_theme_core::try_parse_color(hex).expect("core 侧解析");
        assert_eq!(via_gpui.h.to_bits(), via_core.h.to_bits(), "{hex}: h");
        assert_eq!(via_gpui.s.to_bits(), via_core.s.to_bits(), "{hex}: s");
        assert_eq!(via_gpui.l.to_bits(), via_core.l.to_bits(), "{hex}: l");
        assert_eq!(via_gpui.a.to_bits(), via_core.a.to_bits(), "{hex}: a");
    }
}

/// 反向：非法输入两边必须**同时**失败。core 若放宽了格式（比如接受了
/// `rgb(...)` 函数式写法），就会出现「gpui 侧报错、core 侧接受」的不对称，
/// 那会让上游主题文件在某个后端下悄悄解析出意料之外的值。
#[test]
fn try_parse_color_rejects_the_same_invalid_inputs() {
    for bad in [
        "",
        "rgb(1,2,3)",
        "24273a",
        "#",
        "#12",
        "#1234567",
        "#gggggg",
        " # ",
    ] {
        assert!(
            aa_theme_gpui::try_parse_color(bad).is_err(),
            "gpui 侧本应拒绝 {bad:?}"
        );
        assert!(
            aa_theme_core::try_parse_color(bad).is_err(),
            "core 侧本应拒绝 {bad:?}"
        );
    }
}
