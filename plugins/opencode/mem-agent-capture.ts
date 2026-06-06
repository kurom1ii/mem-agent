import type { Plugin } from "@opencode-ai/plugin";

const FILE_TOOLS = new Set(["Read", "Write", "Edit", "Glob", "Grep"]);
const FILE_KEYS = ["filePath", "file_path", "path", "file", "pattern"];
const MAX_TRACKED_FILES = 30;
const DEBUG = process.env.MEMAGENT_DEBUG === "1";

const MEMAGENT_INSTRUCTIONS = `<mem-agent-instructions>
You have access to mem-agent for persistent cross-session memory. Use these MCP tools proactively.

CORE MCP TOOLS (all prefixed with "mem-agent_"):

memory_search — Hybrid search (BM25 + vector semantic) over memory store.
  Required: query (text to search), limit (max results, default 10)
  Optional: mode ("fts5" | "vector" | "hybrid", default "fts5")
  Use when: user asks to recall, search history, find past decisions, or needs context.

memory_add — Add a new memory entry.
  Required: title (short title), content (full text to remember)
  Optional: tags (comma-separated keywords)
  Use when: user says "remember this", after a key decision, learning a project convention.

memory_get — Get a memory entry by ID.
  Required: id (integer memory ID)
  Use when: you need full details of a specific memory.

memory_list — List recent memory entries.
  Optional: limit (default 20)
  Use when: user asks "what memories do we have", session overview, listing history.

memory_delete — Delete a memory entry by ID.
  Required: id (integer memory ID)
  Use when: user says "forget that", wants to remove incorrect/outdated memory.

index_stats — Get memory index statistics (counts).
  Use when: you want to know how many memories are stored.

All tools return text results. Present clearly to the user. Always verify tool output before summarizing.

Active knowledge management:
1. At session start, call memory_list to see recent context.
2. After important decisions, call memory_add.
3. Before editing files, call memory_search to check past context.
4. When user asks questions about past work, call memory_search.
</mem-agent-instructions>`;

function extractFilePaths(args: Record<string, unknown>): string[] {
  const files: string[] = [];
  for (const key of FILE_KEYS) {
    const val = args[key];
    if (typeof val === "string" && val.length > 0) files.push(val);
  }
  return files;
}

let activeSessionId: string | null = null;
let projectPath: string | null = null;
const trackedFiles = new Map<string, Set<string>>();
const contextInjected = new Set<string>();

function fileSet(sid: string): Set<string> {
  let s = trackedFiles.get(sid);
  if (!s) { s = new Set<string>(); trackedFiles.set(sid, s); }
  return s;
}

function pruneExpired(): void {
  if (trackedFiles.size > 50) {
    const oldest = [...trackedFiles.keys()].slice(0, 10);
    for (const k of oldest) trackedFiles.delete(k);
  }
  if (contextInjected.size > 50) {
    const oldest = [...contextInjected].slice(0, 10);
    for (const k of oldest) contextInjected.delete(k);
  }
}

export const MemAgentPlugin: Plugin = async (ctx) => {
  projectPath = ctx.worktree || ctx.project?.id || process.cwd();

  return {
    event: async ({ event }) => {
      const type = event.type;
      const props = (event as any).properties || {};

      if (type === "session.created") {
        const info = props.info as Record<string, unknown> | undefined;
        activeSessionId = (info?.id as string) || props.sessionID || null;
        if (activeSessionId) {
          trackedFiles.set(activeSessionId, new Set());
          contextInjected.delete(activeSessionId);
        }
        if (DEBUG) console.log(`[mem-agent] Session created: ${activeSessionId}`);
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID || activeSessionId;
        if (sid) {
          trackedFiles.delete(sid);
          contextInjected.delete(sid);
          if (sid === activeSessionId) activeSessionId = null;
          pruneExpired();
        }
      }

      if (type === "session.error") {
        const sid = props.sessionID || activeSessionId;
        if (sid && DEBUG) {
          console.error(`[mem-agent] Session error: ${JSON.stringify(props.error).slice(0, 500)}`);
        }
      }

      if (type === "file.edited") {
        const sid = props.sessionID || activeSessionId;
        if (sid && typeof props.file === "string") {
          const stash = fileSet(sid);
          stash.add(props.file);
          if (stash.size > MAX_TRACKED_FILES) {
            const keep = [...stash].slice(-MAX_TRACKED_FILES);
            stash.clear();
            for (const f of keep) stash.add(f);
          }
        }
      }
    },

    "chat.message": async (input, _output) => {
      const sid = input.sessionID || activeSessionId;
      if (!sid) return;
      if (DEBUG) {
        console.log(`[mem-agent] Chat message from session ${sid}`);
      }
    },

    "experimental.chat.system.transform": async (input, output) => {
      const sid = input.sessionID || activeSessionId;
      if (!sid || !Array.isArray(output.system)) return;

      if (!contextInjected.has(sid)) {
        output.system.push(MEMAGENT_INSTRUCTIONS);

        const stash = fileSet(sid);
        if (stash.size > 0) {
          const files = [...stash].slice(0, 10);
          output.system.push(
            `\n<mem-agent-files>\nRecently edited files: ${files.join(", ")}\nUse memory_search to check past context about these files.\n</mem-agent-files>\n`
          );
        }

        contextInjected.add(sid);
        pruneExpired();
        if (DEBUG) console.log(`[mem-agent] Context injected for session ${sid}`);
      }
    },

    "tool.execute.before": async (input, output) => {
      if (!FILE_TOOLS.has(input.tool)) return;
      const sid = input.sessionID || activeSessionId;
      if (!sid) return;
      const args = output.args as Record<string, unknown> | undefined;
      if (!args) return;
      const stash = fileSet(sid);
      for (const fp of extractFilePaths(args)) stash.add(fp);
      if (stash.size > MAX_TRACKED_FILES) {
        const keep = [...stash].slice(-MAX_TRACKED_FILES);
        stash.clear();
        for (const f of keep) stash.add(f);
      }
    },

    config: async (input) => {
      if (DEBUG) {
        console.log(`[mem-agent] Config loaded: model=${input.model?.id}, theme=${input.theme}`);
      }
    },
  };
};
