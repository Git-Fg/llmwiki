# Editor configuration for `llmwiki-cli lsp`

The LSP server is invoked as:

```sh
llmwiki-cli lsp
```

It speaks LSP over stdio. All four LSP methods (`hover`, `completion`,
`documentSymbol`, `publishDiagnostics`) are supported for `wiki-root.toml`.

## Helix

In `~/.config/helix/config.toml`:

```toml
[[language]]
name = "toml"
scope = "source.toml"
file-types = ["toml"]
roots = []
language-server = { llmwiki-cli-lsp = { command = "llmwiki-cli", args = ["lsp"] } }
```

Helix auto-detects `wiki-root.toml` because it uses `toml` file-type
detection. The `roots = []` is fine since wiki config files don't need
a project root marker.

## Neovim

With `nvim-lspconfig` (0.1.x or 0.2.x):

```lua
require('lspconfig').llmwiki_cli_lsp = {
  cmd = { 'llmwiki-cli', 'lsp' },
  filetypes = { 'toml' },
  root_dir = require('lspconfig.util').root_pattern('wiki-root.toml', '.git'),
  settings = {},
}
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'toml',
  callback = function(args)
    if vim.fn.findfile('wiki-root.toml', '.;') ~= '' then
      vim.lsp.start({
        name = 'llmwiki-cli-lsp',
        cmd = { 'llmwiki-cli', 'lsp' },
        filetypes = { 'toml' },
      })
    end
  end,
})
```

The autocmd ensures the LSP only starts for `wiki-root.toml` files, not
arbitrary `.toml` configs (e.g. `Cargo.toml`).

## Zed

In `~/.config/zed/settings.json`:

```json
{
  "lsp": {
    "llmwiki-cli": {
      "binary": {
        "path": "llmwiki-cli",
        "args": ["lsp"]
      },
      "file_types": ["toml"],
      "root": "wiki-root.toml"
    }
  }
}
```

## VS Code

In `.vscode/settings.json`:

```json
{
  "languageServerProtocol": {
    "llmwiki-cli-lsp": {
      "command": "llmwiki-cli",
      "args": ["lsp"],
      "filetypes": ["toml"]
    }
  }
}
```

Or with an extension shim, point to the binary via `"command"` and `"args"`.

## Other editors

Any editor with LSP support can integrate:

1. Spawn `llmwiki-cli lsp` as a subprocess with stdio pipes.
2. Route `.toml` files (or specifically `wiki-root.toml`) to this server.
3. Capabilities advertised by the server are: hover, completion,
   document-symbol, and full text-document sync. No pull diagnostics;
   the server pushes on every text change.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `command not found: llmwiki-cli` | Binary not on PATH | Run `install.sh` (see `setup/`) |
| No hover/completion appearing | Editor didn't start the LSP | Check editor LSP log; verify `wiki-root.toml` detection |
| "Connection closed" in editor log | LSP crashed | Run `llmwiki-cli lsp` in a terminal to see the panic |
| Diagnostics not updating | LSP doesn't auto-restart on file change | Save the file again to retrigger `didChange` |