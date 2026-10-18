//! zed 语义色令牌的 iced 版本（`ThemeColors` 的用得上的子集）。
//!
//! 字段名与 zed 的 `styles/colors.rs` **一一对应**，照抄 zed 界面代码时不用做名字
//! 翻译（`surface_background`、`ghost_element_hover`、`text_muted`…）。
//!
//! 色值来源：Catppuccin Macchiato（深色）/ Latte（浅色），与 GPUI 侧
//! `aa_theme_gpui::default_colors` **同源同值**（末尾注释保留上游「这是哪个
//! catppuccin 色」的标注）。改色值请两边一起改。
//!
//! 本文件说白了就是数据：**不依赖 gpui、不依赖 iced 的控件层**，只用到
//! `iced_core::Color`。

use iced_core::Color;

/// `0xRRGGBB` → 不透明 `Color`。
pub fn rgb(hex: u32) -> Color {
    Color {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

/// `0xRRGGBB` + alpha。
pub fn rgba(hex: u32, a: f32) -> Color {
    Color { a, ..rgb(hex) }
}

/// 状态色（zed `StatusColors` 的前景色子集）。
///
/// 上游每个状态还有配套的 `*_background` / `*_border`（15% 透明底 + 同色边）；
/// 我们暂时只用到前景（线程行上的状态点），需要时再补。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusColors {
    /// 出错（线程失败、命令非零退出）。
    pub error: Color,
    /// 警告（等待确认之类）。
    pub warning: Color,
    /// 提示。
    pub info: Color,
    /// 成功。
    pub success: Color,
}

/// 界面语义色。
///
/// 与 zed 的 `ThemeColors` 同名字段语义一致；**只搬了现在用得上的**（约 40 个），
/// 不是全部 143 个 —— 需要哪个再加，别一次搬空。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    // ── 边框 ──
    /// surface0
    pub border: Color,
    /// surface0-darker
    pub border_variant: Color,
    /// lavender
    pub border_focused: Color,
    /// mauve
    pub border_selected: Color,
    /// overlay0
    pub border_disabled: Color,
    pub border_transparent: Color,

    // ── 表面 / 元素底色 ──
    /// base — Zed JSON "background"
    pub background: Color,
    /// mantle
    pub surface_background: Color,
    /// mantle (同 surface)
    pub elevated_surface_background: Color,
    /// crust
    pub element_background: Color,
    /// surface0
    pub element_hover: Color,
    /// surface2 @30%
    pub element_active: Color,
    /// surface0 @30%
    pub element_selected: Color,
    pub element_disabled: Color,

    // ── 幽灵元素（与所在表面同色，靠悬停/选中态区分） ──
    pub ghost_element_background: Color,
    /// surface1 @30%
    pub ghost_element_hover: Color,
    /// surface2 @60%
    pub ghost_element_active: Color,
    /// overlay2 @40%
    pub ghost_element_selected: Color,
    pub ghost_element_disabled: Color,

    // ── 文字 ──
    /// text
    pub text: Color,
    /// subtext0
    pub text_muted: Color,
    /// surface2
    pub text_placeholder: Color,
    /// overlay0
    pub text_disabled: Color,
    /// mauve
    pub text_accent: Color,

    // ── 图标 ──
    pub icon: Color,
    /// overlay2
    pub icon_muted: Color,
    pub icon_disabled: Color,
    pub icon_accent: Color,
    pub icon_placeholder: Color,

    // ── 面板 / 窗口框架 ──
    /// crust
    pub status_bar_background: Color,
    /// crust
    pub title_bar_background: Color,
    /// base (CTP base #24273a)
    pub toolbar_background: Color,
    /// mantle
    pub panel_background: Color,
    /// text
    pub panel_focused_border: Color,
    /// mantle
    pub panel_overlay_background: Color,

    // ── 滚动条 ──
    /// surface2 @50%
    pub scrollbar_thumb_background: Color,
    /// overlay0
    pub scrollbar_thumb_hover_background: Color,
    /// crust
    pub scrollbar_track_background: Color,

    // ── 查找命中底色 ──
    /// teal @30%
    pub search_match_background: Color,
    /// red @30%
    pub search_active_match_background: Color,

    /// 错误 / 警告 / 成功 / 提示四态的前景色。
    pub status: StatusColors,
}

/// Macchiato 的四态前景色（与 `aa_theme_gpui::default_colors::status_colors_macchiato` 同值）。
fn macchiato_status() -> StatusColors {
    StatusColors {
        error: rgb(0xed8796),   // red
        warning: rgb(0xf5deb3), // wheat
        info: rgb(0x91d7e3),    // sky
        success: rgb(0xa6da95), // green
    }
}

/// Latte 的四态前景色（同上，浅色版）。
fn latte_status() -> StatusColors {
    StatusColors {
        error: rgb(0xd20f39),   // red
        warning: rgb(0xdf8e1d), // yellow
        info: rgb(0x179299),    // teal
        success: rgb(0x40a02b), // green
    }
}

/// Catppuccin Macchiato（深色内置主题）。
pub(crate) fn macchiato() -> Colors {
    Colors {
        // 边框
        border: rgb(0x363a4f),          // surface0
        border_variant: rgb(0x313148),  // surface0-darker
        border_focused: rgb(0xb7bdf8),  // lavender
        border_selected: rgb(0xc6a0f6), // mauve
        border_disabled: rgb(0x6e738d), // overlay0
        border_transparent: Color::TRANSPARENT,

        // 表面 / 元素底色
        background: rgb(0x2c2f46),         // base — Zed JSON "background"
        surface_background: rgb(0x1e2030), // mantle
        elevated_surface_background: rgb(0x1e2030), // mantle (同 surface)
        element_background: rgb(0x181926), // crust
        element_hover: rgba(0x363a4f, 1.0), // surface0
        element_active: rgba(0x5b6078, 0.30), // surface2 @30%
        element_selected: rgba(0x363a4f, 0.30), // surface0 @30%
        element_disabled: rgb(0x6e738d),

        // 幽灵元素（与所在表面同色，靠悬停/选中态区分）
        ghost_element_background: Color::TRANSPARENT,
        ghost_element_hover: rgba(0x494d64, 0.30), // surface1 @30%
        ghost_element_active: rgba(0x5b6078, 0.60), // surface2 @60%
        ghost_element_selected: rgba(0x737995, 0.40), // overlay2 @40%
        ghost_element_disabled: rgb(0x6e738d),

        // 文字
        text: rgb(0xcad3f5),             // text
        text_muted: rgb(0xb8c0e0),       // subtext0
        text_placeholder: rgb(0x5b6078), // surface2
        text_disabled: rgb(0x6e738d),    // overlay0
        text_accent: rgb(0xc6a0f6),      // mauve

        // 图标
        icon: rgb(0xcad3f5),
        icon_muted: rgb(0x8087a2), // overlay2
        icon_disabled: rgb(0x6e738d),
        icon_accent: rgb(0xc6a0f6),
        icon_placeholder: rgb(0x5b6078),

        // 面板 / 窗口框架
        status_bar_background: rgb(0x181926),    // crust
        title_bar_background: rgb(0x181926),     // crust
        toolbar_background: rgb(0x24273a),       // base (CTP base #24273a)
        panel_background: rgb(0x1e2030),         // mantle
        panel_focused_border: rgb(0xcad3f5),     // text
        panel_overlay_background: rgb(0x1e2030), // mantle

        // 滚动条
        scrollbar_thumb_background: rgba(0x5b6078, 0.50), // surface2 @50%
        scrollbar_thumb_hover_background: rgb(0x6e738d),  // overlay0
        scrollbar_track_background: rgb(0x181926),        // crust

        // 查找命中底色
        search_match_background: rgba(0x8bd5ca, 0.30), // teal @30%
        search_active_match_background: rgba(0xed8796, 0.30), // red @30%

        status: macchiato_status(),
    }
}

/// Catppuccin Latte（浅色内置主题）。
pub(crate) fn latte() -> Colors {
    Colors {
        // 边框
        border: rgb(0xccd0da),
        border_variant: rgb(0xbcc0cc),
        border_focused: rgb(0x1e66f5),
        border_selected: rgb(0x1e66f5),
        border_disabled: rgb(0xccd0da),
        border_transparent: Color::TRANSPARENT,

        // 表面 / 元素底色
        background: rgb(0xeff1f5),
        surface_background: rgb(0xe6e9ef),
        elevated_surface_background: rgb(0xffffff),
        element_background: rgb(0xccd0da),
        element_hover: rgb(0xbcc0cc),
        element_active: rgb(0xacb0be),
        element_selected: rgb(0xbcc0cc),
        element_disabled: rgb(0xccd0da),

        // 幽灵元素（与所在表面同色，靠悬停/选中态区分）
        ghost_element_background: Color::TRANSPARENT,
        ghost_element_hover: rgb(0xccd0da),
        ghost_element_active: rgb(0xbcc0cc),
        ghost_element_selected: rgb(0xbcc0cc),
        ghost_element_disabled: Color::TRANSPARENT,

        // 文字
        text: rgb(0x4c4f69),
        text_muted: rgb(0x6c6f85),
        text_placeholder: rgb(0x9ca0b0),
        text_disabled: rgb(0x8c8fa1),
        text_accent: rgb(0x1e66f5),

        // 图标
        icon: rgb(0x4c4f69),
        icon_muted: rgb(0x6c6f85),
        icon_disabled: rgb(0xacb0be),
        icon_accent: rgb(0x1e66f5),
        icon_placeholder: rgb(0xacb0be),

        // 面板 / 窗口框架
        status_bar_background: rgb(0xdce0e8),
        title_bar_background: rgb(0xdce0e8),
        toolbar_background: rgb(0xeff1f5),
        panel_background: rgb(0xe6e9ef),
        panel_focused_border: rgb(0x4c4f69),
        panel_overlay_background: rgb(0xe6e9ef),

        // 滚动条
        scrollbar_thumb_background: rgb(0xacb0be),
        scrollbar_thumb_hover_background: rgb(0x9ca0b0),
        scrollbar_track_background: rgb(0xe6e9ef),

        // 查找命中底色
        search_match_background: rgb(0xccd0da),
        search_active_match_background: rgb(0xbcc0cc),

        status: latte_status(),
    }
}
