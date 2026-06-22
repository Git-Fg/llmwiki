---
name: sync
description: |
  Sync a wiki across tailnet devices via git. Use when the user asks
  "set up wiki on a new device", "sync my notes", "pull latest wiki
  changes", or wants to set up a new laptop.
whenToUse: |
  Do NOT use for ingestion or search.
allowed-tools: Bash(wiki:*), Bash(git:*)
---

# Wiki — Sync (new device setup)

## On the new device

1. **Install the CLI**:
   ```bash
   cargo install --path /path/to/wiki   # or `cargo install wiki` once published
   ```

2. **Clone the wiki** (assuming the user has it in a git remote):
   ```bash
   git clone <wiki-remote-url> ~/my-wiki
   ```

3. **Register the wiki in wiki-root.toml**:
   ```bash
   wiki config add mywiki ~/my-wiki --tag personal
   ```
   No `.wiki/` directory needs to exist — the registry is the source of truth.

4. **Set the NIM API key** in your shell rc:
   ```bash
   export NVIDIA_NIM_API_KEY="nvapi-..."
   ```

5. **Install the wiki skill globally**:
   ```bash
   wiki install-skill --global
   ```

6. **Verify**:
   ```bash
   wiki doctor
   ```

## Switching wikis without `cd`

```bash
wiki --wiki pharma ls
wiki --wiki mevin search "..."
```

The `--wiki <alias>` flag works on every command.

## Pull latest changes

```bash
cd ~/my-wiki && git pull
wiki embed   # refresh embeddings for any new pages
```
