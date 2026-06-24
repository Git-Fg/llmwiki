// Include the canonical type definitions and their `default_*` helpers.
// These files are also included by `src/core/config.rs` and
// `src/cli/doctor.rs` for the runtime types. This is the single source
// of truth for both runtime serialization and schema generation —
// adding a field to Config or DoctorReport automatically updates the
// generated JSON Schema. The default_* helpers MUST live in the included
// type file (not in build.rs separately) because schemars 1.0 resolves
// them at macro-expansion time AND calls them at schema-gen time to
// populate the JSON Schema's `default` keyword entries.
include!("src/core/config_types.rs");
include!("src/cli/doctor_report.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/core/config.rs");
    println!("cargo:rerun-if-changed=src/core/config_types.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor.rs");
    println!("cargo:rerun-if-changed=src/cli/doctor_report.rs");
    // `rust-embed` (in `src/skills/mod.rs`) emits its own
    // `cargo:rerun-if-changed=` lines for every file in
    // `src/skills/data/`, so we don't need to list them here.
}
