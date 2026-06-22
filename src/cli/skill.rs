use clap::{Args, Subcommand};

use crate::error::WikiError;
use crate::skills;

#[derive(Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub cmd: Option<SkillCmd>,
}

#[derive(Subcommand)]
pub enum SkillCmd {
    /// Print the full SKILL.md content or a specific topic
    Show { topic: Option<String> },
    /// List available skill topics
    List,
    /// Print installed skill stub path
    Path,
}

pub fn run(args: SkillArgs) -> Result<(), WikiError> {
    match args.cmd {
        None | Some(SkillCmd::Show { topic: None }) => {
            print!("{}", skills::SKILL_MD);
        }
        Some(SkillCmd::Show { topic: Some(t) }) => match skills::find_topic(&t) {
            Some(content) => print!("{}", content),
            None => {
                eprintln!("Unknown topic '{}'. Available:", t);
                for (name, lines) in skills::list_topics() {
                    eprintln!("  {:<20} ({} lines)", name, lines);
                }
                return Err(WikiError::UnknownSkillTopic(t));
            }
        },
        Some(SkillCmd::List) => {
            for (name, lines) in skills::list_topics() {
                println!("{:<20} ({} lines)", name, lines);
            }
        }
        Some(SkillCmd::Path) => {
            let home = std::env::var("HOME").unwrap_or_default();
            println!("{}/.agents/skills/wiki/SKILL.md", home);
        }
    }
    Ok(())
}
