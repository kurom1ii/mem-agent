# mem-agent OpenCode Plugin

Persistent memory for OpenCode — Rust backend with hybrid BM25 + vector search.

## Install

```bash
# Copy plugin to OpenCode
cp -r plugins/ ~/.config/opencode/plugins/mem-agent/

# Or symlink for development
ln -s $(pwd)/plugins/ ~/.config/opencode/plugins/mem-agent
```

## Setup

```bash
# Build mem-agent
cargo build --release

# Download the ONNX model (one time)
./target/release/mem-agent download

# Verify model works
./target/release/mem-agent verify

# Add mem-agent to PATH
export PATH="$PWD/target/release:$PATH"
```

## Features

- **Auto context injection**: Instructions + recent file context injected at session start
- **File tracking**: Tracks edited files for context enrichment
- **5 slash commands**: `/remember`, `/recall`, `/forget`, `/memories`, `/memstats`
- **6 MCP tools**: memory_search, memory_add, memory_get, memory_list, memory_delete, index_stats
- **Zero external dependencies**: SQLite only, local-first

## MCP Tools

| Tool | Description |
|------|-------------|
| `memory_search` | Hybrid search (BM25 + vector) |
| `memory_add` | Add new memory entry |
| `memory_get` | Get memory by ID |
| `memory_list` | List recent memories |
| `memory_delete` | Delete memory by ID |
| `index_stats` | Memory statistics |

## Architecture

```
plugins/
├── plugin.json              # Plugin manifest
├── .mcp.json                # MCP server config (mem-agent mcp)
├── hooks/hooks.json         # Session lifecycle hooks
├── opencode/
│   ├── plugin.json          # OpenCode plugin manifest
│   ├── mem-agent-capture.ts # Main hook: context injection + file tracking
│   └── commands/            # Slash commands
└── skills/                  # Agent skills (MD with YAML frontmatter)
    ├── remember/
    ├── recall/
    ├── forget/
    ├── session-history/
    ├── recap/
    └── handoff/
```
