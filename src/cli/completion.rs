//! Shell completion script generation for `llmwiki-cli`.
//!
//! Outputs a static completion script to stdout that the user installs
//! once per shell. We do NOT support dynamic completion (which would
//! query the registry on every TAB) because the alias list lives in
//! `wiki-root.toml` and the script can be much smaller + faster by
//! just listing `--wiki` as a free-form argument with a hint that
//! `wiki config list` shows the available values.
//!
//! Supported shells: bash, zsh, fish, elvish, powershell.
//!
//! Install:
//!   wiki completion bash  > ~/.local/share/bash-completion/completions/llmwiki-cli
//!   wiki completion zsh   > "${fpath[1]}/_llmwiki-cli"
//!   wiki completion fish  > ~/.config/fish/completions/llmwiki-cli.fish
//!   wiki completion powershell > $HOME\Documents\PowerShell\Completions\llmwiki-cli.ps1
use crate::error::WikiError;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

#[derive(clap::Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completions for: bash, zsh, fish, elvish, powershell
    #[arg(value_enum)]
    pub shell: ShellArg,
}

/// Thin wrapper around `clap_complete::Shell` so the value-parser rejects
/// unknown shells with a clear error and lists the supported set.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum ShellArg {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
}

impl From<ShellArg> for Shell {
    fn from(s: ShellArg) -> Shell {
        match s {
            ShellArg::Bash => Shell::Bash,
            ShellArg::Zsh => Shell::Zsh,
            ShellArg::Fish => Shell::Fish,
            ShellArg::Elvish => Shell::Elvish,
            ShellArg::PowerShell => Shell::PowerShell,
        }
    }
}

pub fn run(args: CompletionArgs) -> Result<(), WikiError> {
    let mut cmd = crate::cli::Cli::command();
    let bin_name = "llmwiki-cli";
    let shell: Shell = args.shell.into();
    generate(shell, &mut cmd, bin_name, &mut io::stdout());
    Ok(())
}
