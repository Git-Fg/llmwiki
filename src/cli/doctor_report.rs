/// JSON Schema for the output of `llmwiki-cli doctor --json`. Auto-generated
/// from this struct (included via `include!` from `src/cli/doctor.rs`
/// and from `build.rs`). This file is the single source of truth —
/// do NOT duplicate this struct in `build.rs`.
///
/// **Breaking v0.3.17**: `active_alias` is `string | null` (was `""`
/// sentinel pre-v0.3.17). **v0.3.23**: `nim_status` bounded to HTTP
/// range [100, 599]. **v0.3.22**: `config` / `config_sources` are
/// reflective dotted-key maps.
///
/// Type imports are intentionally written as full paths on the derive
/// attributes (see `src/core/config_types.rs` for rationale).
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)] // → schema root `additionalProperties: false`
pub struct DoctorReport {
    /// Resolved workspace directory (canonical absolute path).
    pub workspace: String,
    /// Active wiki-root.toml alias for this workspace, if any is registered. `None`
    /// means no alias resolves to this workspace. v0.3.17+: was `""` sentinel pre-v0.3.17.
    pub active_alias: Option<String>,
    /// Path to the resolved `wiki-root.toml`, or empty string if no registry entry matched.
    pub wiki_root_path: String,
    /// Count of entries in the discovered wiki-root.toml registry (0 if no registry).
    pub registry_entries: usize,
    /// Whether the per-workspace / per-computer config was loaded successfully.
    pub config_loaded: bool,
    /// Resolved NIM embedding model name from the merged effective config.
    pub embed_model: String,
    /// Resolved NIM base URL (host only, no `/v1` segment).
    pub nim_base_url: String,
    /// Resolved NIM API key length in bytes. 0 if the key env var is unset/empty.
    pub api_key_length: usize,
    /// Name of the env var that holds the NIM API key.
    pub api_key_env: String,
    /// Whether `GET {nim_base_url}/v1/models` returned 2xx during this run.
    pub nim_reachable: bool,
    /// HTTP status from the NIM probe, or `None` on network error. v0.3.23+:
    /// bounded to HTTP range [100, 599] (was the u16 range [0, 65535] before).
    #[schemars(range(min = 100, max = 599))]
    pub nim_status: Option<u16>,
    /// Human-readable error string from the NIM probe, or `None` on success.
    pub nim_error: Option<String>,
    /// Whether the resolved `embed_model` appears in the live NIM `/v1/models` response.
    pub embed_model_available: bool,
    /// Reflective dump of the resolved effective config as dotted-key → value pairs.
    #[schemars(
        description = "dotted-key to value map (same shape as `llmwiki-cli config show-effective`)"
    )]
    pub config: std::collections::BTreeMap<String, String>,
    /// Per-key source attribution: dotted-key → file-it-came-from (`<default>` for built-in defaults).
    #[schemars(description = "dotted-key to source file path map (v0.3.12+)")]
    pub config_sources: std::collections::BTreeMap<String, String>,
}