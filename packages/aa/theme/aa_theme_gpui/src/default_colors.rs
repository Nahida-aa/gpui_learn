use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, Rgba, hsla};

use crate::ColorScale;
use crate::ThemeColors;
use crate::scale::{ColorScaleSet, ColorScales};
use crate::styles::{StatusColors, SyntaxTheme};

pub(crate) fn neutral() -> ColorScaleSet {
    sand()
}

// git diff 的语义色（zed 同款常量）。`pub(crate)`：CTP 配色在
// `fallback_themes` 里补齐「zed 默认」字段时要引用。
pub(crate) const ADDED_COLOR: Hsla = Hsla {
    h: 134. / 360.,
    s: 0.55,
    l: 0.40,
    a: 1.0,
};
pub(crate) const WORD_ADDED_COLOR: Hsla = Hsla {
    h: 134. / 360.,
    s: 0.55,
    l: 0.40,
    a: 0.35,
};
pub(crate) const REMOVED_COLOR: Hsla = Hsla {
    h: 350. / 360.,
    s: 0.88,
    l: 0.25,
    a: 1.0,
};
pub(crate) const WORD_DELETED_COLOR: Hsla = Hsla {
    h: 350. / 360.,
    s: 0.88,
    l: 0.25,
    a: 0.80,
};
/// 主题的默认基底。
///
/// Themes that do not specify all colors are refined off of these defaults.
///
/// 本仓库的「真默认」就是 Catppuccin 风格：浅色 = Latte、深色 = Macchiato。
/// 配色实现就在本文件末尾（`theme_colors_{latte,macchiato}`）；`fallback_themes`
/// 只负责兜底家族组装与「缺失字段补齐」规则，依赖方向与 zed 一致（单向）。
impl ThemeColors {
    /// 本仓库的浅色基底：Catppuccin Latte。
    pub fn light() -> Self {
        theme_colors_latte()
    }

    /// 本仓库的深色基底：Catppuccin Macchiato。
    pub fn dark() -> Self {
        theme_colors_macchiato()
    }
}

type StaticColorScale = [&'static str; 12];

struct StaticColorScaleSet {
    scale: &'static str,
    light: StaticColorScale,
    light_alpha: StaticColorScale,
    dark: StaticColorScale,
    dark_alpha: StaticColorScale,
}

impl TryFrom<StaticColorScaleSet> for ColorScaleSet {
    type Error = anyhow::Error;

    fn try_from(value: StaticColorScaleSet) -> Result<Self, Self::Error> {
        fn to_color_scale(scale: StaticColorScale) -> anyhow::Result<ColorScale> {
            scale
                .into_iter()
                .map(|color| Rgba::try_from(color).map(Hsla::from))
                .collect::<Result<Vec<_>, _>>()
                .map(ColorScale::from_iter)
        }

        Ok(Self::new(
            value.scale,
            to_color_scale(value.light)?,
            to_color_scale(value.light_alpha)?,
            to_color_scale(value.dark)?,
            to_color_scale(value.dark_alpha)?,
        ))
    }
}

/// Color scales used to build the default themes.
pub fn default_color_scales() -> ColorScales {
    ColorScales {
        gray: gray(),
        mauve: mauve(),
        slate: slate(),
        sage: sage(),
        olive: olive(),
        sand: sand(),
        gold: gold(),
        bronze: bronze(),
        brown: brown(),
        yellow: yellow(),
        amber: amber(),
        orange: orange(),
        tomato: tomato(),
        red: red(),
        ruby: ruby(),
        crimson: crimson(),
        pink: pink(),
        plum: plum(),
        purple: purple(),
        violet: violet(),
        iris: iris(),
        indigo: indigo(),
        blue: blue(),
        cyan: cyan(),
        teal: teal(),
        jade: jade(),
        green: green(),
        grass: grass(),
        lime: lime(),
        mint: mint(),
        sky: sky(),
        black: black(),
        white: white(),
    }
}

pub(crate) fn gray() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Gray",
        light: [
            "#fcfcfcff",
            "#f9f9f9ff",
            "#f0f0f0ff",
            "#e8e8e8ff",
            "#e0e0e0ff",
            "#d9d9d9ff",
            "#cececeff",
            "#bbbbbbff",
            "#8d8d8dff",
            "#838383ff",
            "#646464ff",
            "#202020ff",
        ],
        light_alpha: [
            "#00000003",
            "#00000006",
            "#0000000f",
            "#00000017",
            "#0000001f",
            "#00000026",
            "#00000031",
            "#00000044",
            "#00000072",
            "#0000007c",
            "#0000009b",
            "#000000df",
        ],
        dark: [
            "#111111ff",
            "#191919ff",
            "#222222ff",
            "#2a2a2aff",
            "#313131ff",
            "#3a3a3aff",
            "#484848ff",
            "#606060ff",
            "#6e6e6eff",
            "#7b7b7bff",
            "#b4b4b4ff",
            "#eeeeeeff",
        ],
        dark_alpha: [
            "#00000000",
            "#ffffff09",
            "#ffffff12",
            "#ffffff1b",
            "#ffffff22",
            "#ffffff2c",
            "#ffffff3b",
            "#ffffff55",
            "#ffffff64",
            "#ffffff72",
            "#ffffffaf",
            "#ffffffed",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn mauve() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Mauve",
        light: [
            "#fdfcfdff",
            "#faf9fbff",
            "#f2eff3ff",
            "#eae7ecff",
            "#e3dfe6ff",
            "#dbd8e0ff",
            "#d0cdd7ff",
            "#bcbac7ff",
            "#8e8c99ff",
            "#84828eff",
            "#65636dff",
            "#211f26ff",
        ],
        light_alpha: [
            "#55005503",
            "#2b005506",
            "#30004010",
            "#20003618",
            "#20003820",
            "#14003527",
            "#10003332",
            "#08003145",
            "#05001d73",
            "#0500197d",
            "#0400119c",
            "#020008e0",
        ],
        dark: [
            "#121113ff",
            "#1a191bff",
            "#232225ff",
            "#2b292dff",
            "#323035ff",
            "#3c393fff",
            "#49474eff",
            "#625f69ff",
            "#6f6d78ff",
            "#7c7a85ff",
            "#b5b2bcff",
            "#eeeef0ff",
        ],
        dark_alpha: [
            "#00000000",
            "#f5f4f609",
            "#ebeaf814",
            "#eee5f81d",
            "#efe6fe25",
            "#f1e6fd30",
            "#eee9ff40",
            "#eee7ff5d",
            "#eae6fd6e",
            "#ece9fd7c",
            "#f5f1ffb7",
            "#fdfdffef",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn slate() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Slate",
        light: [
            "#fcfcfdff",
            "#f9f9fbff",
            "#f0f0f3ff",
            "#e8e8ecff",
            "#e0e1e6ff",
            "#d9d9e0ff",
            "#cdced6ff",
            "#b9bbc6ff",
            "#8b8d98ff",
            "#80838dff",
            "#60646cff",
            "#1c2024ff",
        ],
        light_alpha: [
            "#00005503",
            "#00005506",
            "#0000330f",
            "#00002d17",
            "#0009321f",
            "#00002f26",
            "#00062e32",
            "#00083046",
            "#00051d74",
            "#00071b7f",
            "#0007149f",
            "#000509e3",
        ],
        dark: [
            "#111113ff",
            "#18191bff",
            "#212225ff",
            "#272a2dff",
            "#2e3135ff",
            "#363a3fff",
            "#43484eff",
            "#5a6169ff",
            "#696e77ff",
            "#777b84ff",
            "#b0b4baff",
            "#edeef0ff",
        ],
        dark_alpha: [
            "#00000000",
            "#d8f4f609",
            "#ddeaf814",
            "#d3edf81d",
            "#d9edfe25",
            "#d6ebfd30",
            "#d9edff40",
            "#d9edff5d",
            "#dfebfd6d",
            "#e5edfd7b",
            "#f1f7feb5",
            "#fcfdffef",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn sage() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Sage",
        light: [
            "#fbfdfcff",
            "#f7f9f8ff",
            "#eef1f0ff",
            "#e6e9e8ff",
            "#dfe2e0ff",
            "#d7dad9ff",
            "#cbcfcdff",
            "#b8bcbaff",
            "#868e8bff",
            "#7c8481ff",
            "#5f6563ff",
            "#1a211eff",
        ],
        light_alpha: [
            "#00804004",
            "#00402008",
            "#002d1e11",
            "#001f1519",
            "#00180820",
            "#00140d28",
            "#00140a34",
            "#000f0847",
            "#00110b79",
            "#00100a83",
            "#000a07a0",
            "#000805e5",
        ],
        dark: [
            "#101211ff",
            "#171918ff",
            "#202221ff",
            "#272a29ff",
            "#2e3130ff",
            "#373b39ff",
            "#444947ff",
            "#5b625fff",
            "#63706bff",
            "#717d79ff",
            "#adb5b2ff",
            "#eceeedff",
        ],
        dark_alpha: [
            "#00000000",
            "#f0f2f108",
            "#f3f5f412",
            "#f2fefd1a",
            "#f1fbfa22",
            "#edfbf42d",
            "#edfcf73c",
            "#ebfdf657",
            "#dffdf266",
            "#e5fdf674",
            "#f4fefbb0",
            "#fdfffeed",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn olive() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Olive",
        light: [
            "#fcfdfcff",
            "#f8faf8ff",
            "#eff1efff",
            "#e7e9e7ff",
            "#dfe2dfff",
            "#d7dad7ff",
            "#cccfccff",
            "#b9bcb8ff",
            "#898e87ff",
            "#7f847dff",
            "#60655fff",
            "#1d211cff",
        ],
        light_alpha: [
            "#00550003",
            "#00490007",
            "#00200010",
            "#00160018",
            "#00180020",
            "#00140028",
            "#000f0033",
            "#040f0047",
            "#050f0078",
            "#040e0082",
            "#020a00a0",
            "#010600e3",
        ],
        dark: [
            "#111210ff",
            "#181917ff",
            "#212220ff",
            "#282a27ff",
            "#2f312eff",
            "#383a36ff",
            "#454843ff",
            "#5c625bff",
            "#687066ff",
            "#767d74ff",
            "#afb5adff",
            "#eceeecff",
        ],
        dark_alpha: [
            "#00000000",
            "#f1f2f008",
            "#f4f5f312",
            "#f3fef21a",
            "#f2fbf122",
            "#f4faed2c",
            "#f2fced3b",
            "#edfdeb57",
            "#ebfde766",
            "#f0fdec74",
            "#f6fef4b0",
            "#fdfffded",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn sand() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Sand",
        light: [
            "#fdfdfcff",
            "#f9f9f8ff",
            "#f1f0efff",
            "#e9e8e6ff",
            "#e2e1deff",
            "#dad9d6ff",
            "#cfcecaff",
            "#bcbbb5ff",
            "#8d8d86ff",
            "#82827cff",
            "#63635eff",
            "#21201cff",
        ],
        light_alpha: [
            "#55550003",
            "#25250007",
            "#20100010",
            "#1f150019",
            "#1f180021",
            "#19130029",
            "#19140035",
            "#1915014a",
            "#0f0f0079",
            "#0c0c0083",
            "#080800a1",
            "#060500e3",
        ],
        dark: [
            "#111110ff",
            "#191918ff",
            "#222221ff",
            "#2a2a28ff",
            "#31312eff",
            "#3b3a37ff",
            "#494844ff",
            "#62605bff",
            "#6f6d66ff",
            "#7c7b74ff",
            "#b5b3adff",
            "#eeeeecff",
        ],
        dark_alpha: [
            "#00000000",
            "#f4f4f309",
            "#f6f6f513",
            "#fefef31b",
            "#fbfbeb23",
            "#fffaed2d",
            "#fffbed3c",
            "#fff9eb57",
            "#fffae965",
            "#fffdee73",
            "#fffcf4b0",
            "#fffffded",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn gold() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Gold",
        light: [
            "#fdfdfcff",
            "#faf9f2ff",
            "#f2f0e7ff",
            "#eae6dbff",
            "#e1dccfff",
            "#d8d0bfff",
            "#cbc0aaff",
            "#b9a88dff",
            "#978365ff",
            "#8c7a5eff",
            "#71624bff",
            "#3b352bff",
        ],
        light_alpha: [
            "#55550003",
            "#9d8a000d",
            "#75600018",
            "#6b4e0024",
            "#60460030",
            "#64440040",
            "#63420055",
            "#633d0072",
            "#5332009a",
            "#492d00a1",
            "#362100b4",
            "#130c00d4",
        ],
        dark: [
            "#121211ff",
            "#1b1a17ff",
            "#24231fff",
            "#2d2b26ff",
            "#38352eff",
            "#444039ff",
            "#544f46ff",
            "#696256ff",
            "#978365ff",
            "#a39073ff",
            "#cbb99fff",
            "#e8e2d9ff",
        ],
        dark_alpha: [
            "#91911102",
            "#f9e29d0b",
            "#f8ecbb15",
            "#ffeec41e",
            "#feecc22a",
            "#feebcb37",
            "#ffedcd48",
            "#fdeaca5f",
            "#ffdba690",
            "#fedfb09d",
            "#fee7c6c8",
            "#fef7ede7",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn bronze() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Bronze",
        light: [
            "#fdfcfcff",
            "#fdf7f5ff",
            "#f6edeaff",
            "#efe4dfff",
            "#e7d9d3ff",
            "#dfcdc5ff",
            "#d3bcb3ff",
            "#c2a499ff",
            "#a18072ff",
            "#957468ff",
            "#7d5e54ff",
            "#43302bff",
        ],
        light_alpha: [
            "#55000003",
            "#cc33000a",
            "#92250015",
            "#80280020",
            "#7423002c",
            "#7324003a",
            "#6c1f004c",
            "#671c0066",
            "#551a008d",
            "#4c150097",
            "#3d0f00ab",
            "#1d0600d4",
        ],
        dark: [
            "#141110ff",
            "#1c1917ff",
            "#262220ff",
            "#302a27ff",
            "#3b3330ff",
            "#493e3aff",
            "#5a4c47ff",
            "#6f5f58ff",
            "#a18072ff",
            "#ae8c7eff",
            "#d4b3a5ff",
            "#ede0d9ff",
        ],
        dark_alpha: [
            "#d1110004",
            "#fbbc910c",
            "#faceb817",
            "#facdb622",
            "#ffd2c12d",
            "#ffd1c03c",
            "#fdd0c04f",
            "#ffd6c565",
            "#fec7b09b",
            "#fecab5a9",
            "#ffd7c6d1",
            "#fff1e9ec",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn brown() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Brown",
        light: [
            "#fefdfcff",
            "#fcf9f6ff",
            "#f6eee7ff",
            "#f0e4d9ff",
            "#ebdacaff",
            "#e4cdb7ff",
            "#dcbc9fff",
            "#cea37eff",
            "#ad7f58ff",
            "#a07553ff",
            "#815e46ff",
            "#3e332eff",
        ],
        light_alpha: [
            "#aa550003",
            "#aa550009",
            "#a04b0018",
            "#9b4a0026",
            "#9f4d0035",
            "#a04e0048",
            "#a34e0060",
            "#9f4a0081",
            "#823c00a7",
            "#723300ac",
            "#522100b9",
            "#140600d1",
        ],
        dark: [
            "#12110fff",
            "#1c1816ff",
            "#28211dff",
            "#322922ff",
            "#3e3128ff",
            "#4d3c2fff",
            "#614a39ff",
            "#7c5f46ff",
            "#ad7f58ff",
            "#b88c67ff",
            "#dbb594ff",
            "#f2e1caff",
        ],
        dark_alpha: [
            "#91110002",
            "#fba67c0c",
            "#fcb58c19",
            "#fbbb8a24",
            "#fcb88931",
            "#fdba8741",
            "#ffbb8856",
            "#ffbe8773",
            "#feb87da8",
            "#ffc18cb3",
            "#fed1aad9",
            "#feecd4f2",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn yellow() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Yellow",
        light: [
            "#fdfdf9ff",
            "#fefce9ff",
            "#fffab8ff",
            "#fff394ff",
            "#ffe770ff",
            "#f3d768ff",
            "#e4c767ff",
            "#d5ae39ff",
            "#ffe629ff",
            "#ffdc00ff",
            "#9e6c00ff",
            "#473b1fff",
        ],
        light_alpha: [
            "#aaaa0006",
            "#f4dd0016",
            "#ffee0047",
            "#ffe3016b",
            "#ffd5008f",
            "#ebbc0097",
            "#d2a10098",
            "#c99700c6",
            "#ffe100d6",
            "#ffdc00ff",
            "#9e6c00ff",
            "#2e2000e0",
        ],
        dark: [
            "#14120bff",
            "#1b180fff",
            "#2d2305ff",
            "#362b00ff",
            "#433500ff",
            "#524202ff",
            "#665417ff",
            "#836a21ff",
            "#ffe629ff",
            "#ffff57ff",
            "#f5e147ff",
            "#f6eeb4ff",
        ],
        dark_alpha: [
            "#d1510004",
            "#f9b4000b",
            "#ffaa001e",
            "#fdb70028",
            "#febb0036",
            "#fec40046",
            "#fdcb225c",
            "#fdca327b",
            "#ffe629ff",
            "#ffff57ff",
            "#fee949f5",
            "#fef6baf6",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn amber() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Amber",
        light: [
            "#fefdfbff",
            "#fefbe9ff",
            "#fff7c2ff",
            "#ffee9cff",
            "#fbe577ff",
            "#f3d673ff",
            "#e9c162ff",
            "#e2a336ff",
            "#ffc53dff",
            "#ffba18ff",
            "#ab6400ff",
            "#4f3422ff",
        ],
        light_alpha: [
            "#c0800004",
            "#f4d10016",
            "#ffde003d",
            "#ffd40063",
            "#f8cf0088",
            "#eab5008c",
            "#dc9b009d",
            "#da8a00c9",
            "#ffb300c2",
            "#ffb300e7",
            "#ab6400ff",
            "#341500dd",
        ],
        dark: [
            "#16120cff",
            "#1d180fff",
            "#302008ff",
            "#3f2700ff",
            "#4d3000ff",
            "#5c3d05ff",
            "#714f19ff",
            "#8f6424ff",
            "#ffc53dff",
            "#ffd60aff",
            "#ffca16ff",
            "#ffe7b3ff",
        ],
        dark_alpha: [
            "#e63c0006",
            "#fd9b000d",
            "#fa820022",
            "#fc820032",
            "#fd8b0041",
            "#fd9b0051",
            "#ffab2567",
            "#ffae3587",
            "#ffc53dff",
            "#ffd60aff",
            "#ffca16ff",
            "#ffe7b3ff",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn orange() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Orange",
        light: [
            "#fefcfbff",
            "#fff7edff",
            "#ffefd6ff",
            "#ffdfb5ff",
            "#ffd19aff",
            "#ffc182ff",
            "#f5ae73ff",
            "#ec9455ff",
            "#f76b15ff",
            "#ef5f00ff",
            "#cc4e00ff",
            "#582d1dff",
        ],
        light_alpha: [
            "#c0400004",
            "#ff8e0012",
            "#ff9c0029",
            "#ff91014a",
            "#ff8b0065",
            "#ff81007d",
            "#ed6c008c",
            "#e35f00aa",
            "#f65e00ea",
            "#ef5f00ff",
            "#cc4e00ff",
            "#431200e2",
        ],
        dark: [
            "#17120eff",
            "#1e160fff",
            "#331e0bff",
            "#462100ff",
            "#562800ff",
            "#66350cff",
            "#7e451dff",
            "#a35829ff",
            "#f76b15ff",
            "#ff801fff",
            "#ffa057ff",
            "#ffe0c2ff",
        ],
        dark_alpha: [
            "#ec360007",
            "#fe6d000e",
            "#fb6a0025",
            "#ff590039",
            "#ff61004a",
            "#fd75045c",
            "#ff832c75",
            "#fe84389d",
            "#fe6d15f7",
            "#ff801fff",
            "#ffa057ff",
            "#ffe0c2ff",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn tomato() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Tomato",
        light: [
            "#fffcfcff",
            "#fff8f7ff",
            "#feebe7ff",
            "#ffdcd3ff",
            "#ffcdc2ff",
            "#fdbdafff",
            "#f5a898ff",
            "#ec8e7bff",
            "#e54d2eff",
            "#dd4425ff",
            "#d13415ff",
            "#5c271fff",
        ],
        light_alpha: [
            "#ff000003",
            "#ff200008",
            "#f52b0018",
            "#ff35002c",
            "#ff2e003d",
            "#f92d0050",
            "#e7280067",
            "#db250084",
            "#df2600d1",
            "#d72400da",
            "#cd2200ea",
            "#460900e0",
        ],
        dark: [
            "#181111ff",
            "#1f1513ff",
            "#391714ff",
            "#4e1511ff",
            "#5e1c16ff",
            "#6e2920ff",
            "#853a2dff",
            "#ac4d39ff",
            "#e54d2eff",
            "#ec6142ff",
            "#ff977dff",
            "#fbd3cbff",
        ],
        dark_alpha: [
            "#f1121208",
            "#ff55330f",
            "#ff35232b",
            "#fd201142",
            "#fe332153",
            "#ff4f3864",
            "#fd644a7d",
            "#fe6d4ea7",
            "#fe5431e4",
            "#ff6847eb",
            "#ff977dff",
            "#ffd6cefb",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn red() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Red",
        light: [
            "#fffcfcff",
            "#fff7f7ff",
            "#feebecff",
            "#ffdbdcff",
            "#ffcdceff",
            "#fdbdbeff",
            "#f4a9aaff",
            "#eb8e90ff",
            "#e5484dff",
            "#dc3e42ff",
            "#ce2c31ff",
            "#641723ff",
        ],
        light_alpha: [
            "#ff000003",
            "#ff000008",
            "#f3000d14",
            "#ff000824",
            "#ff000632",
            "#f8000442",
            "#df000356",
            "#d2000571",
            "#db0007b7",
            "#d10005c1",
            "#c40006d3",
            "#55000de8",
        ],
        dark: [
            "#191111ff",
            "#201314ff",
            "#3b1219ff",
            "#500f1cff",
            "#611623ff",
            "#72232dff",
            "#8c333aff",
            "#b54548ff",
            "#e5484dff",
            "#ec5d5eff",
            "#ff9592ff",
            "#ffd1d9ff",
        ],
        dark_alpha: [
            "#f4121209",
            "#f22f3e11",
            "#ff173f2d",
            "#fe0a3b44",
            "#ff204756",
            "#ff3e5668",
            "#ff536184",
            "#ff5d61b0",
            "#fe4e54e4",
            "#ff6465eb",
            "#ff9592ff",
            "#ffd1d9ff",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn ruby() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Ruby",
        light: [
            "#fffcfdff",
            "#fff7f8ff",
            "#feeaedff",
            "#ffdce1ff",
            "#ffced6ff",
            "#f8bfc8ff",
            "#efacb8ff",
            "#e592a3ff",
            "#e54666ff",
            "#dc3b5dff",
            "#ca244dff",
            "#64172bff",
        ],
        light_alpha: [
            "#ff005503",
            "#ff002008",
            "#f3002515",
            "#ff002523",
            "#ff002a31",
            "#e4002440",
            "#ce002553",
            "#c300286d",
            "#db002cb9",
            "#d2002cc4",
            "#c10030db",
            "#550016e8",
        ],
        dark: [
            "#191113ff",
            "#1e1517ff",
            "#3a141eff",
            "#4e1325ff",
            "#5e1a2eff",
            "#6f2539ff",
            "#883447ff",
            "#b3445aff",
            "#e54666ff",
            "#ec5a72ff",
            "#ff949dff",
            "#fed2e1ff",
        ],
        dark_alpha: [
            "#f4124a09",
            "#fe5a7f0e",
            "#ff235d2c",
            "#fd195e42",
            "#fe2d6b53",
            "#ff447665",
            "#ff577d80",
            "#ff5c7cae",
            "#fe4c70e4",
            "#ff617beb",
            "#ff949dff",
            "#ffd3e2fe",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn crimson() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Crimson",
        light: [
            "#fffcfdff",
            "#fef7f9ff",
            "#ffe9f0ff",
            "#fedce7ff",
            "#faceddff",
            "#f3bed1ff",
            "#eaacc3ff",
            "#e093b2ff",
            "#e93d82ff",
            "#df3478ff",
            "#cb1d63ff",
            "#621639ff",
        ],
        light_alpha: [
            "#ff005503",
            "#e0004008",
            "#ff005216",
            "#f8005123",
            "#e5004f31",
            "#d0004b41",
            "#bf004753",
            "#b6004a6c",
            "#e2005bc2",
            "#d70056cb",
            "#c4004fe2",
            "#530026e9",
        ],
        dark: [
            "#191114ff",
            "#201318ff",
            "#381525ff",
            "#4d122fff",
            "#5c1839ff",
            "#6d2545ff",
            "#873356ff",
            "#b0436eff",
            "#e93d82ff",
            "#ee518aff",
            "#ff92adff",
            "#fdd3e8ff",
        ],
        dark_alpha: [
            "#f4126709",
            "#f22f7a11",
            "#fe2a8b2a",
            "#fd158741",
            "#fd278f51",
            "#fe459763",
            "#fd559b7f",
            "#fe5b9bab",
            "#fe418de8",
            "#ff5693ed",
            "#ff92adff",
            "#ffd5eafd",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn pink() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Pink",
        light: [
            "#fffcfeff",
            "#fef7fbff",
            "#fee9f5ff",
            "#fbdcefff",
            "#f6cee7ff",
            "#efbfddff",
            "#e7acd0ff",
            "#dd93c2ff",
            "#d6409fff",
            "#cf3897ff",
            "#c2298aff",
            "#651249ff",
        ],
        light_alpha: [
            "#ff00aa03",
            "#e0008008",
            "#f4008c16",
            "#e2008b23",
            "#d1008331",
            "#c0007840",
            "#b6006f53",
            "#af006f6c",
            "#c8007fbf",
            "#c2007ac7",
            "#b60074d6",
            "#59003bed",
        ],
        dark: [
            "#191117ff",
            "#21121dff",
            "#37172fff",
            "#4b143dff",
            "#591c47ff",
            "#692955ff",
            "#833869ff",
            "#a84885ff",
            "#d6409fff",
            "#de51a8ff",
            "#ff8dccff",
            "#fdd1eaff",
        ],
        dark_alpha: [
            "#f412bc09",
            "#f420bb12",
            "#fe37cc29",
            "#fc1ec43f",
            "#fd35c24e",
            "#fd51c75f",
            "#fd62c87b",
            "#ff68c8a2",
            "#fe49bcd4",
            "#ff5cc0dc",
            "#ff8dccff",
            "#ffd3ecfd",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn plum() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Plum",
        light: [
            "#fefcffff",
            "#fdf7fdff",
            "#fbebfbff",
            "#f7def8ff",
            "#f2d1f3ff",
            "#e9c2ecff",
            "#deade3ff",
            "#cf91d8ff",
            "#ab4abaff",
            "#a144afff",
            "#953ea3ff",
            "#53195dff",
        ],
        light_alpha: [
            "#aa00ff03",
            "#c000c008",
            "#cc00cc14",
            "#c200c921",
            "#b700bd2e",
            "#a400b03d",
            "#9900a852",
            "#9000a56e",
            "#89009eb5",
            "#7f0092bb",
            "#730086c1",
            "#40004be6",
        ],
        dark: [
            "#181118ff",
            "#201320ff",
            "#351a35ff",
            "#451d47ff",
            "#512454ff",
            "#5e3061ff",
            "#734079ff",
            "#92549cff",
            "#ab4abaff",
            "#b658c4ff",
            "#e796f3ff",
            "#f4d4f4ff",
        ],
        dark_alpha: [
            "#f112f108",
            "#f22ff211",
            "#fd4cfd27",
            "#f646ff3a",
            "#f455ff48",
            "#f66dff56",
            "#f07cfd70",
            "#ee84ff95",
            "#e961feb6",
            "#ed70ffc0",
            "#f19cfef3",
            "#feddfef4",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn purple() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Purple",
        light: [
            "#fefcfeff",
            "#fbf7feff",
            "#f7edfeff",
            "#f2e2fcff",
            "#ead5f9ff",
            "#e0c4f4ff",
            "#d1afecff",
            "#be93e4ff",
            "#8e4ec6ff",
            "#8347b9ff",
            "#8145b5ff",
            "#402060ff",
        ],
        light_alpha: [
            "#aa00aa03",
            "#8000e008",
            "#8e00f112",
            "#8d00e51d",
            "#8000db2a",
            "#7a01d03b",
            "#6d00c350",
            "#6600c06c",
            "#5c00adb1",
            "#53009eb8",
            "#52009aba",
            "#250049df",
        ],
        dark: [
            "#18111bff",
            "#1e1523ff",
            "#301c3bff",
            "#3d224eff",
            "#48295cff",
            "#54346bff",
            "#664282ff",
            "#8457aaff",
            "#8e4ec6ff",
            "#9a5cd0ff",
            "#d19dffff",
            "#ecd9faff",
        ],
        dark_alpha: [
            "#b412f90b",
            "#b744f714",
            "#c150ff2d",
            "#bb53fd42",
            "#be5cfd51",
            "#c16dfd61",
            "#c378fd7a",
            "#c47effa4",
            "#b661ffc2",
            "#bc6fffcd",
            "#d19dffff",
            "#f1ddfffa",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn violet() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Violet",
        light: [
            "#fdfcfeff",
            "#faf8ffff",
            "#f4f0feff",
            "#ebe4ffff",
            "#e1d9ffff",
            "#d4cafeff",
            "#c2b5f5ff",
            "#aa99ecff",
            "#6e56cfff",
            "#654dc4ff",
            "#6550b9ff",
            "#2f265fff",
        ],
        light_alpha: [
            "#5500aa03",
            "#4900ff07",
            "#4400ee0f",
            "#4300ff1b",
            "#3600ff26",
            "#3100fb35",
            "#2d01dd4a",
            "#2b00d066",
            "#2400b7a9",
            "#2300abb2",
            "#1f0099af",
            "#0b0043d9",
        ],
        dark: [
            "#14121fff",
            "#1b1525ff",
            "#291f43ff",
            "#33255bff",
            "#3c2e69ff",
            "#473876ff",
            "#56468bff",
            "#6958adff",
            "#6e56cfff",
            "#7d66d9ff",
            "#baa7ffff",
            "#e2ddfeff",
        ],
        dark_alpha: [
            "#4422ff0f",
            "#853ff916",
            "#8354fe36",
            "#7d51fd50",
            "#845ffd5f",
            "#8f6cfd6d",
            "#9879ff83",
            "#977dfea8",
            "#8668ffcc",
            "#9176fed7",
            "#baa7ffff",
            "#e3defffe",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn iris() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Iris",
        light: [
            "#fdfdffff",
            "#f8f8ffff",
            "#f0f1feff",
            "#e6e7ffff",
            "#dadcffff",
            "#cbcdffff",
            "#b8baf8ff",
            "#9b9ef0ff",
            "#5b5bd6ff",
            "#5151cdff",
            "#5753c6ff",
            "#272962ff",
        ],
        light_alpha: [
            "#0000ff02",
            "#0000ff07",
            "#0011ee0f",
            "#000bff19",
            "#000eff25",
            "#000aff34",
            "#0008e647",
            "#0008d964",
            "#0000c0a4",
            "#0000b6ae",
            "#0600abac",
            "#000246d8",
        ],
        dark: [
            "#13131eff",
            "#171625ff",
            "#202248ff",
            "#262a65ff",
            "#303374ff",
            "#3d3e82ff",
            "#4a4a95ff",
            "#5958b1ff",
            "#5b5bd6ff",
            "#6e6adeff",
            "#b1a9ffff",
            "#e0dffeff",
        ],
        dark_alpha: [
            "#3636fe0e",
            "#564bf916",
            "#525bff3b",
            "#4d58ff5a",
            "#5b62fd6b",
            "#6d6ffd7a",
            "#7777fe8e",
            "#7b7afeac",
            "#6a6afed4",
            "#7d79ffdc",
            "#b1a9ffff",
            "#e1e0fffe",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn indigo() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Indigo",
        light: [
            "#fdfdfeff",
            "#f7f9ffff",
            "#edf2feff",
            "#e1e9ffff",
            "#d2deffff",
            "#c1d0ffff",
            "#abbdf9ff",
            "#8da4efff",
            "#3e63ddff",
            "#3358d4ff",
            "#3a5bc7ff",
            "#1f2d5cff",
        ],
        light_alpha: [
            "#00008002",
            "#0040ff08",
            "#0047f112",
            "#0044ff1e",
            "#0044ff2d",
            "#003eff3e",
            "#0037ed54",
            "#0034dc72",
            "#0031d2c1",
            "#002ec9cc",
            "#002bb7c5",
            "#001046e0",
        ],
        dark: [
            "#11131fff",
            "#141726ff",
            "#182449ff",
            "#1d2e62ff",
            "#253974ff",
            "#304384ff",
            "#3a4f97ff",
            "#435db1ff",
            "#3e63ddff",
            "#5472e4ff",
            "#9eb1ffff",
            "#d6e1ffff",
        ],
        dark_alpha: [
            "#1133ff0f",
            "#3354fa17",
            "#2f62ff3c",
            "#3566ff57",
            "#4171fd6b",
            "#5178fd7c",
            "#5a7fff90",
            "#5b81feac",
            "#4671ffdb",
            "#5c7efee3",
            "#9eb1ffff",
            "#d6e1ffff",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn blue() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Blue",
        light: [
            "#fbfdffff",
            "#f4faffff",
            "#e6f4feff",
            "#d5efffff",
            "#c2e5ffff",
            "#acd8fcff",
            "#8ec8f6ff",
            "#5eb1efff",
            "#0090ffff",
            "#0588f0ff",
            "#0d74ceff",
            "#113264ff",
        ],
        light_alpha: [
            "#0080ff04",
            "#008cff0b",
            "#008ff519",
            "#009eff2a",
            "#0093ff3d",
            "#0088f653",
            "#0083eb71",
            "#0084e6a1",
            "#0090ffff",
            "#0086f0fa",
            "#006dcbf2",
            "#002359ee",
        ],
        dark: [
            "#0d1520ff",
            "#111927ff",
            "#0d2847ff",
            "#003362ff",
            "#004074ff",
            "#104d87ff",
            "#205d9eff",
            "#2870bdff",
            "#0090ffff",
            "#3b9effff",
            "#70b8ffff",
            "#c2e6ffff",
        ],
        dark_alpha: [
            "#004df211",
            "#1166fb18",
            "#0077ff3a",
            "#0075ff57",
            "#0081fd6b",
            "#0f89fd7f",
            "#2a91fe98",
            "#3094feb9",
            "#0090ffff",
            "#3b9effff",
            "#70b8ffff",
            "#c2e6ffff",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn cyan() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Cyan",
        light: [
            "#fafdfeff",
            "#f2fafbff",
            "#def7f9ff",
            "#caf1f6ff",
            "#b5e9f0ff",
            "#9ddde7ff",
            "#7dcedcff",
            "#3db9cfff",
            "#00a2c7ff",
            "#0797b9ff",
            "#107d98ff",
            "#0d3c48ff",
        ],
        light_alpha: [
            "#0099cc05",
            "#009db10d",
            "#00c2d121",
            "#00bcd435",
            "#01b4cc4a",
            "#00a7c162",
            "#009fbb82",
            "#00a3c0c2",
            "#00a2c7ff",
            "#0094b7f8",
            "#007491ef",
            "#00323ef2",
        ],
        dark: [
            "#0b161aff",
            "#101b20ff",
            "#082c36ff",
            "#003848ff",
            "#004558ff",
            "#045468ff",
            "#12677eff",
            "#11809cff",
            "#00a2c7ff",
            "#23afd0ff",
            "#4ccce6ff",
            "#b6ecf7ff",
        ],
        dark_alpha: [
            "#0091f70a",
            "#02a7f211",
            "#00befd28",
            "#00baff3b",
            "#00befd4d",
            "#00c7fd5e",
            "#14cdff75",
            "#11cfff95",
            "#00cfffc3",
            "#28d6ffcd",
            "#52e1fee5",
            "#bbf3fef7",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn teal() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Teal",
        light: [
            "#fafefdff",
            "#f3fbf9ff",
            "#e0f8f3ff",
            "#ccf3eaff",
            "#b8eae0ff",
            "#a1ded2ff",
            "#83cdc1ff",
            "#53b9abff",
            "#12a594ff",
            "#0d9b8aff",
            "#008573ff",
            "#0d3d38ff",
        ],
        light_alpha: [
            "#00cc9905",
            "#00aa800c",
            "#00c69d1f",
            "#00c39633",
            "#00b49047",
            "#00a6855e",
            "#0099807c",
            "#009783ac",
            "#009e8ced",
            "#009684f2",
            "#008573ff",
            "#00332df2",
        ],
        dark: [
            "#0d1514ff",
            "#111c1bff",
            "#0d2d2aff",
            "#023b37ff",
            "#084843ff",
            "#145750ff",
            "#1c6961ff",
            "#207e73ff",
            "#12a594ff",
            "#0eb39eff",
            "#0bd8b6ff",
            "#adf0ddff",
        ],
        dark_alpha: [
            "#00deab05",
            "#12fbe60c",
            "#00ffe61e",
            "#00ffe92d",
            "#00ffea3b",
            "#1cffe84b",
            "#2efde85f",
            "#32ffe775",
            "#13ffe49f",
            "#0dffe0ae",
            "#0afed5d6",
            "#b8ffebef",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn jade() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Jade",
        light: [
            "#fbfefdff",
            "#f4fbf7ff",
            "#e6f7edff",
            "#d6f1e3ff",
            "#c3e9d7ff",
            "#acdec8ff",
            "#8bceb6ff",
            "#56ba9fff",
            "#29a383ff",
            "#26997bff",
            "#208368ff",
            "#1d3b31ff",
        ],
        light_alpha: [
            "#00c08004",
            "#00a3460b",
            "#00ae4819",
            "#00a85129",
            "#00a2553c",
            "#009a5753",
            "#00945f74",
            "#00976ea9",
            "#00916bd6",
            "#008764d9",
            "#007152df",
            "#002217e2",
        ],
        dark: [
            "#0d1512ff",
            "#121c18ff",
            "#0f2e22ff",
            "#0b3b2cff",
            "#114837ff",
            "#1b5745ff",
            "#246854ff",
            "#2a7e68ff",
            "#29a383ff",
            "#27b08bff",
            "#1fd8a4ff",
            "#adf0d4ff",
        ],
        dark_alpha: [
            "#00de4505",
            "#27fba60c",
            "#02f99920",
            "#00ffaa2d",
            "#11ffb63b",
            "#34ffc24b",
            "#45fdc75e",
            "#48ffcf75",
            "#38feca9d",
            "#31fec7ab",
            "#21fec0d6",
            "#b8ffe1ef",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn green() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Green",
        light: [
            "#fbfefcff",
            "#f4fbf6ff",
            "#e6f6ebff",
            "#d6f1dfff",
            "#c4e8d1ff",
            "#adddc0ff",
            "#8eceaaff",
            "#5bb98bff",
            "#30a46cff",
            "#2b9a66ff",
            "#218358ff",
            "#193b2dff",
        ],
        light_alpha: [
            "#00c04004",
            "#00a32f0b",
            "#00a43319",
            "#00a83829",
            "#019c393b",
            "#00963c52",
            "#00914071",
            "#00924ba4",
            "#008f4acf",
            "#008647d4",
            "#00713fde",
            "#002616e6",
        ],
        dark: [
            "#0e1512ff",
            "#121b17ff",
            "#132d21ff",
            "#113b29ff",
            "#174933ff",
            "#20573eff",
            "#28684aff",
            "#2f7c57ff",
            "#30a46cff",
            "#33b074ff",
            "#3dd68cff",
            "#b1f1cbff",
        ],
        dark_alpha: [
            "#00de4505",
            "#29f99d0b",
            "#22ff991e",
            "#11ff992d",
            "#2bffa23c",
            "#44ffaa4b",
            "#50fdac5e",
            "#54ffad73",
            "#44ffa49e",
            "#43fea4ab",
            "#46fea5d4",
            "#bbffd7f0",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn grass() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Grass",
        light: [
            "#fbfefbff",
            "#f5fbf5ff",
            "#e9f6e9ff",
            "#daf1dbff",
            "#c9e8caff",
            "#b2ddb5ff",
            "#94ce9aff",
            "#65ba74ff",
            "#46a758ff",
            "#3e9b4fff",
            "#2a7e3bff",
            "#203c25ff",
        ],
        light_alpha: [
            "#00c00004",
            "#0099000a",
            "#00970016",
            "#009f0725",
            "#00930536",
            "#008f0a4d",
            "#018b0f6b",
            "#008d199a",
            "#008619b9",
            "#007b17c1",
            "#006514d5",
            "#002006df",
        ],
        dark: [
            "#0e1511ff",
            "#141a15ff",
            "#1b2a1eff",
            "#1d3a24ff",
            "#25482dff",
            "#2d5736ff",
            "#366740ff",
            "#3e7949ff",
            "#46a758ff",
            "#53b365ff",
            "#71d083ff",
            "#c2f0c2ff",
        ],
        dark_alpha: [
            "#00de1205",
            "#5ef7780a",
            "#70fe8c1b",
            "#57ff802c",
            "#68ff8b3b",
            "#71ff8f4b",
            "#77fd925d",
            "#77fd9070",
            "#65ff82a1",
            "#72ff8dae",
            "#89ff9fcd",
            "#ceffceef",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn lime() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Lime",
        light: [
            "#fcfdfaff",
            "#f8faf3ff",
            "#eef6d6ff",
            "#e2f0bdff",
            "#d3e7a6ff",
            "#c2da91ff",
            "#abc978ff",
            "#8db654ff",
            "#bdee63ff",
            "#b0e64cff",
            "#5c7c2fff",
            "#37401cff",
        ],
        light_alpha: [
            "#66990005",
            "#6b95000c",
            "#96c80029",
            "#8fc60042",
            "#81bb0059",
            "#72aa006e",
            "#61990087",
            "#559200ab",
            "#93e4009c",
            "#8fdc00b3",
            "#375f00d0",
            "#1e2900e3",
        ],
        dark: [
            "#11130cff",
            "#151a10ff",
            "#1f2917ff",
            "#29371dff",
            "#334423ff",
            "#3d522aff",
            "#496231ff",
            "#577538ff",
            "#bdee63ff",
            "#d4ff70ff",
            "#bde56cff",
            "#e3f7baff",
        ],
        dark_alpha: [
            "#11bb0003",
            "#78f7000a",
            "#9bfd4c1a",
            "#a7fe5c29",
            "#affe6537",
            "#b2fe6d46",
            "#b6ff6f57",
            "#b6fd6d6c",
            "#caff69ed",
            "#d4ff70ff",
            "#d1fe77e4",
            "#e9febff7",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn mint() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Mint",
        light: [
            "#f9fefdff",
            "#f2fbf9ff",
            "#ddf9f2ff",
            "#c8f4e9ff",
            "#b3ecdeff",
            "#9ce0d0ff",
            "#7ecfbdff",
            "#4cbba5ff",
            "#86ead4ff",
            "#7de0cbff",
            "#027864ff",
            "#16433cff",
        ],
        light_alpha: [
            "#00d5aa06",
            "#00b18a0d",
            "#00d29e22",
            "#00cc9937",
            "#00c0914c",
            "#00b08663",
            "#00a17d81",
            "#009e7fb3",
            "#00d3a579",
            "#00c39982",
            "#007763fd",
            "#00312ae9",
        ],
        dark: [
            "#0e1515ff",
            "#0f1b1bff",
            "#092c2bff",
            "#003a38ff",
            "#004744ff",
            "#105650ff",
            "#1e685fff",
            "#277f70ff",
            "#86ead4ff",
            "#a8f5e5ff",
            "#58d5baff",
            "#c4f5e1ff",
        ],
        dark_alpha: [
            "#00dede05",
            "#00f9f90b",
            "#00fff61d",
            "#00fff42c",
            "#00fff23a",
            "#0effeb4a",
            "#34fde55e",
            "#41ffdf76",
            "#92ffe7e9",
            "#aefeedf5",
            "#67ffded2",
            "#cbfee9f5",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn sky() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Sky",
        light: [
            "#f9feffff",
            "#f1fafdff",
            "#e1f6fdff",
            "#d1f0faff",
            "#bee7f5ff",
            "#a9daedff",
            "#8dcae3ff",
            "#60b3d7ff",
            "#7ce2feff",
            "#74daf8ff",
            "#00749eff",
            "#1d3e56ff",
        ],
        light_alpha: [
            "#00d5ff06",
            "#00a4db0e",
            "#00b3ee1e",
            "#00ace42e",
            "#00a1d841",
            "#0092ca56",
            "#0089c172",
            "#0085bf9f",
            "#00c7fe83",
            "#00bcf38b",
            "#00749eff",
            "#002540e2",
        ],
        dark: [
            "#0d141fff",
            "#111a27ff",
            "#112840ff",
            "#113555ff",
            "#154467ff",
            "#1b537bff",
            "#1f6692ff",
            "#197caeff",
            "#7ce2feff",
            "#a8eeffff",
            "#75c7f0ff",
            "#c2f3ffff",
        ],
        dark_alpha: [
            "#0044ff0f",
            "#1171fb18",
            "#1184fc33",
            "#128fff49",
            "#1c9dfd5d",
            "#28a5ff72",
            "#2badfe8b",
            "#1db2fea9",
            "#7ce3fffe",
            "#a8eeffff",
            "#7cd3ffef",
            "#c2f3ffff",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn black() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "Black",
        light: [
            "#0000000d",
            "#0000001a",
            "#00000026",
            "#00000033",
            "#0000004d",
            "#00000066",
            "#00000080",
            "#00000099",
            "#000000b3",
            "#000000cc",
            "#000000e6",
            "#000000f2",
        ],
        light_alpha: [
            "#0000000d",
            "#0000001a",
            "#00000026",
            "#00000033",
            "#0000004d",
            "#00000066",
            "#00000080",
            "#00000099",
            "#000000b3",
            "#000000cc",
            "#000000e6",
            "#000000f2",
        ],
        dark: [
            "#0000000d",
            "#0000001a",
            "#00000026",
            "#00000033",
            "#0000004d",
            "#00000066",
            "#00000080",
            "#00000099",
            "#000000b3",
            "#000000cc",
            "#000000e6",
            "#000000f2",
        ],
        dark_alpha: [
            "#0000000d",
            "#0000001a",
            "#00000026",
            "#00000033",
            "#0000004d",
            "#00000066",
            "#00000080",
            "#00000099",
            "#000000b3",
            "#000000cc",
            "#000000e6",
            "#000000f2",
        ],
    }
    .try_into()
    .unwrap()
}

pub(crate) fn white() -> ColorScaleSet {
    StaticColorScaleSet {
        scale: "White",
        light: [
            "#ffffff0d",
            "#ffffff1a",
            "#ffffff26",
            "#ffffff33",
            "#ffffff4d",
            "#ffffff66",
            "#ffffff80",
            "#ffffff99",
            "#ffffffb3",
            "#ffffffcc",
            "#ffffffe6",
            "#fffffff2",
        ],
        light_alpha: [
            "#ffffff0d",
            "#ffffff1a",
            "#ffffff26",
            "#ffffff33",
            "#ffffff4d",
            "#ffffff66",
            "#ffffff80",
            "#ffffff99",
            "#ffffffb3",
            "#ffffffcc",
            "#ffffffe6",
            "#fffffff2",
        ],
        dark: [
            "#ffffff0d",
            "#ffffff1a",
            "#ffffff26",
            "#ffffff33",
            "#ffffff4d",
            "#ffffff66",
            "#ffffff80",
            "#ffffff99",
            "#ffffffb3",
            "#ffffffcc",
            "#ffffffe6",
            "#fffffff2",
        ],
        dark_alpha: [
            "#ffffff0d",
            "#ffffff1a",
            "#ffffff26",
            "#ffffff33",
            "#ffffff4d",
            "#ffffff66",
            "#ffffff80",
            "#ffffff99",
            "#ffffffb3",
            "#ffffffcc",
            "#ffffffe6",
            "#fffffff2",
        ],
    }
    .try_into()
    .unwrap()
}

// ---------------------------------------------------------------------------
// 内置 Catppuccin 主题的配色
// ---------------------------------------------------------------------------
//
// 上面 `ThemeColors::{light,dark}` 是 zed 的默认灰阶配色；这一节是**本仓库
// 内置主题实际使用的配色**（Catppuccin Mocha / Latte）。
//
// 为什么放在这里而不是单独的 `builtin.rs`：与 zed 的文件划分一致——
// zed 把「内置配色」放在 `default_colors.rs`（色板 + `ThemeColors::{light,dark}`），
// 没有单独的 builtin 文件。我们的内置主题换了配色（Catppuccin 而非灰阶），
// 但角色相同，故同样落在本文件。
//
// 语法色的 capture 名对齐 tree-sitter 的常见捕获（`keyword` / `string` /
// `comment` / `function`…），将来接高亮查询时直接复用。

// ── Catppuccin Macchiato（深色内置主题，= 基底 ThemeColors::dark()）─────────
// 色值来源: https://github.com/catppuccin/palette (macchiato)

// ── Catppuccin Macchiato ──────────────────────────────────────────────────────
// Macchiato = Mocha 的次深 variant (base #24273A vs Mocha #1E1E2E)。
// 色值来源: https://github.com/catppuccin/palette (macchiato)
/// `0xRRGGBB` → `Hsla`。
pub(crate) fn h(hex: u32) -> Hsla {
    gpui::rgb(hex).into()
}

/// 带透明度的 `Hsla`。
pub(crate) fn ha(hex: u32, a: f32) -> Hsla {
    let mut c: Hsla = gpui::rgb(hex).into();
    c.a = a;
    c
}

/// 单状态三色组：(前景, 15% 透明底, 边框)。
///
/// zed 的 `StatusColors` 是扁平结构（`error` / `error_background` /
/// `error_border` 三个独立字段），所以这里返回三元组。
fn status(hex: u32) -> (Hsla, Hsla, Hsla) {
    (h(hex), ha(hex, 0.15), h(hex))
}
pub(crate) fn syntax_macchiato() -> SyntaxTheme {
    SyntaxTheme::new([
        (
            "comment".into(),
            HighlightStyle {
                color: Some(h(0x6e738d)),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        ),
        (
            "keyword".into(),
            HighlightStyle {
                color: Some(h(0xc6a0f6)),
                ..Default::default()
            },
        ),
        (
            "string".into(),
            HighlightStyle {
                color: Some(h(0xa6da95)),
                ..Default::default()
            },
        ),
        (
            "number".into(),
            HighlightStyle {
                color: Some(h(0xf5deb3)),
                ..Default::default()
            },
        ),
        (
            "function".into(),
            HighlightStyle {
                color: Some(h(0x8aadf4)),
                ..Default::default()
            },
        ),
        (
            "type".into(),
            HighlightStyle {
                color: Some(h(0x91d7e3)),
                ..Default::default()
            },
        ),
        (
            "constant".into(),
            HighlightStyle {
                color: Some(h(0xf5a97f)),
                ..Default::default()
            },
        ),
        (
            "variable".into(),
            HighlightStyle {
                color: Some(h(0xcad3f5)),
                ..Default::default()
            },
        ),
        (
            "operator".into(),
            HighlightStyle {
                color: Some(h(0x8bd5ca)),
                ..Default::default()
            },
        ),
        (
            "punctuation".into(),
            HighlightStyle {
                color: Some(h(0x9198b0)),
                ..Default::default()
            },
        ),
        (
            "tag".into(),
            HighlightStyle {
                color: Some(h(0xed8796)),
                ..Default::default()
            },
        ),
        (
            "attribute".into(),
            HighlightStyle {
                color: Some(h(0xf5a97f)),
                ..Default::default()
            },
        ),
        (
            "title".into(),
            HighlightStyle {
                color: Some(h(0x8aadf4)),
                font_weight: Some(FontWeight::MEDIUM),
                ..Default::default()
            },
        ),
        (
            "link".into(),
            HighlightStyle {
                color: Some(h(0x8aadf4)),
                ..Default::default()
            },
        ),
    ])
}

pub fn status_colors_macchiato() -> StatusColors {
    let mut colors = StatusColors::dark();
    let (fg, bg, border) = status(0xed8796); // red
    colors.conflict = fg;
    colors.conflict_background = bg;
    colors.conflict_border = border;
    let (fg, bg, border) = status(0xa6da95); // green
    colors.created = fg;
    colors.created_background = bg;
    colors.created_border = border;
    let (fg, bg, border) = status(0xed8796);
    colors.deleted = fg;
    colors.deleted_background = bg;
    colors.deleted_border = border;
    let (fg, bg, border) = status(0xed8796);
    colors.error = fg;
    colors.error_background = bg;
    colors.error_border = border;
    let (fg, bg, border) = status(0x6e738d); // overlay0 (muted)
    colors.hidden = fg;
    colors.hidden_background = bg;
    colors.hidden_border = border;
    let (fg, bg, border) = status(0x6e738d);
    colors.ignored = fg;
    colors.ignored_background = bg;
    colors.ignored_border = border;
    let (fg, bg, border) = status(0x91d7e3); // sky
    colors.info = fg;
    colors.info_background = bg;
    colors.info_border = border;
    let (fg, bg, border) = status(0xf5a97f); // peach
    colors.modified = fg;
    colors.modified_background = bg;
    colors.modified_border = border;
    let (fg, bg, border) = status(0x8aadf4); // blue
    colors.renamed = fg;
    colors.renamed_background = bg;
    colors.renamed_border = border;
    let (fg, bg, border) = status(0xa6da95); // green
    colors.success = fg;
    colors.success_background = bg;
    colors.success_border = border;
    let (fg, bg, border) = status(0xf5deb3); // yellow
    colors.warning = fg;
    colors.warning_background = bg;
    colors.warning_border = border;
    // — Zed JSON 里 macchiato 额外给了 hint / predictive / unreachable（JSON 给了完整三色，不派生）—
    colors.hint = h(0x5b6078);
    colors.hint_background = h(0x1e2030);
    colors.hint_border = h(0x5b6078);
    colors.predictive = h(0x6e738d);
    colors.predictive_background = h(0x1e2030);
    colors.predictive_border = h(0xb7bdf8);
    colors.unreachable = h(0xed8796);
    colors.unreachable_background = ha(0xed8796, 0.12);
    colors.unreachable_border = h(0xed8796);
    colors
}

/// Catppuccin Macchiato 的 `ThemeColors`（= 深色基底 [`ThemeColors::dark`]）。
///
/// **结构**：`ThemeColors::zed_dark()` 起步，再覆盖 CTP 调色板有的字段。
/// 因此本函数只写 CTP 定义了的角色；**CTP 没定义的角色自动留在 zed 默认
/// 配色上**——这是有意的分层，不是漏配：
///
/// - CTP 是固定调色板（26 个语义色），没有 zed 那种 12 级色阶；
/// - zed 对 vim 模式、diff hunk、code lens、jump label 等角色都有默认值，
///   CTP JSON 则不提供，故这些字段**来自 zed 默认配色**（`ThemeColors::zed_dark()`）。
///
/// 未覆盖字段共 35 个，与 `theme_colors_latte` 的 33 个大致同批
/// （vim_* 全套、editor_diff_hunk_*、editor_code_lens_foreground、
/// editor_highlighted_line_background、editor_hover_line_number、
/// element_selection_background、version_control_word_*、
/// vim_helix_jump_label_foreground、drop_target_border、
/// minimap_thumb_border、panel_overlay_hover），外加
/// scrollbar_thumb_active_background / scrollbar_thumb_border。
///
/// 若日后 CTP 或自定义主题要覆盖这些角色，直接在本函数体内补 `colors.x = …`。
pub fn theme_colors_macchiato() -> ThemeColors {
    let system = crate::styles::SystemColors::default();

    ThemeColors {
        border: h(0x363a4f),          // surface0
        border_variant: h(0x313148),  // surface0-darker
        border_focused: h(0xb7bdf8),  // lavender
        border_selected: h(0xc6a0f6), // mauve
        border_disabled: h(0x6e738d), // overlay0
        border_transparent: hsla(0., 0., 0., 0.),
        background: h(0x2c2f46),         // base — Zed JSON "background"
        surface_background: h(0x1e2030), // mantle
        elevated_surface_background: h(0x1e2030), // mantle (同 surface)
        element_background: h(0x181926), // crust
        element_hover: ha(0x363a4f, 1.0), // surface0
        element_active: ha(0x5b6078, 0.30), // surface2 @30%
        element_selected: ha(0x363a4f, 0.30), // surface0 @30%
        element_disabled: h(0x6e738d),
        ghost_element_background: hsla(0., 0., 0., 0.),
        ghost_element_hover: ha(0x494d64, 0.30), // surface1 @30%
        ghost_element_active: ha(0x5b6078, 0.60), // surface2 @60%
        ghost_element_selected: ha(0x737995, 0.40), // overlay2 @40%
        ghost_element_disabled: h(0x6e738d),
        text: h(0xcad3f5),             // text
        text_muted: h(0xb8c0e0),       // subtext0
        text_placeholder: h(0x5b6078), // surface2
        text_disabled: h(0x6e738d),    // overlay0
        text_accent: h(0xc6a0f6),      // mauve
        icon: h(0xcad3f5),
        icon_muted: h(0x8087a2), // overlay2
        icon_disabled: h(0x6e738d),
        icon_placeholder: h(0x5b6078),
        icon_accent: h(0xc6a0f6),
        // — Zed JSON 显式设置的状态/标题栏/Tab/toolbar（之前缺失！）——
        status_bar_background: h(0x181926),          // crust
        title_bar_background: h(0x181926),           // crust
        title_bar_inactive_background: h(0x1e1f30),  // mantle-darker
        toolbar_background: h(0x24273a),             // base (CTP base #24273a)
        tab_bar_background: h(0x181926),             // crust
        tab_inactive_background: h(0x12121c),        // base-darker
        tab_active_background: h(0x24273a),          // base
        search_match_background: ha(0x8bd5ca, 0.30), // teal @30%
        search_active_match_background: ha(0xed8796, 0.30), // red @30%
        panel_background: h(0x1e2030),               // mantle
        panel_focused_border: h(0xcad3f5),           // text
        panel_indent_guide: ha(0x363a4f, 0.60),      // surface0 @60%
        panel_indent_guide_active: h(0x5b6078),      // surface2
        panel_indent_guide_hover: h(0xc6a0f6),       // mauve
        panel_overlay_background: h(0x1e2030),       // mantle
        pane_focused_border: h(0xcad3f5),            // text
        pane_group_border: h(0x363a4f),              // surface0
        scrollbar_thumb_background: ha(0x5b6078, 0.50), // surface2 @50%
        scrollbar_thumb_hover_background: h(0x6e738d), // overlay0
        scrollbar_track_background: h(0x181926),     // crust
        scrollbar_track_border: ha(0xcad3f5, 0.07),  // text @7%
        editor_foreground: h(0xcad3f5),
        editor_background: h(0x24273a), // base
        editor_gutter_background: h(0x24273a),
        editor_subheader_background: h(0x1e2030), // mantle
        editor_active_line_background: ha(0xcad3f5, 0.07), // text @7%
        editor_line_number: h(0x8087a2),          // overlay2
        editor_active_line_number: h(0xc6a0f6),   // mauve
        editor_wrap_guide: h(0x5b6078),           // surface2
        editor_indent_guide: ha(0x363a4f, 0.60),  // surface0 @60%
        editor_indent_guide_active: h(0x5b6078),  // surface2
        editor_invisible: ha(0x939ab7, 0.40),     // overlay1 @40%
        editor_debugger_active_line_background: ha(0xf5a97f, 0.07), // peach @7%
        selection_background: ha(0x939ab7, 0.25), // overlay1 @25%
        editor_cursor: h(0xf4dbd6),               // rosewater
        editor_document_highlight_read_background: ha(0xa5adcb, 0.16), // subtext1 @16%
        editor_document_highlight_write_background: ha(0xa5adcb, 0.16), // subtext1 @16%
        editor_document_highlight_bracket_background: ha(0xc6a0f6, 0.09), // mauve @9%
        terminal_background: h(0x24273a),
        terminal_foreground: h(0xcad3f5),
        terminal_dim_foreground: h(0x8087a2),
        terminal_bright_foreground: h(0xcad3f5),
        terminal_ansi_background: h(0x24273a),
        terminal_ansi_black: h(0x494d64),
        terminal_ansi_white: h(0xa5adcb),
        terminal_ansi_red: h(0xed8796),
        terminal_ansi_green: h(0xa6da95),
        terminal_ansi_yellow: h(0xeed49f),
        terminal_ansi_blue: h(0x8aadf4),
        terminal_ansi_magenta: h(0xf5bde6),
        terminal_ansi_cyan: h(0x8bd5ca),
        link: h(0x91d7e3), // teal-light
        // — Zed JSON 的 debugger / version_control / minimap / terminal ansi（之前缺失！）—
        debugger_accent: h(0xed8796),
        version_control_added: h(0xa6da95),
        version_control_deleted: h(0xed8796),
        version_control_modified: h(0xeed49f),
        version_control_renamed: h(0x7dc4e4),
        version_control_conflict: h(0xf5a97f),
        version_control_ignored: h(0x6e738d),
        version_control_conflict_marker_ours: ha(0xa6da95, 0.20),
        version_control_conflict_marker_theirs: ha(0x8aadf4, 0.20),
        drop_target_background: ha(0x363a4f, 0.40),
        editor_active_wrap_guide: h(0x5b6078),
        link_text_hover: h(0x91d7e3),
        minimap_thumb_background: ha(0xc6a0f6, 0.20),
        minimap_thumb_hover_background: ha(0xc6a0f6, 0.40),
        minimap_thumb_active_background: ha(0xc6a0f6, 0.60),
        terminal_ansi_bright_black: h(0x5b6078),
        terminal_ansi_bright_white: h(0xb8c0e0),
        terminal_ansi_bright_red: h(0xec7486),
        terminal_ansi_bright_green: h(0x8ccf7f),
        terminal_ansi_bright_yellow: h(0xe1c682),
        terminal_ansi_bright_blue: h(0x78a1f6),
        terminal_ansi_bright_magenta: h(0xf2a9dd),
        terminal_ansi_bright_cyan: h(0x63cbc0),
        terminal_ansi_dim_black: h(0x494d64),
        terminal_ansi_dim_white: h(0xa5adcb),
        terminal_ansi_dim_red: h(0xed8796),
        terminal_ansi_dim_green: h(0xa6da95),
        terminal_ansi_dim_yellow: h(0xeed49f),
        terminal_ansi_dim_blue: h(0x8aadf4),
        terminal_ansi_dim_magenta: h(0xf5bde6),
        terminal_ansi_dim_cyan: h(0x8bd5ca),
        // ── 以下字段 CTP 调色板未定义，取自 zed 默认配色 ──
        // （ThemeColors::zed_dark() 的同名值，显式写出便于审阅/微调）
        drop_target_border: neutral().dark().step_12(), // zed 默认（CTP 调色板未定义）
        editor_code_lens_foreground: None,
        editor_diff_hunk_added_background: ADDED_COLOR.opacity(0.12), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_added_hollow_background: ADDED_COLOR.opacity(0.06), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_added_hollow_border: ADDED_COLOR.opacity(0.36), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_deleted_background: REMOVED_COLOR.opacity(0.12), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_deleted_hollow_background: REMOVED_COLOR.opacity(0.06), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_deleted_hollow_border: REMOVED_COLOR.opacity(0.36), // zed 默认（CTP 调色板未定义）
        editor_highlighted_line_background: yellow().dark_alpha().step_4(), // zed 默认（CTP 调色板未定义）
        editor_hover_line_number: neutral().dark_alpha().step_12(), // zed 默认（CTP 调色板未定义）
        element_selection_background: blue().dark().step_3().alpha(0.25), // zed 默认（CTP 调色板未定义）
        minimap_thumb_border: gpui::transparent_black(), // zed 默认（CTP 调色板未定义）
        panel_overlay_hover: neutral().dark().step_4(),  // zed 默认（CTP 调色板未定义）
        scrollbar_thumb_active_background: neutral().dark_alpha().step_5(), // zed 默认（CTP 调色板未定义）
        scrollbar_thumb_border: gpui::transparent_black(), // zed 默认（CTP 调色板未定义）
        version_control_word_added: WORD_ADDED_COLOR,      // zed 默认（CTP 调色板未定义）
        version_control_word_deleted: WORD_DELETED_COLOR,  // zed 默认（CTP 调色板未定义）
        vim_helix_jump_label_foreground: red().dark().step_9(), // zed 默认（CTP 调色板未定义）
        vim_helix_normal_background: system.transparent,   // zed 默认（CTP 调色板未定义）
        vim_helix_normal_foreground: system.transparent,   // zed 默认（CTP 调色板未定义）
        vim_helix_select_background: system.transparent,   // zed 默认（CTP 调色板未定义）
        vim_helix_select_foreground: system.transparent,   // zed 默认（CTP 调色板未定义）
        vim_insert_background: system.transparent,         // zed 默认（CTP 调色板未定义）
        vim_insert_foreground: system.transparent,         // zed 默认（CTP 调色板未定义）
        vim_normal_background: system.transparent,         // zed 默认（CTP 调色板未定义）
        vim_normal_foreground: system.transparent,         // zed 默认（CTP 调色板未定义）
        vim_replace_background: system.transparent,        // zed 默认（CTP 调色板未定义）
        vim_replace_foreground: system.transparent,        // zed 默认（CTP 调色板未定义）
        vim_visual_background: system.transparent,         // zed 默认（CTP 调色板未定义）
        vim_visual_block_background: system.transparent,   // zed 默认（CTP 调色板未定义）
        vim_visual_block_foreground: system.transparent,   // zed 默认（CTP 调色板未定义）
        vim_visual_foreground: system.transparent,         // zed 默认（CTP 调色板未定义）
        vim_visual_line_background: system.transparent,    // zed 默认（CTP 调色板未定义）
        vim_visual_line_foreground: system.transparent,    // zed 默认（CTP 调色板未定义）
        vim_yank_background: neutral().dark_alpha().step_4(), // zed 默认（CTP 调色板未定义）
    }
}

/// Catppuccin Macchiato 兜底主题(= 深色基底 [`ThemeColors::dark`])。
///
/// 直接复用 [`default_colors`](crate::default_colors) 的内置主题构造，

// ── Catppuccin Latte（浅色内置主题，= 基底 ThemeColors::light()）───────────
// 色值来源: https://github.com/catppuccin/palette (latte)

pub fn status_colors_latte() -> StatusColors {
    // 同 `status_colors_macchiato`（浅色版用 Latte 的 red）。

    let mut colors = StatusColors::light();
    let (fg, bg, border) = status(0xd20f39);
    colors.conflict = fg;
    colors.conflict_background = bg;
    colors.conflict_border = border;
    let (fg, bg, border) = status(0x40a02b);
    colors.created = fg;
    colors.created_background = bg;
    colors.created_border = border;
    let (fg, bg, border) = status(0xd20f39);
    colors.deleted = fg;
    colors.deleted_background = bg;
    colors.deleted_border = border;
    let (fg, bg, border) = status(0xd20f39);
    colors.error = fg;
    colors.error_background = bg;
    colors.error_border = border;
    let (fg, bg, border) = status(0x9ca0b0);
    colors.hidden = fg;
    colors.hidden_background = bg;
    colors.hidden_border = border;
    let (fg, bg, border) = status(0x9ca0b0);
    colors.ignored = fg;
    colors.ignored_background = bg;
    colors.ignored_border = border;
    let (fg, bg, border) = status(0x179299);
    colors.info = fg;
    colors.info_background = bg;
    colors.info_border = border;
    let (fg, bg, border) = status(0xdf8e1d);
    colors.modified = fg;
    colors.modified_background = bg;
    colors.modified_border = border;
    let (fg, bg, border) = status(0x1e66f5);
    colors.renamed = fg;
    colors.renamed_background = bg;
    colors.renamed_border = border;
    let (fg, bg, border) = status(0x40a02b);
    colors.success = fg;
    colors.success_background = bg;
    colors.success_border = border;
    let (fg, bg, border) = status(0xdf8e1d);
    colors.warning = fg;
    colors.warning_background = bg;
    colors.warning_border = border;
    // — Zed JSON 里 latte 额外给了 hint / predictive / unreachable（JSON 给了完整三色，不派生）—
    colors.hint = h(0xacb0be);
    colors.hint_background = h(0xe6e9ef);
    colors.hint_border = h(0xacb0be);
    colors.predictive = h(0x9ca0b0);
    colors.predictive_background = h(0xe6e9ef);
    colors.predictive_border = h(0x7287fd);
    colors.unreachable = h(0xd20f39);
    colors.unreachable_background = ha(0xd20f39, 0.12);
    colors.unreachable_border = h(0xd20f39);
    colors
}

/// Catppuccin Latte 的 `ThemeColors`（= 浅色基底 [`ThemeColors::light`]）。
///
/// **结构**同 [`theme_colors_macchiato`]：`ThemeColors::zed_light()` 起步，
/// 只覆盖 CTP 定义了的角色；未覆盖的 33 个字段**来自 zed 默认配色**
/// （vim_* 全套、editor_diff_hunk_*、editor_code_lens_foreground、
/// editor_highlighted_line_background、editor_hover_line_number、
/// element_selection_background、version_control_word_*、
/// vim_helix_jump_label_foreground、drop_target_border、
/// minimap_thumb_border、panel_overlay_hover）。
pub fn theme_colors_latte() -> ThemeColors {
    let system = crate::styles::SystemColors::default();

    ThemeColors {
        border: h(0xccd0da),
        border_variant: h(0xbcc0cc),
        border_focused: h(0x1e66f5),
        border_selected: h(0x1e66f5),
        border_disabled: h(0xccd0da),
        border_transparent: hsla(0., 0., 0., 0.),
        background: h(0xeff1f5),
        surface_background: h(0xe6e9ef),
        elevated_surface_background: h(0xffffff),
        element_background: h(0xccd0da),
        element_hover: h(0xbcc0cc),
        element_active: h(0xacb0be),
        element_selected: h(0xbcc0cc),
        element_disabled: h(0xccd0da),
        ghost_element_background: hsla(0., 0., 0., 0.),
        ghost_element_hover: h(0xccd0da),
        ghost_element_active: h(0xbcc0cc),
        ghost_element_selected: h(0xbcc0cc),
        ghost_element_disabled: hsla(0., 0., 0., 0.),
        text: h(0x4c4f69),
        text_muted: h(0x6c6f85),
        text_placeholder: h(0x9ca0b0),
        text_disabled: h(0x8c8fa1),
        text_accent: h(0x1e66f5),
        icon: h(0x4c4f69),
        icon_muted: h(0x6c6f85),
        icon_disabled: h(0xacb0be),
        icon_accent: h(0x1e66f5),
        editor_foreground: h(0x4c4f69),
        editor_background: h(0xeff1f5),
        editor_gutter_background: h(0xeff1f5),
        editor_active_line_background: h(0xccd0da),
        editor_line_number: h(0x9ca0b0),
        editor_active_line_number: h(0x4c4f69),
        editor_wrap_guide: h(0xccd0da),
        editor_indent_guide: h(0xccd0da),
        editor_indent_guide_active: h(0xbcc0cc),
        editor_invisible: h(0x9ca0b0),
        selection_background: h(0xbcc0cc),
        editor_cursor: h(0xdc8a78),
        editor_document_highlight_read_background: ha(0x1e66f5, 0.12),
        editor_document_highlight_write_background: ha(0x1e66f5, 0.2),
        panel_background: h(0xe6e9ef),
        pane_focused_border: h(0x1e66f5),
        pane_group_border: h(0xccd0da),
        search_match_background: h(0xccd0da),
        search_active_match_background: h(0xbcc0cc),
        scrollbar_thumb_background: h(0xacb0be),
        scrollbar_thumb_hover_background: h(0x9ca0b0),
        scrollbar_thumb_active_background: h(0x1e66f5),
        scrollbar_thumb_border: hsla(0., 0., 0., 0.),
        scrollbar_track_background: h(0xe6e9ef),
        scrollbar_track_border: hsla(0., 0., 0., 0.),
        link: h(0x1e66f5),
        // — Zed JSON 里 latte 还显式设置了这些（之前缺失，落回 zed 浅色默认值）—
        icon_placeholder: h(0xacb0be),
        status_bar_background: h(0xdce0e8),
        title_bar_background: h(0xdce0e8),
        title_bar_inactive_background: h(0xe6e9ee),
        toolbar_background: h(0xeff1f5),
        tab_bar_background: h(0xdce0e8),
        tab_inactive_background: h(0xd2d7e2),
        tab_active_background: h(0xeff1f5),
        panel_focused_border: h(0x4c4f69),
        panel_indent_guide: ha(0xccd0da, 0.60),
        panel_indent_guide_active: h(0xacb0be),
        panel_indent_guide_hover: h(0x8839ef),
        panel_overlay_background: h(0xe6e9ef),
        drop_target_background: ha(0xccd0da, 0.40),
        minimap_thumb_background: ha(0x8839ef, 0.20),
        minimap_thumb_hover_background: ha(0x8839ef, 0.40),
        minimap_thumb_active_background: ha(0x8839ef, 0.60),
        editor_subheader_background: h(0xe6e9ef),
        editor_active_wrap_guide: h(0xacb0be),
        editor_document_highlight_bracket_background: ha(0x8839ef, 0.09),
        editor_debugger_active_line_background: ha(0xfe640b, 0.07),
        link_text_hover: h(0x04a5e5),
        debugger_accent: h(0xd20f39),
        version_control_added: h(0x40a02b),
        version_control_deleted: h(0xd20f39),
        version_control_modified: h(0xdf8e1d),
        version_control_renamed: h(0x209fb5),
        version_control_conflict: h(0xfe640b),
        version_control_ignored: h(0x9ca0b0),
        version_control_conflict_marker_ours: ha(0x40a02b, 0.20),
        version_control_conflict_marker_theirs: ha(0x1e66f5, 0.20),
        terminal_background: h(0xeff1f5),
        terminal_ansi_background: h(0xeff1f5),
        terminal_foreground: h(0x4c4f69),
        terminal_dim_foreground: h(0x8c8fa1),
        terminal_bright_foreground: h(0x4c4f69),
        terminal_ansi_black: h(0x5c5f77),
        terminal_ansi_white: h(0xacb0be),
        terminal_ansi_red: h(0xd20f39),
        terminal_ansi_green: h(0x40a02b),
        terminal_ansi_yellow: h(0xdf8e1d),
        terminal_ansi_blue: h(0x1e66f5),
        terminal_ansi_magenta: h(0xea76cb),
        terminal_ansi_cyan: h(0x179299),
        terminal_ansi_bright_black: h(0x6c6f85),
        terminal_ansi_bright_white: h(0xbcc0cc),
        terminal_ansi_bright_red: h(0xde293e),
        terminal_ansi_bright_green: h(0x49af3d),
        terminal_ansi_bright_yellow: h(0xeea02d),
        terminal_ansi_bright_blue: h(0x456eff),
        terminal_ansi_bright_magenta: h(0xfe85d8),
        terminal_ansi_bright_cyan: h(0x2d9fa8),
        terminal_ansi_dim_black: h(0x5c5f77),
        terminal_ansi_dim_white: h(0xacb0be),
        terminal_ansi_dim_red: h(0xd20f39),
        terminal_ansi_dim_green: h(0x40a02b),
        terminal_ansi_dim_yellow: h(0xdf8e1d),
        terminal_ansi_dim_blue: h(0x1e66f5),
        terminal_ansi_dim_magenta: h(0xea76cb),
        terminal_ansi_dim_cyan: h(0x179299),
        // ── 以下字段 CTP 调色板未定义，取自 zed 默认配色 ──
        // （ThemeColors::zed_light() 的同名值，显式写出便于审阅/微调）
        drop_target_border: neutral().light().step_12(), // zed 默认（CTP 调色板未定义）
        editor_code_lens_foreground: None,
        editor_diff_hunk_added_background: ADDED_COLOR.opacity(0.16), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_added_hollow_background: ADDED_COLOR.opacity(0.08), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_added_hollow_border: ADDED_COLOR.opacity(0.48), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_deleted_background: REMOVED_COLOR.opacity(0.16), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_deleted_hollow_background: REMOVED_COLOR.opacity(0.08), // zed 默认（CTP 调色板未定义）
        editor_diff_hunk_deleted_hollow_border: REMOVED_COLOR.opacity(0.48), // zed 默认（CTP 调色板未定义）
        editor_highlighted_line_background: neutral().light_alpha().step_3(), // zed 默认（CTP 调色板未定义）
        editor_hover_line_number: neutral().light().step_12(), // zed 默认（CTP 调色板未定义）
        element_selection_background: blue().light().step_3().alpha(0.25), // zed 默认（CTP 调色板未定义）
        minimap_thumb_border: gpui::transparent_black(), // zed 默认（CTP 调色板未定义）
        panel_overlay_hover: neutral().light().step_4(), // zed 默认（CTP 调色板未定义）
        version_control_word_added: WORD_ADDED_COLOR,    // zed 默认（CTP 调色板未定义）
        version_control_word_deleted: WORD_DELETED_COLOR, // zed 默认（CTP 调色板未定义）
        vim_helix_jump_label_foreground: red().light().step_9(), // zed 默认（CTP 调色板未定义）
        vim_helix_normal_background: system.transparent, // zed 默认（CTP 调色板未定义）
        vim_helix_normal_foreground: system.transparent, // zed 默认（CTP 调色板未定义）
        vim_helix_select_background: system.transparent, // zed 默认（CTP 调色板未定义）
        vim_helix_select_foreground: system.transparent, // zed 默认（CTP 调色板未定义）
        vim_insert_background: system.transparent,       // zed 默认（CTP 调色板未定义）
        vim_insert_foreground: system.transparent,       // zed 默认（CTP 调色板未定义）
        vim_normal_background: system.transparent,       // zed 默认（CTP 调色板未定义）
        vim_normal_foreground: system.transparent,       // zed 默认（CTP 调色板未定义）
        vim_replace_background: system.transparent,      // zed 默认（CTP 调色板未定义）
        vim_replace_foreground: system.transparent,      // zed 默认（CTP 调色板未定义）
        vim_visual_background: system.transparent,       // zed 默认（CTP 调色板未定义）
        vim_visual_block_background: system.transparent, // zed 默认（CTP 调色板未定义）
        vim_visual_block_foreground: system.transparent, // zed 默认（CTP 调色板未定义）
        vim_visual_foreground: system.transparent,       // zed 默认（CTP 调色板未定义）
        vim_visual_line_background: system.transparent,  // zed 默认（CTP 调色板未定义）
        vim_visual_line_foreground: system.transparent,  // zed 默认（CTP 调色板未定义）
        vim_yank_background: neutral().light_alpha().step_3(), // zed 默认（CTP 调色板未定义）
    }
}

#[cfg(test)]
mod tests {
    //! 内置 Catppuccin 主题「必须与 JSON 一致」的回归测试。
    //!
    //! 这些字段曾长期漏配（落到 zed 通用 dark/light 默认色），表现为断点色
    //! 不是 CTP red、编辑器 git diff 色不是 CTP 绿/黄/红。这里锁住
    //! `assets/themes/catppuccin/catppuccin-mauve.json` 的对应值。
    use super::*;

    fn assert_ctp(colors: &ThemeColors, debugger: u32, added: u32, modified: u32, deleted: u32) {
        assert_eq!(
            colors.debugger_accent,
            gpui::rgb(debugger).into(),
            "debugger.accent 应取自 CTP 调色板"
        );
        assert_eq!(colors.version_control_added, gpui::rgb(added).into());
        assert_eq!(colors.version_control_modified, gpui::rgb(modified).into());
        assert_eq!(colors.version_control_deleted, gpui::rgb(deleted).into());
    }

    #[test]
    fn macchiato_uses_catppuccin_debugger_and_version_control_colors() {
        assert_ctp(
            &theme_colors_macchiato(),
            0xed8796,
            0xa6da95,
            0xeed49f,
            0xed8796,
        );
    }

    #[test]
    fn latte_uses_catppuccin_debugger_and_version_control_colors() {
        assert_ctp(
            &theme_colors_latte(),
            0xd20f39,
            0x40a02b,
            0xdf8e1d,
            0xd20f39,
        );
    }
}
