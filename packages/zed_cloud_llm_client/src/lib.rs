#[cfg(feature = "predict-edits")]
pub mod predict_edits_v3;
#[cfg(feature = "predict-edits")]
pub mod predict_edits_v4;

use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

/// The name of the header used to indicate which version of Zed the client is running.
pub const ZED_VERSION_HEADER_NAME: &str = "x-zed-version";

/// The name of the header used to indicate which edit prediction experiment should be used.
pub const PREFERRED_EXPERIMENT_HEADER_NAME: &str = "x-zed-preferred-experiment";

/// The name of the header used to indicate when a request failed due to an
/// expired LLM token.
///
/// The client may use this as a signal to refresh the token.
pub const EXPIRED_LLM_TOKEN_HEADER_NAME: &str = "x-zed-expired-token";

/// The name of the header used to indicate when a request failed due to an outdated LLM token.
///
/// A token is considered "outdated" when we can't parse the claims (e.g., after adding a new required claim).
///
/// This is distinct from [`EXPIRED_LLM_TOKEN_HEADER_NAME`] which indicates the token's time-based validity has passed.
/// An outdated token means the token's structure is incompatible with the current server expectations.
pub const OUTDATED_LLM_TOKEN_HEADER_NAME: &str = "x-zed-outdated-token";

/// The name of the header used to indicate the usage limit for edit predictions.
pub const EDIT_PREDICTIONS_USAGE_LIMIT_HEADER_NAME: &str = "x-zed-edit-predictions-usage-limit";

/// The name of the header used to indicate the usage amount for edit predictions.
pub const EDIT_PREDICTIONS_USAGE_AMOUNT_HEADER_NAME: &str = "x-zed-edit-predictions-usage-amount";

/// The name of the header used to indicate the minimum required Zed version.
///
/// This can be used to force a Zed upgrade in order to continue communicating
/// with the LLM service.
pub const MINIMUM_REQUIRED_VERSION_HEADER_NAME: &str = "x-zed-minimum-required-version";

/// The name of the header used by the client to indicate to the server that it supports receiving status messages.
pub const CLIENT_SUPPORTS_STATUS_MESSAGES_HEADER_NAME: &str =
    "x-zed-client-supports-status-messages";

/// The name of the header used by the client to indicate to the server that it supports receiving a "stream_ended" request completion status.
pub const CLIENT_SUPPORTS_STATUS_STREAM_ENDED_HEADER_NAME: &str =
    "x-zed-client-supports-stream-ended-request-completion-status";

/// The name of the header used by the server to indicate to the client that it supports sending status messages.
pub const SERVER_SUPPORTS_STATUS_MESSAGES_HEADER_NAME: &str =
    "x-zed-server-supports-status-messages";

/// The name of the header used by the client to indicate that it supports receiving xAI models.
pub const CLIENT_SUPPORTS_X_AI_HEADER_NAME: &str = "x-zed-client-supports-x-ai";
