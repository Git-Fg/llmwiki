# Host configuration for `llmwiki-cli mcp`

The MCP server is invoked as:

```sh
llmwiki-cli mcp
```

It speaks JSON-RPC 2.0 over stdio. All five tools (`validate`, `hover`,
`completion`, `schema`, `doctor`) are exposed.

## Claude Desktop

In `~/Library/Application Support/Claude/claude_desktop_config.json`
(macOS) or `%APPDATA%/Claude/claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "llmwiki-cli": {
      "command": "llmwiki-cli",
      "args": ["mcp"]
    }
  }
}
```

Restart Claude Desktop after saving. The wiki tools appear under the 🔨
icon in the chat composer.

## Claude Code

Claude Code uses the CLI subcommand for registration:

```sh
claude mcp add llmwiki-cli -- llmwiki-cli mcp
```

This writes to `~/.claude.json` (or the project-scoped `.mcp.json` if run
from a project root with `--scope project`). Use `claude mcp list` to
confirm registration.

## Cursor

In `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "llmwiki-cli": {
      "command": "llmwiki-cli",
      "args": ["mcp"]
    }
  }
}
```

Cursor surfaces MCP tools under the Agent Tools picker (Cmd+I → Tools).

## Codex

Codex reads `~/.codex/config.toml`. Add:

```toml
[[mcp_servers]]
name = "llmwiki-cli"
command = "llmwiki-cli"
args = ["mcp"]
```

Restart Codex after saving.

## Continue.dev

In `~/.continue/config.json` under `"experimental"`:

```json
{
  "experimental": {
    "modelContextProtocolServers": [
      { "name": "llmwiki-cli", "command": "llmwiki-cli", "args": ["mcp"] }
    ]
  }
}
```

## Generic MCP host

Any host that speaks MCP-over-stdio can integrate:

1. Spawn `llmwiki-cli mcp` as a subprocess with stdio pipes.
2. Send JSON-RPC 2.0 messages terminated by `\n` (or with `Content-Length`
   framing for HTTP-style transports).
3. The server advertises 5 tools on `tools/list`; call them via `tools/call`
   with `name` and `arguments` (a JSON object matching the tool's input schema).

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `command not found: llmwiki-cli` | Binary not on PATH | Run `install.sh` (see `setup/`) |
| Host can't list tools | Binary panicked at startup | Run `llmwiki-cli mcp` in a terminal to see the panic |
| `validate` returns errors for a valid file | Cached old binary | Rebuild with `cargo install --path . --force` |
| `doctor` tool times out | NIM endpoint unreachable | Check `~/.wiki-root.toml` `[nim].base_url` |
| Tools appear but `completion` is empty | Cursor at column 0 | Move cursor to inside a `[table]` block |
