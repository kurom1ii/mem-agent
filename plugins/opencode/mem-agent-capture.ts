import type { Plugin } from "@opencode-ai/plugin";
import { spawn } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const FILE_TOOLS = new Set(["Read", "Write", "Edit", "Glob", "Grep", "Bash"]);
const FILE_KEYS = ["filePath", "file_path", "path", "file", "pattern"];
const MAX_STASHED_FILES = 30;
const MAX_MEMORY_CONTENT = 4000;
const RECENT_MEMORY_LIMIT = 8;
const RECALL_LIMIT = 5;
const DEBUG = process.env.MEMAGENT_DEBUG === "1";
const AUTO_SAVE = process.env.MEMAGENT_AUTO_SAVE !== "0";
const AUTO_RECALL = process.env.MEMAGENT_AUTO_RECALL !== "0";
const LOG_PATH = "/home/kuromi/work/mywork/mem-agent/plugins/opencode/log.log";
const SEP = "─".repeat(92);
const LABEL_WIDTH = 22;

type PromptCapture = {
  text: string;
  files: string[];
  agent: string;
  model: string;
  variant: string;
  parts: string[];
};

type RunResult = {
  ok: boolean;
  stdout: string;
  stderr: string;
  code: number | null;
};

const ANSI = {
  reset: "\x1b[0m",
  dim: "\x1b[2m",
  bold: "\x1b[1m",
  gray: "\x1b[90m",
  red: "\x1b[91m",
  green: "\x1b[92m",
  yellow: "\x1b[93m",
  blue: "\x1b[94m",
  magenta: "\x1b[95m",
  cyan: "\x1b[96m",
  white: "\x1b[97m",
} as const;

const LABEL_COLORS: Record<string, string> = {
  PLUGIN_LOADED: ANSI.bold + ANSI.white,
  SESSION_CREATED: ANSI.bold + ANSI.green,
  SESSION_DELETED: ANSI.bold + ANSI.red,
  USER_CAPTURE: ANSI.blue,
  ASSIST_CAPTURE: ANSI.green,
  TOOL_CAPTURE: ANSI.cyan,
  PATCH_CAPTURE: ANSI.yellow,
  STEP_CAPTURE: ANSI.magenta,
  FILE_TRACK: ANSI.blue,
  AUTO_SAVE_OK: ANSI.green,
  AUTO_SAVE_ERR: ANSI.red,
  AUTO_RECALL_OK: ANSI.cyan,
  AUTO_RECALL_EMPTY: ANSI.gray,
  AUTO_RECALL_ERR: ANSI.red,
  RECENT_LOAD: ANSI.white,
  SYSTEM_INJECT: ANSI.magenta,
  TOOL_BEFORE: ANSI.yellow,
  DEBUG: ANSI.gray,
  SECTION: ANSI.dim + ANSI.gray,
} as const;

const LABEL_DESCRIPTIONS: Record<string, string> = {
  PLUGIN_LOADED: "plugin boot",
  SESSION_CREATED: "new session",
  SESSION_DELETED: "session closed",
  USER_CAPTURE: "user prompt saved",
  ASSIST_CAPTURE: "assistant meta saved",
  TOOL_CAPTURE: "tool result saved",
  PATCH_CAPTURE: "patch saved",
  STEP_CAPTURE: "step saved",
  FILE_TRACK: "tracked file",
  AUTO_SAVE_OK: "memory write ok",
  AUTO_SAVE_ERR: "memory write fail",
  AUTO_RECALL_OK: "memory recall hit",
  AUTO_RECALL_EMPTY: "memory recall miss",
  AUTO_RECALL_ERR: "memory recall fail",
  RECENT_LOAD: "recent memories",
  SYSTEM_INJECT: "system context inject",
  TOOL_BEFORE: "pre-tool stash",
  DEBUG: "debug",
  SECTION: "session marker",
} as const;

function log(...args: unknown[]): void {
  if (DEBUG) console.log("[mem-agent]", ...args);
  logLine("DEBUG", { text: args.map((arg) => safeSlice(arg, 180)).join(" | ") });
}

function errlog(...args: unknown[]): void {
  if (DEBUG) console.error("[mem-agent]", ...args);
  logLine("AUTO_SAVE_ERR", { text: args.map((arg) => safeSlice(arg, 180)).join(" | ") });
}

function safeSlice(v: unknown, max: number): string {
  if (typeof v === "string") return v.slice(0, max);
  if (v == null) return "";
  try {
    return JSON.stringify(v).slice(0, max);
  } catch {
    return "";
  }
}

function oneLine(v: unknown, max: number): string {
  return safeSlice(v, max).replace(/\s+/g, " ").trim();
}

function nowStamp(): string {
  const d = new Date();
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  const ms = String(d.getMilliseconds()).padStart(3, "0");
  return `${hh}:${mm}:${ss}.${ms}`;
}

function padLabel(label: string): string {
  return label.padEnd(LABEL_WIDTH, " ");
}

function fmtFields(fields: Record<string, unknown>): string {
  return Object.entries(fields)
    .filter(([, value]) => value !== "" && value != null)
    .map(([key, value]) => `${ANSI.dim}${key}${ANSI.reset}=${oneLine(value, 220)}`)
    .join(` ${ANSI.gray}|${ANSI.reset} `);
}

function rawWrite(line: string): void {
  appendFileSync(LOG_PATH, `${line}\n`);
}

function logLine(label: string, fields: Record<string, unknown> = {}): void {
  const color = LABEL_COLORS[label] || ANSI.white;
  const desc = LABEL_DESCRIPTIONS[label];
  const descText = desc ? ` ${ANSI.dim}(${desc})${ANSI.reset}` : "";
  const head = `${ANSI.gray}[${nowStamp()}]${ANSI.reset} ${color}${padLabel(label)}${ANSI.reset}${descText}`;
  const body = fmtFields(fields);
  rawWrite(body ? `${head} ${ANSI.gray}>>${ANSI.reset} ${body}` : head);
}

function logSection(title: string, sessionId: string): void {
  rawWrite(`${LABEL_COLORS.SECTION}${SEP}${ANSI.reset}`);
  logLine("SECTION", { title, session: sessionId || "-" });
  rawWrite(`${LABEL_COLORS.SECTION}${SEP}${ANSI.reset}`);
}

function normalizeTag(tag: string): string {
  return tag
    .toLowerCase()
    .replace(/[^a-z0-9._/-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function joinTags(tags: string[]): string {
  return [...new Set(tags.map(normalizeTag).filter(Boolean))].join(",");
}

function extractFilePaths(args: Record<string, unknown>): string[] {
  const files: string[] = [];
  for (const key of FILE_KEYS) {
    const val = args[key];
    if (typeof val === "string" && val.length > 0) files.push(val);
  }
  return files;
}

function summarizeModel(model: unknown): string {
  if (typeof model === "string") return model;
  if (!model || typeof model !== "object") return "";
  const m = model as Record<string, unknown>;
  const provider = typeof m.providerID === "string" ? m.providerID : "";
  const id = typeof m.id === "string" ? m.id : "";
  return [provider, id].filter(Boolean).join("/");
}

function extractTextParts(parts: unknown[]): string {
  return parts
    .filter((part) => part && typeof part === "object")
    .map((part) => part as Record<string, unknown>)
    .filter((part) => part.type === "text" && !part.synthetic && !part.ignored)
    .map((part) => (typeof part.text === "string" ? part.text : ""))
    .filter(Boolean)
    .join("\n")
    .trim();
}

function extractFileParts(parts: unknown[]): string[] {
  return parts
    .filter((part) => part && typeof part === "object")
    .map((part) => part as Record<string, unknown>)
    .filter((part) => part.type === "file")
    .map((part) => {
      if (typeof part.filename === "string") return part.filename;
      if (typeof part.url === "string") return part.url;
      return "";
    })
    .filter(Boolean);
}

function summarizeParts(parts: unknown[]): string[] {
  return parts
    .filter((part) => part && typeof part === "object")
    .map((part) => {
      const p = part as Record<string, unknown>;
      return typeof p.type === "string" ? p.type : "";
    })
    .filter(Boolean);
}

function makeTitle(prefix: string, text: string): string {
  const preview = oneLine(text, 64) || "event";
  return oneLine(`${prefix}: ${preview}`, 80);
}

function resolveMemAgentBin(): string {
  const envBin = process.env.MEMAGENT_BIN?.trim();
  if (envBin) return envBin;

  const releaseBin = path.resolve(process.cwd(), "target", "release", "mem-agent");
  if (existsSync(releaseBin)) return releaseBin;

  const debugBin = path.resolve(process.cwd(), "target", "debug", "mem-agent");
  if (existsSync(debugBin)) return debugBin;

  return "mem-agent";
}

function resolveDbPath(): string {
  return process.env.MEMAGENT_DB?.trim() || path.resolve(process.cwd(), "memories.db");
}

async function runMemAgent(args: string[], timeoutMs = 8000): Promise<RunResult> {
  return new Promise((resolve) => {
    const child = spawn(resolveMemAgentBin(), ["--db", resolveDbPath(), ...args], {
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";
    let settled = false;

    const finish = (result: RunResult) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(result);
    };

    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish({
        ok: false,
        stdout,
        stderr: stderr || `Timed out after ${timeoutMs}ms`,
        code: null,
      });
    }, timeoutMs);

    child.stdout.on("data", (data) => {
      stdout += data.toString();
    });
    child.stderr.on("data", (data) => {
      stderr += data.toString();
    });
    child.on("error", (error) => {
      finish({
        ok: false,
        stdout,
        stderr: error.message,
        code: null,
      });
    });
    child.on("close", (code) => {
      finish({
        ok: code === 0,
        stdout: stdout.trim(),
        stderr: stderr.trim(),
        code,
      });
    });
  });
}

async function addMemory(title: string, content: string, tags: string[]): Promise<void> {
  if (!AUTO_SAVE) return;

  const cleanTitle = oneLine(title, 80);
  const cleanContent = safeSlice(content, MAX_MEMORY_CONTENT).trim();
  if (!cleanTitle || !cleanContent) return;

  const result = await runMemAgent(
    ["add", cleanTitle, cleanContent, "--tags", joinTags(tags)],
    12000,
  );

  if (!result.ok) {
    logLine("AUTO_SAVE_ERR", {
      title: cleanTitle,
      stderr: result.stderr || result.stdout,
      code: result.code ?? "",
    });
    errlog("auto-save failed", cleanTitle, result.stderr || result.stdout);
    return;
  }

  logLine("AUTO_SAVE_OK", {
    title: cleanTitle,
    tags: joinTags(tags),
    result: result.stdout,
  });
}

async function searchMemories(query: string, limit = RECALL_LIMIT): Promise<string> {
  const cleanQuery = safeSlice(query, 500).trim();
  if (!AUTO_RECALL || !cleanQuery) return "";

  const result = await runMemAgent(
    ["search", cleanQuery, "--mode", "fts5", "-n", String(limit)],
    8000,
  );

  if (!result.ok) {
    logLine("AUTO_RECALL_ERR", {
      query: cleanQuery,
      stderr: result.stderr || result.stdout,
      code: result.code ?? "",
    });
    errlog("auto-recall failed", cleanQuery, result.stderr || result.stdout);
    return "";
  }

  if (!result.stdout || result.stdout.includes("No results found")) {
    logLine("AUTO_RECALL_EMPTY", {
      query: cleanQuery,
      limit,
    });
    return "";
  }

  logLine("AUTO_RECALL_OK", {
    query: cleanQuery,
    limit,
    result: result.stdout,
  });
  return result.stdout;
}

async function listRecentMemories(limit = RECENT_MEMORY_LIMIT): Promise<string> {
  const result = await runMemAgent(["list", "-n", String(limit)], 8000);
  if (!result.ok || !result.stdout || result.stdout.includes("No memories stored")) {
    logLine("RECENT_LOAD", {
      limit,
      status: "empty",
      stderr: result.stderr,
    });
    return "";
  }
  logLine("RECENT_LOAD", {
    limit,
    status: "loaded",
    result: result.stdout,
  });
  return result.stdout;
}

const MEMAGENT_INSTRUCTIONS = `<mem-agent-instructions>
Bạn có quyền truy cập mem-agent — bộ nhớ dài hạn cho AI agent.
Hãy CHỦ ĐỘNG dùng các MCP tool dưới đây.

CÔNG CỤ CÓ SẴN (dùng đúng tên với prefix "mem-agent_"):

mem-agent_memory_search — Tìm kiếm memory đã lưu.
  Args: query (string, bắt buộc), limit (int, mặc định 10),
        mode ("fts5"|"vector"|"hybrid")
  → Dùng khi: user hỏi "nhớ gì về...", cần context trước khi sửa file,
    muốn biết lịch sử project.
  → Mặc định nên dùng mode="hybrid". Nếu model/vector chưa sẵn,
    backend sẽ fallback về FTS5.

mem-agent_memory_add — Lưu memory mới.
  Args: title (string), content (string), tags (string, optional)
  → Dùng khi: user nói "nhớ cái này", sau khi fix bug, sau quyết định
    kiến trúc quan trọng, học được convention mới của project.

mem-agent_memory_list — Danh sách memory gần đây.
  Args: limit (int, mặc định 20)
  → Dùng khi: bắt đầu session (để biết context), user hỏi "có những gì".

mem-agent_memory_get — Lấy chi tiết 1 memory.
  Args: id (int)
  → Dùng khi: cần xem đầy đủ nội dung memory theo ID.

mem-agent_memory_delete — Xóa memory.
  Args: id (int)
  → Dùng khi: user nói "quên đi", memory không còn đúng.

mem-agent_index_stats — Thống kê bộ nhớ.
  → Dùng khi: muốn biết có bao nhiêu memory đã lưu.

QUY TẮC:
1. Đầu session → gọi memory_list để xem context gần đây.
2. Trước khi edit file → gọi memory_search với tên file, ưu tiên mode="hybrid".
3. Sau quyết định quan trọng → gọi memory_add.
4. Khi user hỏi về quá khứ → LUÔN gọi memory_search trước khi trả lời.
</mem-agent-instructions>`;

let activeSessionId: string | null = null;
let projectPath: string | null = null;
const trackedFiles = new Map<string, Set<string>>();
const seenToolCallIds = new Map<string, Set<string>>();
const contextInjected = new Set<string>();
const pendingPrompts = new Map<string, PromptCapture>();
const turnPrompts = new Map<string, string>();

function fileSet(sid: string): Set<string> {
  let s = trackedFiles.get(sid);
  if (!s) {
    s = new Set<string>();
    trackedFiles.set(sid, s);
  }
  return s;
}

function toolCallSet(sid: string): Set<string> {
  let s = seenToolCallIds.get(sid);
  if (!s) {
    s = new Set<string>();
    seenToolCallIds.set(sid, s);
  }
  return s;
}

function cleanupSession(sid: string): void {
  trackedFiles.delete(sid);
  seenToolCallIds.delete(sid);
  contextInjected.delete(sid);
  pendingPrompts.delete(sid);
  turnPrompts.delete(sid);
}

export const MemAgentPlugin: Plugin = async (ctx) => {
  projectPath = ctx.worktree || ctx.project?.id || process.cwd();
  mkdirSync(path.dirname(LOG_PATH), { recursive: true });
  writeFileSync(LOG_PATH, "");
  logLine("PLUGIN_LOADED", {
    project: projectPath,
    pid: process?.pid,
    log: LOG_PATH,
    autoSave: AUTO_SAVE,
    autoRecall: AUTO_RECALL,
    db: resolveDbPath(),
    bin: resolveMemAgentBin(),
  });
  log("plugin loaded", projectPath);

  return {
    event: async ({ event }) => {
      const type = event.type;
      const props = (event as any).properties || {};

      if (type === "session.created") {
        const info = props.info as Record<string, unknown> | undefined;
        activeSessionId = (info?.id as string) || props.sessionID || null;
        if (!activeSessionId) return;
        logSection("SESSION START", activeSessionId);
        logLine("SESSION_CREATED", {
          id: activeSessionId,
          title: safeSlice(info?.title, 100),
        });
        fileSet(activeSessionId);
        seenToolCallIds.delete(activeSessionId);
        contextInjected.delete(activeSessionId);
        pendingPrompts.delete(activeSessionId);
        turnPrompts.delete(activeSessionId);
        return;
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID || activeSessionId;
        if (!sid) return;
        logLine("SESSION_DELETED", { id: sid });
        logSection("SESSION END", sid);
        cleanupSession(sid);
        if (sid === activeSessionId) activeSessionId = null;
        return;
      }

      if (type === "message.updated") {
        const info = props.info as Record<string, unknown> | undefined;
        const sid = props.sessionID || (info?.sessionID as string) || activeSessionId;
        if (!info || !sid) return;

        if (info.role === "user") {
          const capture = pendingPrompts.get(sid);
          const prompt = capture?.text || turnPrompts.get(sid) || "";
          if (!prompt) return;

          turnPrompts.set(sid, prompt);
          await addMemory(
            makeTitle("user", prompt),
            [
              `kind=prompt_submit`,
              `session=${sid}`,
              `project=${projectPath || ""}`,
              `agent=${capture?.agent || ""}`,
              `model=${capture?.model || ""}`,
              `variant=${capture?.variant || ""}`,
              `files=${(capture?.files || []).join(", ")}`,
              `parts=${(capture?.parts || []).join(", ")}`,
              "",
              prompt,
            ].join("\n"),
            ["auto", "prompt", "user", "chat"],
          );
          logLine("USER_CAPTURE", {
            session: sid,
            prompt,
            files: (capture?.files || []).join(", "),
            model: capture?.model || "",
          });
          return;
        }

        if (info.role === "assistant") {
          turnPrompts.delete(sid);
          pendingPrompts.delete(sid);
          await addMemory(
            makeTitle("assistant", `${info.modelID || "reply"} ${info.finish || ""}`),
            [
              `kind=assistant_message`,
              `session=${sid}`,
              `project=${projectPath || ""}`,
              `model=${info.modelID || ""}`,
              `provider=${info.providerID || ""}`,
              `finish=${info.finish || ""}`,
              `error=${safeSlice(info.error, 300)}`,
            ].join("\n"),
            ["auto", "assistant", "reply", "meta"],
          );
          logLine("ASSIST_CAPTURE", {
            session: sid,
            model: info.modelID || "",
            provider: info.providerID || "",
            finish: info.finish || "",
            error: safeSlice(info.error, 200),
          });
          return;
        }
      }

      if (type === "message.part.updated") {
        const part = props.part as Record<string, unknown> | undefined;
        if (!part) return;
        const sid = (part.sessionID as string) || props.sessionID || activeSessionId;
        if (!sid) return;

        if (part.type === "tool") {
          const state = part.state as Record<string, unknown> | undefined;
          if (!state) return;

          const callId = part.callID as string;
          if (!callId) return;

          const seen = toolCallSet(sid);
          if (seen.has(callId)) return;

          if (state.status === "completed" || state.status === "error") {
            seen.add(callId);

            if (state.input && typeof state.input === "object") {
              for (const file of extractFilePaths(state.input as Record<string, unknown>)) {
                fileSet(sid).add(file);
              }
            }

            await addMemory(
              makeTitle(
                `tool-${state.status}`,
                `${part.tool || "tool"} ${state.title || callId}`,
              ),
              [
                `kind=${state.status === "completed" ? "post_tool_use" : "post_tool_failure"}`,
                `session=${sid}`,
                `project=${projectPath || ""}`,
                `tool=${part.tool || ""}`,
                `call_id=${callId}`,
                `title=${safeSlice(state.title, 160)}`,
                `input=${safeSlice(state.input, 2000)}`,
                `output=${safeSlice(state.output ?? state.error, 2000)}`,
              ].join("\n"),
              [
                "auto",
                "tool",
                typeof part.tool === "string" ? part.tool : "tool",
                state.status === "completed" ? "success" : "error",
              ],
            );
            logLine("TOOL_CAPTURE", {
              session: sid,
              tool: part.tool || "",
              status: state.status,
              call: callId,
              title: safeSlice(state.title, 120),
            });
          }
          return;
        }

        if (part.type === "patch") {
          const files = Array.isArray((part as any).files) ? (part as any).files : [];
          for (const file of files) {
            if (typeof file === "string") fileSet(sid).add(file);
          }
          await addMemory(
            makeTitle("patch", files.join(", ") || "patch"),
            [
              `kind=patch_applied`,
              `session=${sid}`,
              `project=${projectPath || ""}`,
              `hash=${safeSlice((part as any).hash, 80)}`,
              `files=${files.join(", ")}`,
            ].join("\n"),
            ["auto", "patch", "code"],
          );
          logLine("PATCH_CAPTURE", {
            session: sid,
            files: files.join(", "),
          });
          return;
        }

        if (part.type === "file") {
          const filename = (part as any).filename || (part as any).url;
          if (typeof filename === "string" && filename.length > 0) {
            fileSet(sid).add(filename);
          }
          return;
        }

        if (part.type === "step-finish") {
          await addMemory(
            makeTitle("step", safeSlice(part.reason, 80)),
            [
              `kind=step_finish`,
              `session=${sid}`,
              `project=${projectPath || ""}`,
              `reason=${safeSlice(part.reason, 120)}`,
              `cost=${safeSlice((part as any).cost, 80)}`,
              `tokens=${safeSlice((part as any).tokens, 200)}`,
            ].join("\n"),
            ["auto", "step", "reasoning"],
          );
          logLine("STEP_CAPTURE", {
            session: sid,
            reason: safeSlice(part.reason, 120),
          });
          return;
        }
      }

      if (type === "file.edited") {
        const sid = props.sessionID || activeSessionId;
        if (sid && typeof props.file === "string") {
          const stash = fileSet(sid);
          stash.add(props.file);
          if (stash.size > MAX_STASHED_FILES) {
            const keep = [...stash].slice(-MAX_STASHED_FILES);
            stash.clear();
            for (const file of keep) stash.add(file);
          }
          logLine("FILE_TRACK", {
            session: sid,
            file: props.file,
            tracked: stash.size,
          });
        }
      }
    },

    "chat.message": async (input, output) => {
      const sid = input.sessionID || activeSessionId;
      if (!sid) return;

      const parts = Array.isArray((output as any)?.parts) ? (output as any).parts : [];
      const text = extractTextParts(parts);
      const files = extractFileParts(parts);

      const stash = fileSet(sid);
      for (const file of files) stash.add(file);
      if (stash.size > MAX_STASHED_FILES) {
        const keep = [...stash].slice(-MAX_STASHED_FILES);
        stash.clear();
        for (const file of keep) stash.add(file);
      }

      if (!text) return;

      const capture: PromptCapture = {
        text,
        files,
        agent: safeSlice(input.agent, 60),
        model: summarizeModel(input.model),
        variant: safeSlice(input.variant, 60),
        parts: summarizeParts(parts),
      };

      pendingPrompts.set(sid, capture);
      turnPrompts.set(sid, text);
      logLine("USER_CAPTURE", {
        session: sid,
        stage: "chat.message",
        text,
        files: files.join(", "),
        agent: capture.agent,
        model: capture.model,
      });
    },

    "chat.params": async (_input, _output) => {},

    "experimental.chat.system.transform": async (input, output) => {
      const sid = input.sessionID || activeSessionId;
      if (!sid || !Array.isArray(output.system)) return;

      if (!contextInjected.has(sid)) {
        output.system.push(MEMAGENT_INSTRUCTIONS);

        const recent = await listRecentMemories();
        if (recent) {
          output.system.push(`<mem-agent-recent>\n${recent}\n</mem-agent-recent>`);
        }

        contextInjected.add(sid);
        logLine("SYSTEM_INJECT", {
          session: sid,
          stage: "initial",
          recent: recent ? "yes" : "no",
        });
      }

      const prompt = turnPrompts.get(sid)?.trim();
      if (prompt) {
        const recalled = await searchMemories(prompt, RECALL_LIMIT);
        if (recalled) {
          output.system.push(
            `<mem-agent-recall>\nQuery: ${safeSlice(prompt, 240)}\n${recalled}\n</mem-agent-recall>`,
          );
        }
        logLine("SYSTEM_INJECT", {
          session: sid,
          stage: "prompt-recall",
          query: prompt,
          recalled: recalled ? "yes" : "no",
        });
      }

      const stash = fileSet(sid);
      if (stash.size > 0) {
        const files = [...stash].slice(0, 5);
        const sections: string[] = [];

        for (const file of files) {
          const recalled = await searchMemories(file, 3);
          if (recalled) {
            sections.push(`File: ${file}\n${recalled}`);
          }
          stash.delete(file);
        }

        if (sections.length > 0) {
          output.system.push(
            `<mem-agent-file-context>\n${sections.join("\n\n")}\n</mem-agent-file-context>`,
          );
        }
        logLine("SYSTEM_INJECT", {
          session: sid,
          stage: "file-recall",
          files: files.join(", "),
          recalled: sections.length,
        });
      }
    },

    "tool.execute.before": async (input, output) => {
      if (!FILE_TOOLS.has(input.tool)) return;
      const sid = input.sessionID || activeSessionId;
      if (!sid) return;

      const args = output.args as Record<string, unknown> | undefined;
      if (!args) return;

      const stash = fileSet(sid);
      for (const file of extractFilePaths(args)) stash.add(file);
      if (stash.size > MAX_STASHED_FILES) {
        const keep = [...stash].slice(-MAX_STASHED_FILES);
        stash.clear();
        for (const file of keep) stash.add(file);
      }
      logLine("TOOL_BEFORE", {
        session: sid,
        tool: input.tool,
        files: extractFilePaths(args).join(", "),
        tracked: stash.size,
      });
    },

    config: async (_input) => {},
  };
};
