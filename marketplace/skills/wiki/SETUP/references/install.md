# Detailed install options for llmwiki-cli

## Recommended: `curl install.sh | bash`

```bash
curl -LsSf https://github.com/<owner>/llmwiki/raw/main/install.sh | bash
```

Installs the latest release binary to `~/.local/bin/llmwiki-cli`. Verifies
SHA256 against the published `.sha256` file. Adds to PATH if needed.

## Alternative: `cargo install` (compiles from source, ~3 minutes)

```bash
cargo install llmwiki-cli --locked
```

Requires Rust 1.85+ installed (per `Cargo.toml`'s `rust-version`). Use this when:
- No pre-built binary matches your platform (e.g. unusual Linux distro).
- You need the absolute latest commit (`cargo install --git https://github.com/<owner>/llmwiki`).

## Alternative: `cargo binstall` (downloads pre-built binary, no compilation)

```bash
cargo binstall llmwiki-cli
```

Faster than `cargo install` if a pre-built binary exists for your target.

## Alternative: Homebrew (macOS, after tap is published)

```bash
brew install llmwiki-cli
```

(The tap is not yet published — see GitHub issue #N.)

## Verifying the install

```bash
llmwiki-cli --version          # Should print "llmwiki-cli 0.3.0"
llmwiki-cli doctor             # Should report "OK" or surface specific issues
llmwiki-cli --help             # Should list all subcommands
```

## Uninstalling

```bash
# Binary installed by install.sh:
rm ~/.local/bin/llmwiki-cli

# Binary installed by cargo:
cargo uninstall llmwiki-cli
```

## Troubleshooting

| Error | Cause | Fix |
|---|---|---|
| `command not found: llmwiki-cli` | PATH not updated | Add `~/.local/bin` to PATH or use absolute path |
| `Permission denied` on install | `$HOME/.local/bin` owned by another user | Run with sudo or set `LLMWIKI_INSTALL_DIR=/tmp/mybin` |
| Binary exists but `--version` panics | Corrupted download | Re-run install.sh; SHA256 mismatch will surface |
| `llmwiki-cli doctor` fails on NIM | API key missing or NIM endpoint down | See TROUBLESHOOTING sub-skill |