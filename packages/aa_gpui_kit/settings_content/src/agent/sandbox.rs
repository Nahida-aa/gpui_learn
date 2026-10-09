use schemars::{JsonSchema, json_schema};
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::ExtendingVec;

/// A persisted sandbox writable-path grant. Deserializes from either a bare
/// path string (`"/tmp/x"`, a legacy/hand-authored entry with no resolved
/// target) or an object (`{ "requested": "/tmp/x", "resolved": "/tmp/real" }`).
/// Serializes back as a bare string when `resolved` is `None` and as an object
/// otherwise, so hand-authored bare strings round-trip and Zed-written grants
/// are objects.
#[derive(Clone, Debug, Default, PartialEq, MergeFrom)]
pub struct GrantedWritePathContent {
    /// The path exactly as the user/model requested it.
    pub requested: PathBuf,
    /// The canonical, symlink-resolved target established when the grant was
    /// approved. Absent for a bare-string entry.
    pub resolved: Option<PathBuf>,
    /// Windows/WSL only: whether the canonical target lives on a Windows-hosted
    /// (DrvFs) filesystem, whose sandbox-integrity guarantees are weaker. Absent
    /// (false) on other platforms and for bare-string entries.
    pub on_windows_fs: bool,
}

impl GrantedWritePathContent {
    /// The path used for lexical subtree/coverage/dedup logic: the resolved
    /// canonical target when known, otherwise the requested path.
    pub fn canonical_or_requested(&self) -> &Path {
        self.resolved.as_deref().unwrap_or(&self.requested)
    }
}

impl Serialize for GrantedWritePathContent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.resolved {
            None => self.requested.serialize(serializer),
            Some(resolved) => {
                use serde::ser::SerializeStruct as _;
                let field_count = if self.on_windows_fs { 3 } else { 2 };
                let mut state =
                    serializer.serialize_struct("GrantedWritePathContent", field_count)?;
                state.serialize_field("requested", &self.requested)?;
                state.serialize_field("resolved", resolved)?;
                if self.on_windows_fs {
                    state.serialize_field("on_windows_fs", &self.on_windows_fs)?;
                }
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for GrantedWritePathContent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Object {
            requested: PathBuf,
            #[serde(default)]
            resolved: Option<PathBuf>,
            #[serde(default)]
            on_windows_fs: bool,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrObject {
            String(PathBuf),
            Object(Object),
        }

        Ok(match StringOrObject::deserialize(deserializer)? {
            StringOrObject::String(requested) => Self {
                requested,
                resolved: None,
                on_windows_fs: false,
            },
            StringOrObject::Object(Object {
                requested,
                resolved,
                on_windows_fs,
            }) => Self {
                requested,
                resolved,
                on_windows_fs,
            },
        })
    }
}

impl JsonSchema for GrantedWritePathContent {
    fn schema_name() -> Cow<'static, str> { "GrantedWritePathContent".into() }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        json_schema!({
            "oneOf": [
                { "type": "string" },
                {
                    "type": "object",
                        "properties": {
                        "requested": { "type": "string" },
                        "resolved": { "type": ["string", "null"] },
                        "on_windows_fs": { "type": "boolean" }
                    },
                    "required": ["requested"]
                }
            ]
        })
    }
}

#[with_fallible_options]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct SandboxPermissionsContent {
    /// Whether sandboxed terminal commands may always reach any host over the
    /// network without prompting.
    /// Default: false
    pub allow_all_hosts: Option<bool>,

    /// Hosts that sandboxed terminal commands may always reach over the
    /// network without prompting. Each entry is an exact hostname
    /// (`github.com`) or a leading-`*.` subdomain wildcard (`*.npmjs.org`).
    /// Default: []
    pub network_hosts: Option<ExtendingVec<String>>,

    /// Whether sandboxed terminal commands may always write anywhere on the
    /// filesystem without prompting.
    /// Default: false
    pub allow_fs_write_all: Option<bool>,

    /// Whether to persistently run agent terminal commands outside the OS
    /// sandbox. This is the model-facing "off switch": when true, the sandboxed
    /// terminal tool is not exposed and the system prompt omits the sandbox
    /// section, so the model uses the plain `terminal` tool. On Windows, WSL
    /// sandbox setup is skipped. Distinct from the model-requested
    /// `unsandboxed: true` escape approved "once" or "for this thread".
    /// Default: false
    pub allow_unsandboxed: Option<bool>,

    /// Directory subtrees that sandboxed terminal commands may always write
    /// to without prompting. Each entry is either a bare path string or an
    /// object `{requested, resolved}`; Zed writes objects (the canonical,
    /// symlink-resolved target established at approval time), while
    /// hand-authored entries may be bare path strings. Paths written by Zed
    /// are absolute.
    /// Default: []
    pub write_paths: Option<ExtendingVec<GrantedWritePathContent>>,

    /// Whether to warn when a sandbox escalation prompt requests a domain or
    /// write path that contains potentially confusable Unicode characters
    /// (homoglyphs, invisible characters, or bidirectional overrides). When
    /// enabled, such prompts show a warning that must be acknowledged before
    /// the request can be allowed.
    /// Default: true
    pub warn_confusable_unicode: Option<bool>,

    /// Whether to warn (Windows/WSL only) when a sandbox grant targets a file on
    /// a Windows-hosted (DrvFs) filesystem. Such grants are enforced inside WSL
    /// via a translated path, and their sandbox-integrity guarantees are weaker
    /// than a distro-native filesystem. When enabled, such grants show a warning
    /// that must be acknowledged before the command runs.
    /// Default: true
    pub warn_ntfs_grants: Option<bool>,
}
