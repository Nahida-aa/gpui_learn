//! 色阶体系，对齐 zed `crates/theme/src/scale.rs`。
//!
//! 这是主题配色的**底层抽象**：一个色阶（[`ColorScale`]）统一用 12 步表达
//! 「从最浅到最深」，每步有明确语义（见 [`ColorScaleStep`] 各常量的文档）。
//! [`ColorScaleSet`] 把同一色相的四种形态打成一包——浅色/深色 × 实色/
//! 半透明——所以主题切换只改「取哪一套」，不换色相。
//!
//! 有了这层，`AccentColors` / `PlayerColors` 那种"按色相取第 9 步"的需求
//! 就能写成 `blue().dark().step_9()`，而不是散落的手写 `hsla(...)`。
//!
//! 与 zed 的差异：zed 把色值表放在 `default_colors.rs`（2500 行），这里
//! 同样分工——本文件只有**类型与算法**，色值在 [`crate::default_colors`]。

use gpui::{App, Hsla, SharedString};

use crate::{ActiveTheme, Appearance};

/// 色阶中的一步（步索引从 1 开始，对齐 zed）。
///
/// 每一步都有既定的语义用途，直接沿用 zed 的约定，便于将来对接
/// zed 主题扩展。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ColorScaleStep(usize);

impl ColorScaleStep {
    /// 第 1 步：应用主背景。
    pub const ONE: Self = Self(1);
    /// 第 2 步：主背景 / 细微组件背景（斑马纹表格、侧栏、卡片）。
    pub const TWO: Self = Self(2);
    /// 第 3 步：UI 组件常态背景（与 11/12 步保证 4.5:1 对比度）。
    pub const THREE: Self = Self(3);
    /// 第 4 步：UI 组件 hover 背景。
    pub const FOUR: Self = Self(4);
    /// 第 5 步：UI 组件按下 / 选中背景。
    pub const FIVE: Self = Self(5);
    /// 第 6 步：非交互组件的细微边框（分隔线、卡片描边）。
    pub const SIX: Self = Self(6);
    /// 第 7 步：交互组件的细微边框。
    pub const SEVEN: Self = Self(7);
    /// 第 8 步：交互组件的强边框与 focus ring。
    pub const EIGHT: Self = Self(8);
    /// 第 9 步：实色背景。**饱和度最高的一步**，语义色（error / warning /
    /// success）与强调色通常取这里。
    pub const NINE: Self = Self(9);
    /// 第 10 步：实色背景的 hover / active 态。
    pub const TEN: Self = Self(10);
    /// 第 11 步：低对比度文字与图标。
    pub const ELEVEN: Self = Self(11);
    /// 第 12 步：高对比度文字与图标。
    pub const TWELVE: Self = Self(12);

    /// 全部 12 步，按顺序。
    pub const ALL: [ColorScaleStep; 12] = [
        Self::ONE,
        Self::TWO,
        Self::THREE,
        Self::FOUR,
        Self::FIVE,
        Self::SIX,
        Self::SEVEN,
        Self::EIGHT,
        Self::NINE,
        Self::TEN,
        Self::ELEVEN,
        Self::TWELVE,
    ];
}

/// 一个色阶：某一色相下的 12 个颜色，从最浅到最深。
#[derive(Debug, Clone, PartialEq)]
pub struct ColorScale(Vec<Hsla>);

impl FromIterator<Hsla> for ColorScale {
    fn from_iter<T: IntoIterator<Item = Hsla>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl ColorScale {
    /// 取指定步（步索引从 1 开始，内部转成 0 起的下标）。
    #[inline]
    pub fn step(&self, step: ColorScaleStep) -> Hsla {
        self.0[step.0 - 1]
    }

    /// 第 1 步。
    #[inline]
    pub fn step_1(&self) -> Hsla {
        self.step(ColorScaleStep::ONE)
    }
    /// 第 2 步。
    #[inline]
    pub fn step_2(&self) -> Hsla {
        self.step(ColorScaleStep::TWO)
    }
    /// 第 3 步。
    #[inline]
    pub fn step_3(&self) -> Hsla {
        self.step(ColorScaleStep::THREE)
    }
    /// 第 4 步。
    #[inline]
    pub fn step_4(&self) -> Hsla {
        self.step(ColorScaleStep::FOUR)
    }
    /// 第 5 步。
    #[inline]
    pub fn step_5(&self) -> Hsla {
        self.step(ColorScaleStep::FIVE)
    }
    /// 第 6 步。
    #[inline]
    pub fn step_6(&self) -> Hsla {
        self.step(ColorScaleStep::SIX)
    }
    /// 第 7 步。
    #[inline]
    pub fn step_7(&self) -> Hsla {
        self.step(ColorScaleStep::SEVEN)
    }
    /// 第 8 步。
    #[inline]
    pub fn step_8(&self) -> Hsla {
        self.step(ColorScaleStep::EIGHT)
    }
    /// 第 9 步（饱和度最高，强调色 / 语义色常用）。
    #[inline]
    pub fn step_9(&self) -> Hsla {
        self.step(ColorScaleStep::NINE)
    }
    /// 第 10 步。
    #[inline]
    pub fn step_10(&self) -> Hsla {
        self.step(ColorScaleStep::TEN)
    }
    /// 第 11 步。
    #[inline]
    pub fn step_11(&self) -> Hsla {
        self.step(ColorScaleStep::ELEVEN)
    }
    /// 第 12 步。
    #[inline]
    pub fn step_12(&self) -> Hsla {
        self.step(ColorScaleStep::TWELVE)
    }

    /// 色阶中的颜色个数（正常是 12）。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// 全部色板（对齐 zed `ColorScales`，33 个）。
///
/// 命名沿用 zed：中性色（`gray` / `mauve` / `slate` / `sage` / `olive` /
/// `sand`）与彩色（其余）。
#[derive(Debug, Clone, PartialEq)]
pub struct ColorScales {
    pub gray: ColorScaleSet,
    pub mauve: ColorScaleSet,
    pub slate: ColorScaleSet,
    pub sage: ColorScaleSet,
    pub olive: ColorScaleSet,
    pub sand: ColorScaleSet,
    pub gold: ColorScaleSet,
    pub bronze: ColorScaleSet,
    pub brown: ColorScaleSet,
    pub yellow: ColorScaleSet,
    pub amber: ColorScaleSet,
    pub orange: ColorScaleSet,
    pub tomato: ColorScaleSet,
    pub red: ColorScaleSet,
    pub ruby: ColorScaleSet,
    pub crimson: ColorScaleSet,
    pub pink: ColorScaleSet,
    pub plum: ColorScaleSet,
    pub purple: ColorScaleSet,
    pub violet: ColorScaleSet,
    pub iris: ColorScaleSet,
    pub indigo: ColorScaleSet,
    pub blue: ColorScaleSet,
    pub cyan: ColorScaleSet,
    pub teal: ColorScaleSet,
    pub jade: ColorScaleSet,
    pub green: ColorScaleSet,
    pub grass: ColorScaleSet,
    pub lime: ColorScaleSet,
    pub mint: ColorScaleSet,
    pub sky: ColorScaleSet,
    pub black: ColorScaleSet,
    pub white: ColorScaleSet,
}

impl IntoIterator for ColorScales {
    type Item = ColorScaleSet;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            self.gray,
            self.mauve,
            self.slate,
            self.sage,
            self.olive,
            self.sand,
            self.gold,
            self.bronze,
            self.brown,
            self.yellow,
            self.amber,
            self.orange,
            self.tomato,
            self.red,
            self.ruby,
            self.crimson,
            self.pink,
            self.plum,
            self.purple,
            self.violet,
            self.iris,
            self.indigo,
            self.blue,
            self.cyan,
            self.teal,
            self.jade,
            self.green,
            self.grass,
            self.lime,
            self.mint,
            self.sky,
            self.black,
            self.white,
        ]
        .into_iter()
    }
}

/// 一个色相的四套色阶（对齐 zed `ColorScaleSet`）。
#[derive(Debug, Clone, PartialEq)]
pub struct ColorScaleSet {
    name: SharedString,
    light: ColorScale,
    dark: ColorScale,
    light_alpha: ColorScale,
    dark_alpha: ColorScale,
}

impl ColorScaleSet {
    pub fn new(
        name: impl Into<SharedString>,
        light: ColorScale,
        light_alpha: ColorScale,
        dark: ColorScale,
        dark_alpha: ColorScale,
    ) -> Self {
        Self {
            name: name.into(),
            light,
            light_alpha,
            dark,
            dark_alpha,
        }
    }

    pub fn name(&self) -> &SharedString {
        &self.name
    }

    pub fn light(&self) -> &ColorScale {
        &self.light
    }

    pub fn light_alpha(&self) -> &ColorScale {
        &self.light_alpha
    }

    pub fn dark(&self) -> &ColorScale {
        &self.dark
    }

    pub fn dark_alpha(&self) -> &ColorScale {
        &self.dark_alpha
    }

    /// 按**当前主题外观**取该步的颜色。
    pub fn step(&self, cx: &App, step: ColorScaleStep) -> Hsla {
        match cx.theme().appearance {
            Appearance::Light => self.light().step(step),
            Appearance::Dark => self.dark().step(step),
        }
    }

    /// 同上，但取半透明变体。
    pub fn step_alpha(&self, cx: &App, step: ColorScaleStep) -> Hsla {
        match cx.theme().appearance {
            Appearance::Light => self.light_alpha.step(step),
            Appearance::Dark => self.dark_alpha.step(step),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_colors::{blue, neutral};

    #[test]
    fn steps_are_one_based() {
        let scale = ColorScale::from_iter([1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12.]
            .into_iter()
            .map(|l| gpui::hsla(0., 0., l / 12., 1.)));
        assert_eq!(scale.step_1().l, 1. / 12., "第 1 步应对应下标 0");
        assert_eq!(scale.step_12().l, 1.0);
    }

    #[test]
    fn step_all_covers_every_step() {
        let colors = (1..=12)
            .map(|i| gpui::hsla(0., 0., i as f32 / 12., 1.))
            .collect::<ColorScale>();
        for (index, step) in ColorScaleStep::ALL.into_iter().enumerate() {
            assert_eq!(
                colors.step(step).l,
                (index + 1) as f32 / 12.,
                "ALL 的第 {index} 项应是第 {} 步",
                index + 1
            );
        }
    }

    #[test]
    fn scale_set_keeps_all_four_variants() {
        let set = blue();
        assert_eq!(set.name(), "Blue");
        assert_eq!(set.light().len(), 12);
        assert_eq!(set.dark().len(), 12);
        assert_eq!(set.light_alpha().len(), 12);
        assert_eq!(set.dark_alpha().len(), 12);
        // 深浅两套的第 1 步方向相反:浅色主题的 1 最亮,深色主题的 1 最暗
        assert!(set.light().step_1().l > set.light().step_12().l);
        assert!(set.dark().step_1().l < set.dark().step_12().l);
    }

    #[test]
    fn neutral_aliases_sand() {
        assert_eq!(neutral(), crate::default_colors::sand());
    }
}
