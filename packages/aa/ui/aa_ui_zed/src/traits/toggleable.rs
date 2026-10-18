/// A trait for elements that can be toggled.
///
/// Implement this for elements that are visually distinct
/// when in two opposing states, like checkboxes or switches.
///
/// 对齐 zed `crates/ui/src/traits/toggleable.rs`。
pub trait Toggleable {
    /// Sets whether the element is selected.
    fn toggle_state(self, selected: bool) -> Self;
}

/// Represents the selection status of an element.
///
/// 对齐 zed 同名枚举（含三态：未选 / 不确定 / 已选，供复选列表用）。
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ToggleState {
    /// The element is not selected.
    #[default]
    Unselected,
    /// The selection state of the element is indeterminate.
    Indeterminate,
    /// The element is selected.
    Selected,
}

impl ToggleState {
    /// Returns the inverse of the current selection status.
    ///
    /// Indeterminate states become selected if inverted.
    pub fn inverse(&self) -> Self {
        match self {
            Self::Unselected | Self::Indeterminate => Self::Selected,
            Self::Selected => Self::Unselected,
        }
    }

    /// Creates a `ToggleState` from the given `any_checked` and `all_checked` flags.
    pub fn from_any_and_all(any_checked: bool, all_checked: bool) -> Self {
        match (any_checked, all_checked) {
            (true, true) => Self::Selected,
            (false, false) => Self::Unselected,
            _ => Self::Indeterminate,
        }
    }

    /// Returns whether this toggle state is selected
    pub fn selected(&self) -> bool {
        match self {
            ToggleState::Indeterminate | ToggleState::Unselected => false,
            ToggleState::Selected => true,
        }
    }
}

impl From<bool> for ToggleState {
    fn from(selected: bool) -> Self {
        if selected {
            Self::Selected
        } else {
            Self::Unselected
        }
    }
}

impl From<Option<bool>> for ToggleState {
    fn from(selected: Option<bool>) -> Self {
        match selected {
            Some(true) => Self::Selected,
            Some(false) => Self::Unselected,
            None => Self::Indeterminate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_flips_states() {
        assert_eq!(ToggleState::Selected.inverse(), ToggleState::Unselected);
        assert_eq!(ToggleState::Unselected.inverse(), ToggleState::Selected);
        // 不确定态反转后变为已选（zed 语义）
        assert_eq!(ToggleState::Indeterminate.inverse(), ToggleState::Selected);
    }

    #[test]
    fn from_any_and_all_covers_three_states() {
        assert_eq!(ToggleState::from_any_and_all(true, true), ToggleState::Selected);
        assert_eq!(
            ToggleState::from_any_and_all(false, false),
            ToggleState::Unselected
        );
        assert_eq!(
            ToggleState::from_any_and_all(true, false),
            ToggleState::Indeterminate
        );
    }

    #[test]
    fn selected_is_only_true_for_selected() {
        assert!(ToggleState::Selected.selected());
        assert!(!ToggleState::Unselected.selected());
        assert!(!ToggleState::Indeterminate.selected());
    }

    #[test]
    fn from_bool_and_option() {
        assert_eq!(ToggleState::from(true), ToggleState::Selected);
        assert_eq!(ToggleState::from(false), ToggleState::Unselected);
        assert_eq!(ToggleState::from(Some(true)), ToggleState::Selected);
        assert_eq!(ToggleState::from(Some(false)), ToggleState::Unselected);
        assert_eq!(
            ToggleState::from(None),
            ToggleState::Indeterminate,
            "None 表示不确定态"
        );
    }
}
