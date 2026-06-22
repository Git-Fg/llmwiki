# Sync Across Devices

The wiki folder is a git repo. Markdown diffs are the only thing on the wire. Embeddings regenerate per device.

Daily workflow on any device:

```bash
cd ~/my-wiki
git pull                                       # get latest markdown
wiki search "..."                              # auto-embed-if-needed, then search
# OR:
wiki embed --skip-existing                     # explicit re-embed
# edit / add pages (via agent or any markdown editor)
git add . && git commit -m "ingest: X" && git push
```

Setup on a new device:
1. Install `wiki` binary
2. `git clone <your-wiki-repo> ~/my-wiki`
3. `wiki install-skill --global`
4. Set `NVIDIA_NIM_API_KEY`
5. `wiki embed --skip-existing` (computes embeddings locally)
6. `wiki doctor` to verify

Conflict resolution:
- Markdown: git's normal merge (rare)
- log.md: append-only; keep both blocks if conflict, then lint
- raw/: don't edit raw files; create new versions instead
- embeddings.jsonl: always gitignored, no conflict

Multi-workspace: `wiki --workspace /path/to/other-wiki <command>` works for any number of wiki repos on the same device.