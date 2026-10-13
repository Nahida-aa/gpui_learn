//! Git 与诊断状态色，对齐 zed `styles/status.rs`。
//!
//! 结构**与 zed 完全一致**：每个状态三个独立字段——`error`（前景）、
//! `error_background`、`error_border`。共 15 个状态 × 3 = 45 个字段。
//!
//! 为什么不用嵌套的三色组（`error: StatusColor { base, background, border }`）:
//! zed 主题扩展 JSON 用**扁平 dotted key**（`"error"` / `"error.background"` /
//! `"error.border"`），字段名与之一一对应才能直接映射；`Refineable` 的逐字段
//! 覆盖也需要扁平结构。取值路径因此是 `status.error_background`。

use gpui::Hsla;
use refineable::Refineable;

/// 单一诊断/状态色（对齐 zed `DiagnosticColors`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagnosticColors {
    pub error: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
}

/// Git 与诊断状态色（字段与注释沿用 zed 原文）。
#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug)]
pub struct StatusColors {
    /// 冲突：文件在打开时被外部修改，或 Git 合并冲突。
    pub conflict: Hsla,
    pub conflict_background: Hsla,
    pub conflict_border: Hsla,

    /// 新增：如 Git 里新添加的文件。
    pub created: Hsla,
    pub created_background: Hsla,
    pub created_border: Hsla,

    /// 删除：如被删除的文件。
    pub deleted: Hsla,
    pub deleted_background: Hsla,
    pub deleted_border: Hsla,

    /// 错误：系统错误、失败的操作、诊断错误。
    pub error: Hsla,
    pub error_background: Hsla,
    pub error_border: Hsla,

    /// 隐藏：如文件树里被隐藏的文件。
    pub hidden: Hsla,
    pub hidden_background: Hsla,
    pub hidden_border: Hsla,

    /// 提示：额外的补充信息。
    pub hint: Hsla,
    pub hint_background: Hsla,
    pub hint_border: Hsla,

    /// 忽略：如被 Git 忽略的文件或操作。
    pub ignored: Hsla,
    pub ignored_background: Hsla,
    pub ignored_border: Hsla,

    /// 信息：告知性状态更新或消息。
    pub info: Hsla,
    pub info_background: Hsla,
    pub info_border: Hsla,

    /// 修改：如被编辑过的文件。
    pub modified: Hsla,
    pub modified_background: Hsla,
    pub modified_border: Hsla,

    /// 预测：如自动补全或生成代码。
    pub predictive: Hsla,
    pub predictive_background: Hsla,
    pub predictive_border: Hsla,

    /// 重命名：如被重命名的文件。
    pub renamed: Hsla,
    pub renamed_background: Hsla,
    pub renamed_border: Hsla,

    /// 成功：操作成功或任务完成。
    pub success: Hsla,
    pub success_background: Hsla,
    pub success_border: Hsla,

    /// 不可达：如永远不会执行到的代码块。
    pub unreachable: Hsla,
    pub unreachable_background: Hsla,
    pub unreachable_border: Hsla,

    /// 警告：即将失败的操作。
    pub warning: Hsla,
    pub warning_background: Hsla,
    pub warning_border: Hsla,
}

impl StatusColors {
    /// 深色主题默认值（由色阶派生，对齐 zed `StatusColors::dark`）。
    pub fn dark() -> Self {
        use crate::default_colors::{blue, grass, neutral, red, yellow};
        Self {
            conflict: red().dark().step_9(),
            conflict_background: red().dark().step_9(),
            conflict_border: red().dark().step_9(),
            created: grass().dark().step_9(),
            created_background: grass().dark().step_9().opacity(0.25),
            created_border: grass().dark().step_9(),
            deleted: red().dark().step_9(),
            deleted_background: red().dark().step_9().opacity(0.25),
            deleted_border: red().dark().step_9(),
            error: red().dark().step_9(),
            error_background: red().dark().step_9(),
            error_border: red().dark().step_9(),
            hidden: neutral().dark().step_9(),
            hidden_background: neutral().dark().step_9(),
            hidden_border: neutral().dark().step_9(),
            hint: blue().dark().step_9(),
            hint_background: blue().dark().step_9(),
            hint_border: blue().dark().step_9(),
            ignored: neutral().dark().step_9(),
            ignored_background: neutral().dark().step_9(),
            ignored_border: neutral().dark().step_9(),
            info: blue().dark().step_9(),
            info_background: blue().dark().step_9(),
            info_border: blue().dark().step_9(),
            modified: yellow().dark().step_9(),
            modified_background: yellow().dark().step_9().opacity(0.25),
            modified_border: yellow().dark().step_9(),
            predictive: neutral().dark_alpha().step_9(),
            predictive_background: neutral().dark_alpha().step_9(),
            predictive_border: neutral().dark_alpha().step_9(),
            renamed: blue().dark().step_9(),
            renamed_background: blue().dark().step_9().opacity(0.25),
            renamed_border: blue().dark().step_9(),
            success: grass().dark().step_9(),
            success_background: grass().dark().step_9().opacity(0.25),
            success_border: grass().dark().step_9(),
            unreachable: neutral().dark().step_9(),
            unreachable_background: neutral().dark().step_9().opacity(0.25),
            unreachable_border: neutral().dark().step_9(),
            warning: yellow().dark().step_9(),
            warning_background: yellow().dark().step_9().opacity(0.25),
            warning_border: yellow().dark().step_9(),
        }
    }

    /// 浅色主题默认值。
    pub fn light() -> Self {
        use crate::default_colors::{blue, grass, neutral, red, yellow};
        Self {
            conflict: red().light().step_9(),
            conflict_background: red().light().step_9(),
            conflict_border: red().light().step_9(),
            created: grass().light().step_9(),
            created_background: grass().light().step_9().opacity(0.25),
            created_border: grass().light().step_9(),
            deleted: red().light().step_9(),
            deleted_background: red().light().step_9().opacity(0.25),
            deleted_border: red().light().step_9(),
            error: red().light().step_9(),
            error_background: red().light().step_9(),
            error_border: red().light().step_9(),
            hidden: neutral().light().step_9(),
            hidden_background: neutral().light().step_9(),
            hidden_border: neutral().light().step_9(),
            hint: blue().light().step_9(),
            hint_background: blue().light().step_9(),
            hint_border: blue().light().step_9(),
            ignored: neutral().light().step_9(),
            ignored_background: neutral().light().step_9(),
            ignored_border: neutral().light().step_9(),
            info: blue().light().step_9(),
            info_background: blue().light().step_9(),
            info_border: blue().light().step_9(),
            modified: yellow().light().step_9(),
            modified_background: yellow().light().step_9().opacity(0.25),
            modified_border: yellow().light().step_9(),
            predictive: neutral().light_alpha().step_9(),
            predictive_background: neutral().light_alpha().step_9(),
            predictive_border: neutral().light_alpha().step_9(),
            renamed: blue().light().step_9(),
            renamed_background: blue().light().step_9().opacity(0.25),
            renamed_border: blue().light().step_9(),
            success: grass().light().step_9(),
            success_background: grass().light().step_9().opacity(0.25),
            success_border: grass().light().step_9(),
            unreachable: neutral().light().step_9(),
            unreachable_background: neutral().light().step_9().opacity(0.25),
            unreachable_border: neutral().light().step_9(),
            warning: yellow().light().step_9(),
            warning_background: yellow().light().step_9().opacity(0.25),
            warning_border: yellow().light().step_9(),
        }
    }
}

impl Default for StatusColors {
    fn default() -> Self {
        Self::dark()
    }
}
