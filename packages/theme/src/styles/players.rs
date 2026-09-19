//! 协作者配色，对齐 zed `styles/players.rs`。
//!
//! 用途：多人协作或 AI agent 出现在编辑器里时，按 **player 索引**给每个
//! 参与者一组三色（光标 / 选区背景 / 选区），让"谁是谁"一眼可辨。
//!
//! **当前只到数据层**：我们还没有 decoration 机制（渲染层画非本地光标与
//! 选区），因此这里只提供类型与配色表，等 `_32_painting` 那步到位后再接
//! 渲染。这与 [`AccentColors`](super::AccentColors) 一样属于"先备着"。
//!
//! 与 zed 的差异：zed 用 `palette` 的色板函数（`blue().dark().step_9()`）
//! 生成阶梯色；gpui 0.2 只导出 blue/green/yellow/red 四个色板函数，
//! 其余用 `hsla` 显式给出——做法与 [`AccentColors`](super::AccentColors)
//! 的默认值一致。

use gpui::Hsla;

/// 单个协作者的三色（对齐 zed `PlayerColor`）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerColor {
    /// 光标竖线色。
    pub cursor: Hsla,
    /// 选区背景色（应低透明度，避免盖住文字）。
    pub background: Hsla,
    /// 选区边框色。
    pub selection: Hsla,
}

/// 协作者配色表（对齐 zed `PlayerColors`）。
///
/// 约定（同 zed）：**第一个永远是本地玩家**（通常蓝色）；其余在色环上
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
            player(gpui::blue()),
            player(gpui::hsla(0.07, 0.75, 0.62, 1.)),
            player(gpui::hsla(0.9, 0.65, 0.68, 1.)),
            player(gpui::green()),
            player(gpui::hsla(0.75, 0.6, 0.66, 1.)),
            player(gpui::yellow()),
            player(gpui::hsla(0.45, 0.55, 0.58, 1.)),
            player(gpui::red()),
        ])
    }

    /// 浅色主题下的 8 色（背景步进更浅，便于压在浅底上）。
    pub fn light() -> Self {
        Self(vec![
            player_light(gpui::blue()),
            player_light(gpui::hsla(0.07, 0.7, 0.55, 1.)),
            player_light(gpui::hsla(0.9, 0.6, 0.6, 1.)),
            player_light(gpui::green()),
            player_light(gpui::hsla(0.75, 0.55, 0.58, 1.)),
            player_light(gpui::yellow()),
            player_light(gpui::hsla(0.45, 0.5, 0.5, 1.)),
            player_light(gpui::red()),
        ])
    }
}

/// 深色主题下由基色派生三色：光标用原色，背景/边框压暗并给透明度。
fn player(base: Hsla) -> PlayerColor {
    PlayerColor {
        cursor: base,
        background: with_alpha(darken(base, 0.55), 0.35),
        selection: with_alpha(darken(base, 0.4), 0.55),
    }
}

/// 浅色主题下由基色派生三色：光标用原色，背景/边框提亮并给透明度。
fn player_light(base: Hsla) -> PlayerColor {
    PlayerColor {
        cursor: base,
        background: with_alpha(lighten(base, 0.35), 0.30),
        selection: with_alpha(lighten(base, 0.2), 0.50),
    }
}

fn darken(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (color.l - amount).max(0.),
        ..color
    }
}

fn lighten(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (color.l + amount).min(1.),
        ..color
    }
}

fn with_alpha(mut color: Hsla, alpha: f32) -> Hsla {
    color.a = alpha;
    color
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
    ///
    /// 对齐 zed `read_only()`：zed 用 `Hsla::grayscale()`，我们手写
    /// （gpui 0.2 的 `Hsla` 没有该方法）——饱和度归零即灰。
    pub fn read_only(&self) -> PlayerColor {
        let local = self.local();
        PlayerColor {
            cursor: grayscale(local.cursor),
            background: grayscale(local.background),
            selection: grayscale(local.selection),
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

/// 去饱和成灰（保留原有明度与透明度）。
fn grayscale(color: Hsla) -> Hsla {
    Hsla {
        s: 0.,
        ..color
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
    fn backgrounds_are_translucent() {
        // 选区背景必须带透明度,否则会盖住文字
        for player in PlayerColors::dark().0 {
            assert!(player.background.a < 1.0, "背景应半透明: {player:?}");
            assert!(player.selection.a < 1.0, "选区应半透明: {player:?}");
            assert_eq!(player.cursor.a, 1.0, "光标应不透明: {player:?}");
        }
    }
}
