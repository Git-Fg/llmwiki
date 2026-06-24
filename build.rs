use std::fs;
use std::path::Path;

// Include the canonical type definitions and their `default_*` helpers.
// These files are also included by `src/core/config.rs` and
// `src/cli/doctor.rs` for the runtime types. This is the single source
// of truth for both runtime serialization and schema generation —
// adding a field to Config or DoctorReport automatically updates the
// generated JSON Schema files. The default_* helpers MUST live in the
// included type file (not in build.rs separately) because schemars 1.0
// resolves them at macro-expansion time AND calls them at schema-gen
// time to populate the JSON Schema's `default` keyword entries.
include!("src/core/config_types.rs");
include!("src/cli/doctor_report.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/core/config.rs");
    println!("cargo:rerun-if-changed=src/core/config_types.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor_report.rs");
    // Note: `rust-embed` (in `src/skills/mod.rs`) emits its own
    // `cargo:rerun-if-changed=` lines for every file in the `skills/`
    // folder, so we don't need to list them here.

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);

    // v0.3.29: drop the marketplace mirror. The skill bundle lives at
    // `skills/` and is embedded by `rust-embed` in `src/skills/mod.rs`
    // directly from there. `agents/skills/` was the legacy marketplace
    // stub directory; no longer needed.

    // Generate the JSON Schema for the Config type and ship it under
    // `skills/references/schema.json` so agents can `cat` it directly
    // (or load via `llmwiki-cli config show-schema`).
    let schema_path = manifest_path.join("skills/references/schema.json");
    if let Some(parent) = schema_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            println!("cargo:warning=failed to create schema dir {parent:?}: {e}");
        }
    }
    let schema = schemars::schema_for!(Config);
    let schema_json = serde_json::to_string_pretty(&schema).expect("schema is always serializable");
    if let Err(e) = fs::write(&schema_path, format!("{schema_json}\n")) {
        println!("cargo:warning=failed to write schema.json {schema_path:?}: {e}");
    }
}
