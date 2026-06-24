// Include the canonical type definitions and their `default_*` helpers.
// These files are also included by `src/core/config.rs` and
// `src/cli/doctor.rs` for the runtime types. This is the single source
// of truth for both runtime serialization and schema generation —
// adding a field to Config, DoctorReport, or Frontmatter automatically
// updates the generated JSON Schema. The default_* helpers MUST live
// in the included type file (not in build.rs separately) because
// schemars 1.0 resolves them at macro-expansion time AND calls them at
// schema-gen time to populate the JSON Schema's `default` keyword entries.
include!("src/core/config_types.rs");
include!("src/cli/doctor_report.rs");
include!("src/core/frontmatter.rs");

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/core/config.rs");
    println!("cargo:rerun-if-changed=src/core/config_types.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor_report.rs");
    println!("cargo:rerun-if-changed=src/core/frontmatter.rs");
    // `rust-embed` (in `src/skills/mod.rs`) emits its own
    // `cargo:rerun-if-changed=` lines for every file in
    // `src/skills/data/`, so we don't need to list them here.

    emit_json_schema(
        "skills/references/frontmatter.schema.json",
        schemars::schema_for!(Frontmatter),
    );
}

fn emit_json_schema(path: &str, schema: schemars::Schema) {
    let json = serde_json::to_string_pretty(&schema).expect("serialize schema");
    let dest = Path::new(path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).expect("create schema parent dir");
    }
    std::fs::write(dest, format!("{json}\n")).expect("write schema");
}
