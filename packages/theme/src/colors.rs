//! 主题颜色集,对齐 zed `crates/theme/src/styles/` 的精简子集。
//!
//! 字段取舍原则:编辑器内核 + 通用组件够用即可(约 55 个语义色),
//! zed 的 150 个字段里面板/工作区相关的暂不收录,需要时按同名补齐。
//! 所有颜色统一用 `Hsla`(与 gpui 的填充系统无缝衔接,`rgb(..).into()` 可转)。

use std::sync::Arc;

use gpui::Hsla;

/// UI 语义色(对齐 zed `ThemeColors` 的常用子集,字段名与其保持一致,
/// 便于将来直接对接 zed 的主题 JSON)。
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeColors {
    // ---- border ----
    /// 常规边框,高对比。
    pub border: Hsla,
    /// 弱化边框,如两块区域的分隔线。
    pub border_variant: Hsla,
    /// 键盘焦点边框。
    pub border_focused: Hsla,
    /// 选中态边框。
    pub border_selected: Hsla,
    /// 禁用态边框。
    pub border_disabled: Hsla,
    /// 透明占位边框(状态切换时浮现的边框)。
    pub border_transparent: Hsla,

    // ---- background ----
    /// 应用/窗口背景。
    pub background: Hsla,
    /// 面板、tab 等贴地表面。
    pub surface_background: Hsla,
    /// 浮层表面(菜单、弹窗、对话框)。
    pub elevated_surface_background: Hsla,
    /// 有自己底色的元素(按钮、输入框……)。
    pub element_background: Hsla,
    /// 该类元素的 hover 态。
    pub element_hover: Hsla,
    /// 该类元素的按下态。
    pub element_active: Hsla,
    /// 该类元素的选中态。
    pub element_selected: Hsla,
    /// 该类元素的禁用态。
    pub element_disabled: Hsla,
    /// 与所在表面同底色的「幽灵」元素(纯文本按钮等)。
    pub ghost_element_background: Hsla,
    pub ghost_element_hover: Hsla,
    pub ghost_element_active: Hsla,
    pub ghost_element_selected: Hsla,
    pub ghost_element_disabled: Hsla,

    // ---- text ----
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    pub text_disabled: Hsla,
    pub text_accent: Hsla,

    // ---- icon ----
    pub icon: Hsla,
    pub icon_muted: Hsla,
    pub icon_disabled: Hsla,
    pub icon_accent: Hsla,

    // ---- editor ----
    pub editor_foreground: Hsla,
    pub editor_background: Hsla,
    pub editor_gutter_background: Hsla,
    /// 光标所在行的底色。
    pub editor_active_line_background: Hsla,
    pub editor_line_number: Hsla,
    pub editor_active_line_number: Hsla,
    /// 换行参考线(80 列等)。
    pub editor_wrap_guide: Hsla,
    pub editor_indent_guide: Hsla,
    pub editor_indent_guide_active: Hsla,
    /// 组字下划线等「不可见字符」的颜色。
    pub editor_invisible: Hsla,
    /// 通用选区底色。
    ///
    /// 注:zed 用 `PlayerColors`(协同每人一色),我们简化为单选区字段。
    pub selection_background: Hsla,
    /// 文本光标(caret)。
    ///
    /// 注:同上,zed 用 `PlayerColors` 里的 cursor。
    pub editor_cursor: Hsla,
    pub editor_document_highlight_read_background: Hsla,
    pub editor_document_highlight_write_background: Hsla,

    // ---- 面板 / 杂项 ----
    pub panel_background: Hsla,
    pub pane_focused_border: Hsla,
    pub pane_group_border: Hsla,
    pub search_match_background: Hsla,
    pub search_active_match_background: Hsla,
    pub scrollbar_thumb_background: Hsla,
    pub scrollbar_thumb_hover_background: Hsla,
    pub scrollbar_thumb_active_background: Hsla,
    pub scrollbar_thumb_border: Hsla,
    pub scrollbar_track_background: Hsla,
    pub scrollbar_track_border: Hsla,
    /// 链接色。
    pub link: Hsla,
}

/// 系统层颜色(对齐 zed `SystemColors` 的精简子集)。
#[derive(Clone, Debug, PartialEq)]
pub struct SystemColors {
    pub transparent: Hsla,
}

impl Default for SystemColors {
    fn default() -> Self {
        SystemColors {
            transparent: gpui::hsla(0., 0., 0., 0.),
        }
    }
}

/// 强调色组(对齐 zed `AccentColors`):用于缩进参考线等按行轮换的颜色。
#[derive(Clone, Debug, PartialEq)]
pub struct AccentColors(pub Arc<[Hsla]>);

impl Default for AccentColors {
    fn default() -> Self {
        // gpui 0.2 的色板函数只有 blue/green/yellow/red;其余用 hsla 补齐
        AccentColors(
            [
                gpui::blue(),
                gpui::green(),
                gpui::yellow(),
                gpui::red(),
                gpui::hsla(0.45, 0.6, 0.6, 1.),
                gpui::hsla(0.55, 0.6, 0.6, 1.),
                gpui::hsla(0.8, 0.6, 0.65, 1.),
                gpui::hsla(0.1, 0.7, 0.6, 1.),
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
        )
    }
}

/// 单个 Git/诊断状态的颜色组(base / background / border)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusColor {
    pub base: Hsla,
    pub background: Hsla,
    pub border: Hsla,
}

/// Git 与诊断状态色(对齐 zed `StatusColors`,每状态收敛为三色组)。
#[derive(Clone, Debug, PartialEq)]
pub struct StatusColors {
    pub conflict: StatusColor,
    pub created: StatusColor,
    pub deleted: StatusColor,
    pub error: StatusColor,
    pub hidden: StatusColor,
    pub ignored: StatusColor,
    pub info: StatusColor,
    pub modified: StatusColor,
    pub renamed: StatusColor,
    pub success: StatusColor,
    pub warning: StatusColor,
}
