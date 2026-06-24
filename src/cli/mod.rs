pub mod build;
pub mod config;
pub mod doctor;
pub mod embed;
pub mod ingest;
pub mod init;
pub mod install_skill;
pub mod lint;
pub mod ls;
pub mod models;
pub mod query;
pub mod search;
pub mod skill;
pub mod status;
pub mod tree;

use clap::{Parser, Subcommand};

use crate::cli::skill::SkillArgs;

#[derive(Parser)]
#[command(
    name = "llmwiki-cli",
    version,
    about = "Karpathy-style LLM Wiki",
    long_about = "Manage a personal LLM Wiki: markdown pages + JSONL embeddings, no database.\n\
                  Single binary; no server; works offline against local files.\n\
                  NVIDIA NIM provides embeddings + optional LLM for RAG queries.\n\
                  \n\
                  Quick start:\n  \
                    llmwiki-cli doctor                        # verify config + NIM\n  \
                    llmwiki-cli skill list                    # discover sub-skills\n  \
                    llmwiki-cli <command> --help              # full flag reference\n  \n\
                  For AI agents: start with `llmwiki-cli skill get <topic>`.",
    after_help = "Categories:\n  \
                    setup:        init, install-skill, models\n  \
                    knowledge:    ingest, build, embed, search, query\n  \
                    maintenance:  lint, ls, tree, status, doctor\n  \
                    config:       config (see `llmwiki-cli config --help`)\n  \n\
                  Discovery:\n  \
                    llmwiki-cli skill list                    # every inline sub-skill\n  \
                    llmwiki-cli skill get <topic>             # load one (e.g. wiki-search)\n  \
                    llmwiki-cli <command> --help              # full flag reference\n  \n\
                  Common flags (all commands):\n  \
                    --workspace <path>                        # override workspace\n  \
                    --wiki <alias>                            # select wiki from registry\n  \n\
                  Env:\n  \
                    NVIDIA_NIM_API_KEY     required (get at https://build.nvidia.com/)\n  \
                    NVIDIA_API_KEY         fallback if NVIDIA_NIM_API_KEY is unset\n  \
                    WIKI_NIM_BASE_URL      override default https://integrate.api.nvidia.com\n  \
                    LLMWIKI_CONFIG         path to a single config file (overrides walk-up)\n  \n\
                  Project layout: <workspace>/{wiki/, raw/, index.md, log.md}\n  \
                                 + per-workspace .llmwiki-cli/config.toml\n  \
                                 + gitignored embeddings.jsonl (regenerate via `llmwiki-cli embed`)"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub workspace: Option<std::path::PathBuf>,
    /// Select wiki by alias from wiki-root.toml
    #[arg(long, global = true)]
    pub wiki: Option<String>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Compile pending raw sources into wiki pages
    Build {
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Scaffold a new wiki at <path>
    Init {
        path: std::path::PathBuf,
        /// Wiki alias for wiki-root.toml registration
        #[arg(long)]
        alias: Option<String>,
        /// Force flat layout (pages at workspace root). Default scaffolds
        /// pages at workspace root; use `--subdir` for legacy `wiki/` subdir.
        #[arg(long)]
        flat: bool,
        /// Force legacy subdirectory layout (pages under `wiki/`).
        #[arg(long, conflicts_with = "flat")]
        subdir: bool,
        /// Tags for this wiki (repeatable)
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
    },
    /// List whitelisted NVIDIA NIM Models
    Models {
        #[arg(long)]
        embed: bool,
        #[arg(long)]
        rerank: bool,
        #[arg(long)]
        commercial: bool,
        #[arg(long)]
        json: bool,
    },
    /// Embed wiki markdown pages into embeddings.jsonl
    Embed {
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        dims: Option<usize>,
        #[arg(long)]
        skip_existing: bool,
        #[arg(long)]
        batch_size: Option<usize>,
    },
    /// Search embedded wiki pages by vector similarity
    #[command(
        long_about = "Find existing wiki content by semantic similarity over embedded chunks.\n\
                            \n\
                            Workflow:\n  \
                              1. Embeddings must exist first: llmwiki-cli embed\n  \
                              2. Run: llmwiki-cli search \"your query\"\n  \
                              3. Adjust --top-k (more results) or --threshold (higher = stricter)\n  \n\
                            For RAG question-answering with citations, use `llmwiki-cli query` instead.\n  \n\
                            For browsing files directly, use native file tools — search is for finding\n  \
                            content you don't already know exists."
    )]
    Search {
        query: String,
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        #[arg(long, default_value_t = 0.0)]
        threshold: f32,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Ask a RAG question over the wiki
    #[command(
        long_about = "Ask a question in natural language; get an answer synthesized from\n\
                            the most relevant wiki pages, with citations back to the source pages.\n\
                            \n\
                            Workflow:\n  \
                              1. Embeddings must exist first: llmwiki-cli embed\n  \
                              2. Run: llmwiki-cli query \"your question\"\n  \
                              3. The answer cites the pages it used; pass --no-citations for clean output\n  \n\
                            For raw semantic search (no LLM synthesis), use `llmwiki-cli search` instead.\n  \n\
                            The LLM model defaults to the wiki's configured nim.embed_model; override\n  \
                            with --llm-model. Requires a separate LLM-capable NIM model if your embed\n  \
                            model is not LLM-capable."
    )]
    Query {
        question: String,
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        llm_model: Option<String>,
        #[arg(long)]
        no_citations: bool,
        #[arg(long)]
        json: bool,
    },
    /// Run quality checks over wiki pages, raw sources, and log
    #[command(long_about = "Deterministic hygiene checks over the wiki:\n\
                            - frontmatter validity (required fields, schema_version)\n  \
                            - wikilink resolution (broken links)\n  \
                            - tag consistency\n  \
                            - page tree integrity (orphans, cycles)\n  \n\
                            Pass --strict to fail on warnings (CI-friendly).\n  \n\
                            Run before `git commit` to catch issues that `embed` / `search`\n  \
                            would otherwise surface as opaque failures downstream.")]
    Lint {
        #[arg(long, default_value = "wiki")]
        scope: String,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        json: bool,
    },
    /// List wiki pages, raw sources, embeddings, links, or config
    Ls {
        #[arg(long)]
        pages: bool,
        #[arg(long)]
        raw: bool,
        #[arg(long)]
        embed: bool,
        #[arg(long)]
        links: bool,
        #[arg(long)]
        config: bool,
        #[arg(long)]
        json: bool,
    },
    /// Diagnose workspace, config, and NIM connectivity
    #[command(long_about = "One-shot health check for a wiki workspace:\n\
                            - workspace discovery (registry → env → walk-up → single-wiki)\n  \
                            - config parse + alias resolution\n  \
                            - NIM API key presence (NVIDIA_NIM_API_KEY or NVIDIA_API_KEY)\n  \
                            - NIM endpoint reachability\n  \n\
                            Always run `llmwiki-cli doctor` first when anything is misbehaving.\n  \
                            Pass --json for machine-readable output (CI / scripts).")]
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Flat, grep-friendly listing of wiki pages (slug, path, title, tags, embedded)
    Tree {
        #[arg(long)]
        json: bool,
    },
    /// Report pages, embeddings, and raw-source coverage
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Add a source file to raw/ and append a log entry
    #[command(
        long_about = "Add a single source file (PDF, markdown, text, etc.) to the wiki's\n\
                            raw/ directory, then compile it into a wiki page.\n\
                            \n\
                            Workflow:\n  \
                              1. llmwiki-cli ingest /path/to/source.pdf     # adds to raw/, compiles\n  \
                              2. Page appears in wiki/ on next `llmwiki-cli ls`\n  \
                              3. Run `llmwiki-cli embed` to embed the new page\n  \n\
                            Pass --no-compile to defer compilation (batch-compile later with `llmwiki-cli build`).\n  \n\
                            For batch ingestion of multiple files, run this command in a loop or use\n  \
                            the watch-based pipeline. For bulk historical imports, see the wiki-ingest sub-skill."
    )]
    Ingest {
        source: std::path::PathBuf,
        #[arg(long)]
        no_compile: bool,
        #[arg(long)]
        source_type: Option<String>,
    },
    /// Show or list the embedded wiki agent skill
    Skill(SkillArgs),
    /// Install the wiki skill as a global or workspace-local agent skill
    InstallSkill {
        #[arg(long)]
        global: bool,
        #[arg(long)]
        workspace: Option<std::path::PathBuf>,
    },
    /// Print version
    Version,
    /// Manage wiki-root.toml and per-workspace configuration
    ///
    /// Config resolution priority (highest wins):
    ///   1. `$LLMWIKI_CONFIG` env var (points at a single config file)
    ///   2. `<workspace>/.llmwiki-cli/config.toml` (per-workspace, walks up)
    ///   3. `~/.llmwiki-cli/config.toml` (per-computer fallback)
    ///   4. built-in defaults
    ///
    /// Use `wiki config paths` to see the resolved search order for the
    /// current workspace, and `wiki config show-effective` to see which file
    /// overrode which key.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the resolved wiki-root.toml file path
    Path,
    /// List all wikis or show merged config for a specific wiki
    List {
        /// Show config for this alias
        #[arg(long)]
        wiki: Option<String>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Get a config value by dotted key
    Get {
        /// e.g. nim.embed_model
        key: String,
        /// Wiki alias (defaults to [defaults])
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Set a config value by dotted key
    Set {
        /// e.g. nim.embed_model
        key: String,
        /// e.g. nvidia/nv-embed-v1
        value: String,
        /// Wiki alias (defaults to [defaults])
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Remove a config override (revert to default). Errors if the wiki alias
    /// is from a lower-priority wiki-root.toml (use WIKI_ROOT_CONFIG to target
    /// the file that owns the alias, or edit it directly).
    Unset {
        /// e.g. nim.embed_model
        key: String,
        /// Wiki alias (required for unset)
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Register a new wiki
    Add {
        /// Wiki alias
        alias: String,
        /// Path to wiki directory
        path: std::path::PathBuf,
        /// Tags (repeatable)
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove a wiki from the registry. Errors if the alias is from a
    /// lower-priority wiki-root.toml (use WIKI_ROOT_CONFIG to target the file
    /// that owns the alias, or edit it directly).
    Rm {
        /// Wiki alias to remove
        alias: String,
    },
    /// Open wiki-root.toml in $EDITOR
    Edit,
    /// Validate wiki-root.toml: parse [defaults] + every [alias], run field-level checks
    Validate,
    /// Print the JSON Schema for the resolved Config type (for editor / LSP use)
    ShowSchema {
        /// Filter output to one section: `wiki` or `nim`.
        #[arg(long, value_parser = ["wiki", "nim"])]
        section: Option<String>,
    },
    /// Print the resolved config search order with each path's existence status.
    /// Useful for debugging why a particular config.toml is or isn't being loaded.
    /// Pass --workspace <path> to override the walk-up start; otherwise the
    /// resolved workspace is used (registry → env → walk-up → single-wiki).
    Paths {
        /// Override the workspace used as the walk-up start. `from_global`
        /// receives the value of the top-level `--workspace` flag (declared
        /// `global = true` on `Cli`) so `wiki --workspace <ws> config paths`
        /// works without a subcommand-level flag.
        #[arg(from_global)]
        workspace: Option<std::path::PathBuf>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Open the highest-priority existing config file in $EDITOR (per-workspace
    /// first, then per-computer, then $LLMWIKI_CONFIG). If no file exists yet,
    /// opens the per-workspace candidate so you can create one.
    ConfigEdit {
        /// Override the workspace used as the walk-up start. `from_global`
        /// receives the value of the top-level `--workspace` flag (declared
        /// `global = true` on `Cli`) so `wiki --workspace <ws> config
        /// config-edit` works without a subcommand-level flag.
        #[arg(from_global)]
        workspace: Option<std::path::PathBuf>,
    },
    /// Print every effective config key, its merged value, and the file it
    /// came from. Mirrors `git config --list --show-origin` so users can see
    /// which file overrides which key. Use `[<prefix>]` to filter to keys
    /// starting with that prefix, `--source <path>` to filter to keys
    /// set in a specific config file, or `--overrides-only` to hide keys
    /// whose value equals the built-in default (the most useful subset for
    /// auditing "what did my config actually change?").
    ShowEffective {
        /// Override the workspace used as the walk-up start. `from_global`
        /// receives the value of the top-level `--workspace` flag so
        /// `wiki --workspace <ws> config show-effective` works.
        #[arg(from_global)]
        workspace: Option<std::path::PathBuf>,
        /// JSON output
        #[arg(long)]
        json: bool,
        /// Only show keys starting with this prefix (e.g. `nim.`, `wiki.`).
        /// Mirrors the positional pattern syntax of `git config --list -- <pattern>`.
        #[arg(value_name = "PREFIX")]
        key_prefix: Option<String>,
        /// Only show keys whose source file matches this path. Useful for
        /// "what did this specific file set?" audits.
        #[arg(long, value_name = "PATH")]
        source: Option<std::path::PathBuf>,
        /// Only show keys whose value differs from the built-in default.
        /// Surfaces the keys your config files actually changed instead of
        /// dumping every key (most of which match defaults).
        #[arg(long)]
        overrides_only: bool,
    },
}

pub async fn run(cli: Cli) {
    let result: Result<(), crate::error::WikiError> = match cli.command {
        Some(Command::Build { since, dry_run }) => {
            crate::cli::build::run(crate::cli::build::BuildArgs {
                workspace: cli.workspace,
                wiki: cli.wiki,
                since,
                dry_run,
            })
        }
        Some(Command::Init {
            path,
            alias,
            flat,
            subdir,
            tags,
        }) => crate::cli::init::run(crate::cli::init::InitArgs {
            path,
            alias,
            flat,
            subdir,
            tags,
        }),
        Some(Command::Models {
            embed,
            rerank,
            commercial,
            json,
        }) => crate::cli::models::run(embed, rerank, commercial, json),
        Some(Command::Embed {
            model,
            dims,
            skip_existing,
            batch_size,
        }) => {
            crate::cli::embed::run(crate::cli::embed::EmbedArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                model,
                dims,
                skip_existing,
                batch_size,
            })
            .await
        }
        Some(Command::Search {
            query,
            top_k,
            threshold,
            model,
            json,
        }) => {
            crate::cli::search::run(crate::cli::search::SearchArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                query,
                top_k,
                threshold,
                model,
                json,
            })
            .await
        }
        Some(Command::Query {
            question,
            top_k,
            model,
            llm_model,
            no_citations,
            json,
        }) => {
            crate::cli::query::run(crate::cli::query::QueryArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                question,
                top_k,
                model,
                llm_model,
                no_citations,
                json,
            })
            .await
        }
        Some(Command::Doctor { json }) => {
            crate::cli::doctor::run(crate::cli::doctor::DoctorArgs {
                workspace: cli.workspace,
                wiki: cli.wiki,
                json,
            })
            .await
        }
        Some(Command::Status { json }) => crate::cli::status::run(crate::cli::status::StatusArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            json,
        }),
        Some(Command::Skill(args)) => crate::cli::skill::run(args),
        Some(Command::InstallSkill { global, workspace }) => {
            crate::cli::install_skill::run(crate::cli::install_skill::InstallSkillArgs {
                global,
                workspace,
            })
        }
        Some(Command::Ingest {
            source,
            no_compile,
            source_type,
        }) => crate::cli::ingest::run(crate::cli::ingest::IngestArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            source,
            no_compile,
            source_type,
        }),
        Some(Command::Lint {
            scope,
            strict,
            json,
        }) => {
            crate::cli::lint::run(crate::cli::lint::LintArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                scope,
                strict,
                json,
            })
            .await
        }
        Some(Command::Ls {
            pages,
            raw,
            embed,
            links,
            config,
            json,
        }) => crate::cli::ls::run(crate::cli::ls::LsArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            pages,
            raw,
            embed,
            links,
            config,
            json,
        }),
        Some(Command::Tree { json }) => crate::cli::tree::run(crate::cli::tree::TreeArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            json,
        }),
        Some(Command::Config { cmd }) => crate::cli::config::run(cmd).await,
        Some(Command::Version) | None => {
            println!("llmwiki-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
