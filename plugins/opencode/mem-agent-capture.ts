import type { Plugin } from "@opencode-ai/plugin";

// =====================================================================
// mem-agent OpenCode Plugin — full session lifecycle capture
// Adapted from agentmemory/rohitg00 pattern
// =====================================================================

const FILE_TOOLS = new Set(["Read", "Write", "Edit", "Glob", "Grep", "Bash"]);
const FILE_KEYS = ["filePath", "file_path", "path", "file", "pattern"];
const MAX_STASHED_FILES = 30;
const DEBUG = process.env.MEMAGENT_DEBUG === "1";

function log(...args: unknown[]): void {
  if (DEBUG) console.log("[mem-agent]", ...args);
}

function error(...args: unknown[]): void {
  if (DEBUG) console.error("[mem-agent]", ...args);
}

// ─── Memory Instructions injected into system prompt ───
const MEMAGENT_INSTRUCTIONS = `<mem-agent-instructions>
You have access to mem-agent for persistent cross-session memory. Use these MCP tools proactively.

AVAILABLE MCP TOOLS (use exact names with "mem-agent_" prefix):

mem-agent_memory_search — Hybrid search (BM25 + vector) over memory store.
  Args: query (string, required), limit (int, default 10), mode (string, "fts5"|"vector"|"hybrid")
  Use: to recall past decisions, search project history, find relevant context before editing.

mem-agent_memory_add — Add a new memory entry.
  Args: title (string, required), content (string, required), tags (string, optional comma-separated)
  Use: to save insights, decisions, learnings, project conventions, bug discoveries.

mem-agent_memory_get — Get a specific memory by ID.
  Args: id (int, required)
  Use: to retrieve full details of a previously saved memory.

mem-agent_memory_list — List recent memory entries.
  Args: limit (int, default 20)
  Use: at session start for overview, to browse what's stored.

mem-agent_memory_delete — Delete a memory entry.
  Args: id (int, required)
  Use: when user says "forget that", remove incorrect/outdated memories.

mem-agent_index_stats — Memory index statistics.
  Use: to check how many memories are stored, vector count.

BEST PRACTICES:
1. At session start, call memory_list to see recent context.
2. After making important decisions, call memory_add.
3. Before editing a file, call memory_search to check past context about it.
4. When user asks about past work, ALWAYS call memory_search first.
5. After fixing a bug, save the root cause and fix as a memory.
6. Tag memories with relevant keywords (comma-separated) for better searchability.
</mem-agent-instructions>`;

// ─── File path extraction ───
function extractFilePaths(args: Record<string, unknown>): string[] {
  const files: string[] = [];
  for (const key of FILE_KEYS) {
    const val = args[key];
    if (typeof val === "string" && val.length > 0) files.push(val);
  }
  return files;
}

function safeSlice(v: unknown, max: number): string {
  if (typeof v === "string") return v.slice(0, max);
  if (v == null) return "";
  try { return JSON.stringify(v).slice(0, max); } catch { return ""; }
}

function extractError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object") {
    const e = err as Record<string, unknown>;
    if (typeof e.message === "string") return e.message;
    return String(e.name ?? JSON.stringify(err).slice(0, 200));
  }
  return String(err ?? "");
}

// ─── Session state ───
let activeSessionId: string | null = null;
let projectPath: string | null = null;
const trackedFiles = new Map<string, Set<string>>();
const seenToolCallIds = new Map<string, Set<string>>();
const contextInjected = new Set<string>();

function fileSet(sid: string): Set<string> {
  let s = trackedFiles.get(sid);
  if (!s) { s = new Set<string>(); trackedFiles.set(sid, s); }
  return s;
}

function toolCallSet(sid: string): Set<string> {
  let s = seenToolCallIds.get(sid);
  if (!s) { s = new Set<string>(); seenToolCallIds.set(sid, s); }
  return s;
}

function pruneMaps(): void {
  if (trackedFiles.size > 100) {
    const oldest = [...trackedFiles.keys()].slice(0, 30);
    for (const k of oldest) trackedFiles.delete(k);
  }
  if (seenToolCallIds.size > 50) {
    const oldest = [...seenToolCallIds.keys()].slice(0, 10);
    for (const k of oldest) seenToolCallIds.delete(k);
  }
  if (contextInjected.size > 50) {
    const oldest = [...contextInjected].slice(0, 10);
    for (const k of oldest) contextInjected.delete(k);
  }
}

// =====================================================================
// PLUGIN ENTRY
// =====================================================================
export const MemAgentPlugin: Plugin = async (ctx) => {
  projectPath = ctx.worktree || ctx.project?.id || process.cwd();
  log(`Plugin loaded — project: ${projectPath}`);

  return {
    // ─── EVENT HANDLER (session lifecycle) ───
    event: async ({ event }) => {
      const type = event.type;
      const props = (event as any).properties || {};

      // session.created
      if (type === "session.created") {
        const info = props.info as Record<string, unknown> | undefined;
        activeSessionId = (info?.id as string) || props.sessionID || null;
        if (activeSessionId) {
          trackedFiles.set(activeSessionId, new Set());
          seenToolCallIds.delete(activeSessionId);
          contextInjected.delete(activeSessionId);
        }
        log(`Session created: ${activeSessionId}`);
      }

      // session.status
      if (type === "session.status") {
        const status = props.status as Record<string, unknown> | undefined;
        const sid = props.sessionID || activeSessionId;
        if (sid && status) {
          log(`Session status: ${status.type} (session=${sid.slice(0, 8)}...)`);
          if (status.type === "idle") {
            log(`Session ${sid.slice(0, 8)} went idle`);
          }
        }
      }

      // session.compacted
      if (type === "session.compacted") {
        const sid = props.sessionID || activeSessionId;
        log(`Session compacted: ${sid?.slice(0, 8)}`);
      }

      // session.updated
      if (type === "session.updated") {
        const info = props.info as Record<string, unknown> | undefined;
        const sid = (info?.id as string) || props.sessionID || activeSessionId;
        if (sid) log(`Session updated: ${sid.slice(0, 8)}`);
      }

      // session.diff
      if (type === "session.diff") {
        const sid = props.sessionID || activeSessionId;
        if (sid && Array.isArray(props.diff)) {
          const diffs = props.diff as Array<Record<string, unknown>>;
          for (const d of diffs) {
            if (typeof d.file === "string") fileSet(sid).add(d.file);
          }
        }
      }

      // session.deleted
      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID || activeSessionId;
        if (sid) {
          log(`Session deleted: ${sid}`);
          trackedFiles.delete(sid);
          seenToolCallIds.delete(sid);
          contextInjected.delete(sid);
          if (sid === activeSessionId) activeSessionId = null;
          pruneMaps();
        }
      }

      // session.error
      if (type === "session.error") {
        const sid = props.sessionID || activeSessionId;
        if (sid) {
          error(`Session error: ${safeSlice(props.error, 500)}`);
        }
      }

      // message.updated (assistant messages)
      if (type === "message.updated") {
        const info = props.info as Record<string, unknown> | undefined;
        if (info?.role === "assistant") {
          log(`Assistant message: ${(info.id as string)?.slice(0, 8)}`);
        }
      }

      // message.removed
      if (type === "message.removed") {
        log(`Message removed: ${props.messageID}`);
      }

      // message.part.updated (tool calls, subtasks)
      if (type === "message.part.updated") {
        const part = props.part as Record<string, unknown> | undefined;
        if (!part) return;
        const sid = (part.sessionID as string) || props.sessionID || activeSessionId;
        if (!sid) return;

        if (part.type === "subtask") {
          log(`Subtask started: ${part.id} (agent=${part.agent})`);
          return;
        }

        if (part.type === "tool") {
          const state = part.state as Record<string, unknown> | undefined;
          if (!state) return;
          const callId = part.callID as string;
          if (!callId) return;
          const toolName = part.tool as string;

          if (state.status === "completed") {
            const callSet = toolCallSet(sid);
            if (callSet.has(callId)) return;
            callSet.add(callId);
            const st = state as Record<string, unknown>;
            log(`Tool completed: ${toolName} (${callId.slice(0, 8)})`);

            // Track file tools for context enrichment
            if (FILE_TOOLS.has(toolName) && st.input) {
              const args = st.input as Record<string, unknown>;
              for (const fp of extractFilePaths(args)) {
                fileSet(sid).add(fp);
              }
            }
          } else if (state.status === "error") {
            const callSet = toolCallSet(sid);
            if (callSet.has(callId)) return;
            callSet.add(callId);
            error(`Tool failed: ${toolName} — ${safeSlice(state.error, 200)}`);
          }
          return;
        }

        if (part.type === "step-finish") {
          log(`Step finished: reason=${part.reason}`);
        }

        if (part.type === "reasoning") {
          log(`Reasoning: ${safeSlice((part as any).text, 100)}`);
        }

        if (part.type === "file") {
          const filename = (part as any).filename || (part as any).url;
          if (filename) fileSet(sid).add(filename);
        }

        if (part.type === "patch") {
          const pf = (part as any).files || [];
          for (const f of pf) fileSet(sid).add(f);
        }
      }

      // file.edited
      if (type === "file.edited") {
        const sid = props.sessionID || activeSessionId;
        if (sid && typeof props.file === "string") {
          const stash = fileSet(sid);
          stash.add(props.file);
          if (stash.size > MAX_STASHED_FILES) {
            const keep = [...stash].slice(-MAX_STASHED_FILES);
            stash.clear();
            for (const f of keep) stash.add(f);
          }
          log(`File edited: ${props.file} (${stash.size} tracked)`);
        }
      }

      // permission.updated
      if (type === "permission.updated") {
        const sid = props.sessionID || activeSessionId;
        if (sid) log(`Permission prompt: ${props.type} — ${props.pattern}`);
      }

      // permission.replied
      if (type === "permission.replied") {
        log(`Permission reply: ${props.response || props.reply}`);
      }

      // todo.updated
      if (type === "todo.updated") {
        const todos = Array.isArray(props.todos) ? props.todos : [];
        const completed = todos.filter((t: any) => t.status === "completed");
        const active = todos.filter((t: any) => t.status !== "completed");
        if (completed.length > 0) {
          log(`Tasks: ${completed.length} done, ${active.length} remaining`);
        }
      }

      // command.executed
      if (type === "command.executed") {
        log(`Command executed: ${props.name}`);
      }
    },

    // ─── chat.message ───
    "chat.message": async (input, _output) => {
      const sid = input.sessionID || activeSessionId;
      if (sid) log(`Chat message from ${sid.slice(0, 8)}`);
    },

    // ─── chat.params ───
    "chat.params": async (input, _output) => {
      if (input.model) {
        log(`Model: ${input.model.providerID}/${input.model.id}`);
      }
    },

    // ─── experimental.chat.system.transform ───
    "experimental.chat.system.transform": async (input, output) => {
      const sid = input.sessionID || activeSessionId;
      if (!sid || !Array.isArray(output.system)) return;

      // Inject instructions once per session
      if (!contextInjected.has(sid)) {
        output.system.push(MEMAGENT_INSTRUCTIONS);

        // Inject tracked files context
        const stash = fileSet(sid);
        if (stash.size > 0) {
          const files = [...stash].slice(0, 10);
          output.system.push(
            `\n<mem-agent-context>\n` +
            `Recently active files: ${files.join(", ")}\n` +
            `Use mem-agent_memory_search to recall past context about these files before editing.\n` +
            `</mem-agent-context>\n`
          );
        }

        contextInjected.add(sid);
        pruneMaps();
        log(`Instructions injected for session ${sid.slice(0, 8)} (${stash.size} files tracked)`);
      }
    },

    // ─── tool.execute.before ───
    "tool.execute.before": async (input, output) => {
      if (!FILE_TOOLS.has(input.tool)) return;
      const sid = input.sessionID || activeSessionId;
      if (!sid) return;
      const args = output.args as Record<string, unknown> | undefined;
      if (!args) return;
      const stash = fileSet(sid);
      for (const fp of extractFilePaths(args)) stash.add(fp);
      if (stash.size > MAX_STASHED_FILES) {
        const keep = [...stash].slice(-MAX_STASHED_FILES);
        stash.clear();
        for (const f of keep) stash.add(f);
      }
    },

    // ─── config ───
    config: async (input) => {
      log(`Config: model=${input.model?.id}, theme=${input.theme}, agent=${input.agent?.id}`);
    },
  };
};
