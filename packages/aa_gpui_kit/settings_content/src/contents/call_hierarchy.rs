use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

use crate::ui::ModalWidthContent;

#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, JsonSchema, MergeFrom, Debug, PartialEq)]
pub struct CallHierarchySettingsContent {
    /// Determines how much space the call hierarchy picker can take up in relation to the available window width.
    ///
    /// Default: medium
    pub modal_max_width: Option<ModalWidthContent>,
}
