use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/skills/WIKI.md");

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);

    // Generate the hub SKILL.md stub from src/skills/WIKI.md
    let hub_src = manifest_path.join("src/skills/WIKI.md");
    if let Ok(content) = fs::read_to_string(&hub_src) {
        let out_path = manifest_path.join("agents/skills/wiki/SKILL.md");
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&out_path, content).ok();
    }
}
