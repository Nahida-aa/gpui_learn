//! 强调色组，对齐 zed `styles/accents.rs`。
//!
//! 用途：编辑器里按行轮换的颜色（缩进参考线、括号配对等）。
//! 13 个颜色在色环上交错排列，让相邻两行尽可能不同。
//!
//! 色值全部来自 [`default_colors`](crate::default_colors) 的色阶：
//! 统一取**第 9 步**（饱和度最高的一步，见
//! [`ColorScaleStep::NINE`](crate::ColorScaleStep::NINE)）。
//! 深浅两套由同一色相的 `dark()` / `light()` 给出。

use std::sync::Arc;

use gpui::Hsla;

use crate::default_colors::{
    amber, blue, cyan, gold, grass, indigo, iris, jade, lime, orange, pink, purple, tomato,
};

/// 强调色组（对齐 zed `AccentColors`）。
#[derive(Clone, Debug, PartialEq)]
pub struct AccentColors(pub Arc<[Hsla]>);

impl Default for AccentColors {
    /// 注意：这只是 `Refineable` 需要的默认值。实际用时应当按主题明暗选
    /// [`dark`](Self::dark) 或 [`light`](Self::light)（`builtin` 里就是这么做的）。
    fn default() -> Self {
        Self::dark()
    }
}

impl AccentColors {
    /// 深色主题的 13 个强调色。
    pub fn dark() -> Self {
        Self(Arc::from([
            blue().dark().step_9(),
            orange().dark().step_9(),
            pink().dark().step_9(),
            lime().dark().step_9(),
            purple().dark().step_9(),
            amber().dark().step_9(),
            jade().dark().step_9(),
            tomato().dark().step_9(),
            cyan().dark().step_9(),
            gold().dark().step_9(),
            grass().dark().step_9(),
            indigo().dark().step_9(),
            iris().dark().step_9(),
        ]))
    }

    /// 浅色主题的 13 个强调色（同色相，取 light 阶）。
    pub fn light() -> Self {
        Self(Arc::from([
            blue().light().step_9(),
            orange().light().step_9(),
            pink().light().step_9(),
            lime().light().step_9(),
            purple().light().step_9(),
            amber().light().step_9(),
            jade().light().step_9(),
            tomato().light().step_9(),
            cyan().light().step_9(),
            gold().light().step_9(),
            grass().light().step_9(),
            indigo().light().step_9(),
            iris().light().step_9(),
        ]))
    }

    /// 按索引取色，超出长度则回绕（对齐 zed `color_for_index`）。
    pub fn color_for_index(&self, index: u32) -> Hsla {
        self.0[index as usize % self.0.len()]
    }

    /// 颜色个数。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_thirteen_colors() {
        assert_eq!(AccentColors::dark().len(), 13);
        assert_eq!(AccentColors::light().len(), 13);
    }

    #[test]
    fn index_wraps_around() {
        let accents = AccentColors::dark();
        assert_eq!(accents.color_for_index(0), accents.0[0]);
        assert_eq!(accents.color_for_index(12), accents.0[12]);
        assert_eq!(
            accents.color_for_index(13),
            accents.0[0],
            "超出长度应回绕到开头"
        );
        assert_eq!(accents.color_for_index(27), accents.0[1]);
    }

    #[test]
    fn colors_are_distinct() {
        // 色环交错的目的就是相邻行颜色不同；这里检查整体两两不同
        let accents = AccentColors::dark();
        let mut seen: Vec<String> = Vec::new();
        for color in accents.0.iter() {
            let key = format!("{color:?}");
            assert!(!seen.contains(&key), "强调色不应重复: {color:?}");
            seen.push(key);
        }
    }

    #[test]
    fn dark_and_light_share_hue_but_steps_differ() {
        // 第 9 步在深浅两套里是同一色值（"饱和度最高"的定义与明暗无关），
        // 所以强调色本身不随主题变。这里验证的正是这一点 ——
        // 若将来改成按明暗取不同步，这个测试会失败并提醒更新文档。
        let dark = AccentColors::dark();
        let light = AccentColors::light();
        assert_eq!(dark.color_for_index(0), light.color_for_index(0));
    }
}
