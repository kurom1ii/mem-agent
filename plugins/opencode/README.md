# mem-agent OpenCode Plugin

Persistent memory for OpenCode, backed by local Rust `mem-agent` MCP server.

## Install

```bash
# Copy plugin package to OpenCode
cp -r plugins/ ~/.config/opencode/plugins/mem-agent/

# Or symlink for development
ln -s $(pwd)/plugins/ ~/.config/opencode/plugins/mem-agent
```

## Setup

```bash
# Build mem-agent
cargo build --release

# Add mem-agent to PATH
export PATH="$PWD/target/release:$PATH"
```

`plugins/.mcp.json` expects `mem-agent` to be callable from `PATH` and starts:

```bash
mem-agent --db memories.db mcp
```

If your DB lives elsewhere, set `MEMAGENT_DB` for the hook scripts and adjust `.mcp.json`.

## OpenCode config

Add plugin + MCP package to your OpenCode config:

```json
{
  "plugin": [
    "~/.config/opencode/plugins/mem-agent/opencode/mem-agent-capture.ts"
  ],
  "mcp": {
    "mem-agent": {
      "type": "local",
      "command": ["mem-agent", "--db", "memories.db", "mcp"],
      "enabled": true
    }
  }
}
```

If you symlinked this repo into `~/.config/opencode/plugins/mem-agent`, path above works directly.

## Logs

Plugin writes colorized debug log to:

```bash
/home/kuromi/work/mywork/mem-agent/plugins/opencode/log.log
```

Watch live:

```bash
tail -f /home/kuromi/work/mywork/mem-agent/plugins/opencode/log.log
```

## Features

- **Auto capture**: User prompts, assistant metadata, tool outcomes, patches, and step-finish events are auto-saved into mem-agent
- **Auto recall**: Recent memories, prompt-based recall, and file-based recall are injected into the system prompt
- **File tracking**: Tracks edited files for recall enrichment
- **5 slash commands**: `/remember`, `/recall`, `/forget`, `/memories`, `/memstats`
- **6 MCP tools**: memory_search, memory_add, memory_get, memory_list, memory_delete, index_stats
- **Local-first**: no HTTP sidecar; plugin auto-capture uses local CLI, agent-facing tools use local MCP

## MCP Tools

| Tool | Description |
|------|-------------|
| `memory_search` | Search memories with `hybrid`, `fts5`, or `vector`. Hybrid falls back to FTS5 if vectors are unavailable. |
| `memory_add` | Add new memory entry |
| `memory_get` | Get memory by ID |
| `memory_list` | List recent memories |
| `memory_delete` | Delete memory by ID |
| `index_stats` | Memory statistics |

## Architecture

```
plugins/
├── plugin.json              # Plugin manifest
├── .mcp.json                # MCP server config (starts `mem-agent --db memories.db mcp`)
├── hooks/hooks.json         # Session lifecycle hooks
├── opencode/
│   ├── plugin.json          # OpenCode plugin manifest
│   ├── mem-agent-capture.ts # Main hook: context injection + file tracking
│   └── commands/            # Slash commands
├── scripts/                 # Hook helpers + MCP wrapper helpers
└── skills/                  # Agent skills (MD with YAML frontmatter)
    ├── remember/
    ├── recall/
    ├── forget/
    ├── session-history/
    ├── recap/
    └── handoff/
```
