# Detailed install options for llmwiki-cli

## Recommended (POSIX): `curl install.sh | sh`

Linux, macOS, and Windows-with-Git-for-Windows (Git Bash / MSYS2 / WSL):

```bash
curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | sh
```

Installs the latest release binary to `~/.local/bin/llmwiki-cli`. Verifies
SHA256 against the published `.sha256` file. Adds to PATH if needed.

## Recommended (Windows native): `irm install.ps1 | iex`

Windows PowerShell 7+ (cross-platform `pwsh`, also runs on Linux/macOS):

```powershell
irm https://github.com/Git-Fg/llmwiki/releases/latest/download/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\llmwiki-cli\bin\llmwiki-cli.exe`. Verifies
SHA256 against the published `.sha256` file. Uses `Expand-Archive` to
extract the Windows `.zip` asset natively — no `tar` required.

If you only have Windows PowerShell 5.1 (the default on older Windows
builds), install PowerShell 7+ first via `winget install Microsoft.PowerShell`,
then run the `irm | iex` one-liner above.

## Alternative: `cargo install` (compiles from source, ~3 minutes)

```bash
cargo install llmwiki-cli --locked
```

Requires Rust 1.85+ installed (per `Cargo.toml`'s `rust-version`). Use this when:
- No pre-built binary matches your platform (e.g. unusual Linux distro).
- You need the absolute latest commit (`cargo install --git https://github.com/Git-Fg/llmwiki`).

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
llmwiki-cli --version          # Should print "llmwiki-cli 0.3.7"
llmwiki-cli doctor             # Should report "OK" or surface specific issues
llmwiki-cli --help             # Should list all subcommands
```

On Windows native, use `llmwiki-cli.exe` (the `.exe` suffix is automatic if
PowerShell resolves PATHEXT, but you can be explicit when scripting).

## Uninstalling

```bash
# POSIX binary installed by install.sh:
rm ~/.local/bin/llmwiki-cli

# Windows binary installed by install.ps1:
Remove-Item "$env:LOCALAPPDATA\llmwiki-cli\bin\llmwiki-cli.exe"

# Binary installed by cargo:
cargo uninstall llmwiki-cli
```

## Troubleshooting

| Error | Cause | Fix |
|---|---|---|
| `command not found: llmwiki-cli` | PATH not updated | Add `~/.local/bin` to PATH or use absolute path |
| `Permission denied` on install | `$HOME/.local/bin` owned by another user | Run with sudo or set `LLMWIKI_INSTALL_DIR=/tmp/mybin` |
| Binary exists but `--version` panics | Corrupted download | Re-run the installer; SHA256 mismatch will surface |
| `llmwiki-cli doctor` fails on NIM | API key missing or NIM endpoint down | See TROUBLESHOOTING sub-skill |