use clap::{Args, Subcommand};
use serde::Serialize;

use crate::error::WikiError;
use crate::skills;

#[derive(Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub cmd: Option<SkillCmd>,
}

#[derive(Subcommand)]
pub enum SkillCmd {
    /// Load a sub-skill by topic name (alias of `show`). Mirrors
    /// `agent-browser skills get <name>` — the canonical discovery
    /// primitive for AI-agent CLIs. Pass `--all` to dump every sub-skill.
    Get {
        /// Topic name (e.g. `wiki-search`, `search`, `wiki-config`).
        /// Omit and pass --all to print every sub-skill.
        topic: Option<String>,

        /// Print every sub-skill concatenated with section headers.
        #[arg(long)]
        all: bool,
    },
    /// Print the hub SKILL.md, or a specific sub-skill by topic name.
    Show {
        /// Topic name (e.g. `wiki-search`, `search`, `wiki-config`).
        topic: Option<String>,
    },
    /// List every sub-skill with its line count.
    List {
        /// Output machine-readable JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Print the path of an installed sub-skill (or the hub if no name).
    Path {
        /// Topic name (e.g. `wiki-search`). Defaults to the hub.
        topic: Option<String>,
    },
}

#[derive(Serialize)]
struct SkillEntry {
    name: String,
    lines: usize,
}

pub fn run(args: SkillArgs) -> Result<(), WikiError> {
    match args.cmd {
        None
        | Some(SkillCmd::Show { topic: None })
        | Some(SkillCmd::Get {
            topic: None,
            all: false,
        }) => {
            print!("{}", skills::hub());
        }
        Some(SkillCmd::Show { topic: Some(t) })
        | Some(SkillCmd::Get {
            topic: Some(t),
            all: false,
        }) => match skills::find_skill(&t) {
            Some(content) => print!("{content}"),
            None => {
                eprintln!("Unknown topic '{t}'. Available:");
                for (name, lines) in skills::list_skills() {
                    eprintln!("  {name:<22} ({lines} lines)");
                }
                return Err(WikiError::UnknownSkillTopic(t));
            }
        },
        Some(SkillCmd::Get {
            topic: None,
            all: true,
        }) => {
            let entries = skills::list_skills();
            if entries.is_empty() {
                eprintln!("No sub-skills found.");
                return Ok(());
            }
            for (name, _) in &entries {
                if let Some(content) = skills::find_skill(name) {
                    println!("=== {name} ===");
                    print!("{content}");
                    println!();
                }
            }
        }
        Some(SkillCmd::Get {
            topic: Some(t),
            all: true,
        }) => {
            return Err(WikiError::Other(anyhow::anyhow!(
                "cannot use --all with a topic name; omit '{t}' or drop --all"
            )));
        }
        Some(SkillCmd::List { json }) => {
            if json {
                let entries: Vec<SkillEntry> = skills::list_skills()
                    .into_iter()
                    .map(|(name, lines)| SkillEntry { name, lines })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entries)
                        .map_err(|e| WikiError::Other(anyhow::anyhow!(e)))?
                );
            } else {
                for (name, lines) in skills::list_skills() {
                    println!("{name:<22} ({lines} lines)");
                }
            }
        }
        Some(SkillCmd::Path { topic }) => {
            let home = std::env::var("HOME").unwrap_or_default();
            match topic {
                None => println!("{home}/.agents/skills/wiki/SKILL.md"),
                Some(t) => {
                    let dir = normalize_for_install(&t);
                    println!("{home}/.agents/skills/{dir}/SKILL.md");
                }
            }
        }
    }
    Ok(())
}

/// Mirror of `skills::normalize_topic` but kept local to avoid a public
/// re-export. Accepts `search` → `wiki-search` and passes through `wiki-search`.
fn normalize_for_install(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if lower.starts_with("wiki-") {
        lower
    } else {
        format!("wiki-{lower}")
    }
}
