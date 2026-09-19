//! 协作者配色，对齐 zed `styles/players.rs`。
//!
//! 用途：多人协作或 AI agent 出现在编辑器里时，按 **player 索引**给每个
//! 参与者一组三色（光标 / 选区背景 / 选区），让"谁是谁"一眼可辨。
//!
//! **当前只到数据层**：我们还没有 decoration 机制（渲染层画非本地光标与
//! 选区），因此这里只提供类型与配色表，等 `_32_painting` 那步到位后再接
//! 渲染。这与 [`AccentColors`](super::AccentColors) 一样属于"先备着"。
//!
//! 三色同样取自 [`default_colors`](crate::default_colors) 的色阶（对齐 zed）：
//!
//! | 主题 | 光标 | 选区背景 | 选区边框 |
//! |---|---|---|---|
//! | 深色 | `step_9`（饱和） | `step_5`（压暗） | `step_3`（更暗） |
//! | 浅色 | `step_9` | `step_4`（提亮） | `step_3` |
//!
//! 深浅两套在同色相的色阶上取不同步，所以浅色主题下的选区不会在浅底上糊掉。

use gpui::Hsla;

use crate::default_colors::{amber, blue, jade, lime, orange, pink, purple, red};

/// 单个协作者的三色（对齐 zed `PlayerColor`）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerColor {
    /// 光标竖线色。
    pub cursor: Hsla,
    /// 选区背景色。
    pub background: Hsla,
    /// 选区边框色。
    pub selection: Hsla,
}

/// 协作者配色表（对齐 zed `PlayerColors`）。
///
/// 约定（同 zed）：**第一个永远是本地玩家**（蓝色）；其余在色环上
/// 来回跳跃排列，让相邻参与者的颜色尽量不相近。
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerColors(pub Vec<PlayerColor>);

impl Default for PlayerColors {
    fn default() -> Self {
        Self::dark()
    }
}

impl PlayerColors {
    /// 深色主题下的 8 色。
    pub fn dark() -> Self {
        Self(vec![
            dark_player(blue()),
            dark_player(orange()),
            dark_player(pink()),
            dark_player(lime()),
            dark_player(purple()),
            dark_player(amber()),
            dark_player(jade()),
            dark_player(red()),
        ])
    }

    /// 浅色主题下的 8 色（背景步进更浅，便于压在浅底上）。
    pub fn light() -> Self {
        Self(vec![
            light_player(blue()),
            light_player(orange()),
            light_player(pink()),
            light_player(lime()),
            light_player(purple()),
            light_player(amber()),
            light_player(jade()),
            light_player(red()),
        ])
    }
}

/// 深色主题下由色相取三色：光标饱和、背景压暗。
fn dark_player(scale: crate::ColorScaleSet) -> PlayerColor {
    PlayerColor {
        cursor: scale.dark().step_9(),
        background: scale.dark().step_5(),
        selection: scale.dark().step_3(),
    }
}

/// 浅色主题下由色相取三色：光标饱和、背景提亮。
fn light_player(scale: crate::ColorScaleSet) -> PlayerColor {
    PlayerColor {
        cursor: scale.light().step_9(),
        background: scale.light().step_4(),
        selection: scale.light().step_3(),
    }
}

impl PlayerColors {
    /// 本地玩家的颜色（第一个）。
    pub fn local(&self) -> PlayerColor {
        self.0.first().copied().unwrap_or_default()
    }

    /// 颜色总数。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 只读场景的本地色（把本地色去饱和化成灰）。
    pub fn read_only(&self) -> PlayerColor {
        let local = self.local();
        PlayerColor {
            cursor: local.cursor.grayscale(),
            background: local.background.grayscale(),
            selection: local.selection.grayscale(),
        }
    }

    /// 第 `participant_index` 个参与者的颜色。
    ///
    /// 与 zed 一致：跳过第 0 个（那是本地玩家），在剩余的 1..len 里轮转。
    pub fn color_for_participant(&self, participant_index: u32) -> PlayerColor {
        if self.0.len() <= 1 {
            return self.local();
        }
        let rest = self.0.len() - 1;
        self.0[(participant_index as usize % rest) + 1]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_color_is_local_and_stable() {
        let colors = PlayerColors::dark();
        assert_eq!(colors.local(), colors.0[0], "第一个永远是本地玩家");
        assert_eq!(colors.len(), 8);
    }

    #[test]
    fn participant_colors_skip_local_and_wrap() {
        let colors = PlayerColors::dark();
        // 参与者 0 → 索引 1；索引 7 是最后一个,再往下轮回到索引 1
        assert_eq!(colors.color_for_participant(0), colors.0[1]);
        assert_eq!(colors.color_for_participant(6), colors.0[7]);
        assert_eq!(colors.color_for_participant(7), colors.0[1], "应轮转回开头");
    }

    #[test]
    fn read_only_is_grayscale() {
        let colors = PlayerColors::dark();
        let read_only = colors.read_only();
        assert_eq!(read_only.cursor.s, 0., "只读色应完全去饱和");
        assert_eq!(
            read_only.cursor.l,
            colors.local().cursor.l,
            "明度应保持不变"
        );
    }

    #[test]
    fn three_colors_use_different_steps() {
        // 三色取自同一色相的不同步：光标最饱和(9)、选区背景更暗(5)、
        // 选区边框最暗(3)。所以明度应当依次递减 —— 这是"同一玩家
        // 一眼可辨"的视觉基础。
        for player in PlayerColors::dark().0 {
            assert!(
                player.cursor.l > player.background.l,
                "光标应比选区背景亮: {player:?}"
            );
            assert!(
                player.background.l > player.selection.l,
                "选区背景应比选区边框亮: {player:?}"
            );
        }
    }

    #[test]
    fn colors_are_opaque() {
        // 色阶本身不透明；zed 的 players 也不额外加透明度
        // （对比度靠 step 之间的明度差，而不是 alpha）。
        for player in PlayerColors::dark().0 {
            assert_eq!(player.cursor.a, 1.0, "光标应不透明: {player:?}");
            assert_eq!(player.background.a, 1.0, "背景应不透明: {player:?}");
            assert_eq!(player.selection.a, 1.0, "选区应不透明: {player:?}");
        }
    }
}
