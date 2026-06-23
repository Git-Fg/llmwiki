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
    println!("cargo:rerun-if-changed=marketplace/skills/wiki/SKILL.md");
    println!("cargo:rerun-if-changed=marketplace/skills/wiki/SETUP/SKILL.md");
    println!("cargo:rerun-if-changed=marketplace/skills/wiki/LSP/SKILL.md");
    println!("cargo:rerun-if-changed=marketplace/skills/wiki/MCP/SKILL.md");
    println!("cargo:rerun-if-changed=src/core/config.rs");
    println!("cargo:rerun-if-changed=src/core/config_types.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor_report.rs");

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);

    // Generate the hub SKILL.md stub from marketplace/skills/wiki/SKILL.md
    let hub_src = manifest_path.join("marketplace/skills/wiki/SKILL.md");
    if let Ok(content) = fs::read_to_string(&hub_src) {
        let out_path = manifest_path.join("agents/skills/wiki/SKILL.md");
        if let Some(parent) = out_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!("cargo:warning=failed to create skill dir {parent:?}: {e}");
            }
        }
        if let Err(e) = fs::write(&out_path, content) {
            println!("cargo:warning=failed to write hub SKILL.md {out_path:?}: {e}");
        }
    }

    // Write the JSON Schema for the Config type to SETUP/references/schema.json
    // so it ships with the skill bundle and can be referenced by agents.
    let schema_path = manifest_path.join("marketplace/skills/wiki/SETUP/references/schema.json");
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

    // Write the JSON Schema for the DoctorReport type (output of
    // `wiki doctor --json`) to MCP/references/doctor.schema.json. The
    // schema is auto-generated from the `DoctorReport` struct below
    // (kept in sync with `src/cli/doctor.rs::DoctorReport` — when
    // either changes, the build will regenerate this file).
    let doctor_schema_path =
        manifest_path.join("marketplace/skills/wiki/MCP/references/doctor.schema.json");
    if let Some(parent) = doctor_schema_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            println!("cargo:warning=failed to create doctor schema dir {parent:?}: {e}");
        }
    }
    let doctor_schema = schemars::schema_for!(DoctorReport);
    // schemars reads `#[serde(deny_unknown_fields)]` on DoctorReport and
    // emits `"additionalProperties": false` at the root natively, so no
    // post-processing is needed.
    let doctor_schema_json =
        serde_json::to_string_pretty(&doctor_schema).expect("doctor schema is always serializable");
    // POSIX-text-file convention: end with a trailing newline. Many
    // editors (Vim with `:set fixendofile`, GitHub's web diff view)
    // expect one. v0.3.23 polish.
    if let Err(e) = fs::write(&doctor_schema_path, format!("{doctor_schema_json}\n")) {
        println!("cargo:warning=failed to write doctor.schema.json {doctor_schema_path:?}: {e}");
    }
}
