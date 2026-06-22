# Setup

Install the wiki CLI: clone the repo, then `cargo install --path .` (or run from source with `cargo run --`).

Install the skill globally: `wiki install-skill --global`. This symlinks the skill stub to `~/.agents/skills/wiki/`.

Create your wiki: `wiki init ~/my-wiki`. This scaffolds `wiki/`, `raw/`, `index.md`, `log.md`, `.wiki/config.yaml`, and initializes a git repo.

Set `NVIDIA_NIM_API_KEY` in your shell rc:

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."
```

Run `wiki doctor` to verify the install.

First-time setup on a new device: clone your wiki repo (`git clone ... ~/my-wiki`), then run `wiki embed --skip-existing` to compute embeddings locally.