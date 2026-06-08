import { spawn } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";

const LOG_DIR = path.resolve(new URL("..", import.meta.url).pathname);
const LOG_PATH = path.join(LOG_DIR, "log.log");

function ensureLogDir() {
  try { mkdirSync(LOG_DIR, { recursive: true }); } catch {}
}

const ANSI = { reset: "\x1b[0m", dim: "\x1b[2m", bold: "\x1b[1m", gray: "\x1b[90m", red: "\x1b[91m", green: "\x1b[92m", yellow: "\x1b[93m", blue: "\x1b[94m", cyan: "\x1b[96m" } as const;

const COLOR = {
  save_ok: ANSI.green + ANSI.bold,
  save_err: ANSI.red + ANSI.bold,
  recall_ok: ANSI.cyan + ANSI.bold,
  recall_empty: ANSI.gray,
  recall_err: ANSI.red,
  recent: ANSI.blue,
  inject: ANSI.yellow + ANSI.bold,
} as const;

const LABEL_WIDTH = 14;

function nowStamp(): string {
  const d = new Date();
  return `${String(d.getHours()).padStart(2,"0")}:${String(d.getMinutes()).padStart(2,"0")}:${String(d.getSeconds()).padStart(2,"0")}.${String(d.getMilliseconds()).padStart(3,"0")}`;
}

function memLog(label: string, color: string, detail: string): void {
  ensureLogDir();
  const tag = label.padEnd(LABEL_WIDTH, " ");
  const line = `${ANSI.gray}[${nowStamp()}]${ANSI.reset} ${color}${tag}${ANSI.reset} ${ANSI.dim}│${ANSI.reset} ${detail}`;
  appendFileSync(LOG_PATH, `${line}\n`);
}

export { memLog };

export type PromptCapture = { text: string; files: string[]; agent: string; model: string; variant: string; parts: string[] };
export type RunResult = { ok: boolean; stdout: string; stderr: string; code: number | null };
export type CacheEntry = { value: string; fetchedAt: number };

export const FILE_TOOLS = new Set(["Read", "Write", "Edit", "Glob", "Grep", "Bash"]);
export const FILE_KEYS = ["filePath", "file_path", "path", "file", "pattern"];
export const MAX_STASHED_FILES = 30;
export const MAX_MESSAGE_CONTENT = 8000;
export const MAX_TOOL_CONTENT = 16000;
export const RECENT_MEMORY_LIMIT = 8;
export const RECALL_LIMIT = 5;
export const CACHE_TTL_MS = 30_000;
export const AUTO_SAVE = process.env.MEMAGENT_AUTO_SAVE !== "0";
export const AUTO_RECALL = process.env.MEMAGENT_AUTO_RECALL !== "0";

export const MEMAGENT_INSTRUCTIONS = `<mem-agent-instructions>
Bạn có quyền truy cập mem-agent — bộ nhớ dài hạn cho AI agent.
Bạn PHẢI chủ động dùng memory, không đợi user nhắc.

CÔNG CỤ CÓ SẴN (dùng đúng tên với prefix "mem-agent_"):

mem-agent_memory_search — Tìm kiếm memory đã lưu. Args: query (string), limit (int, mặc định 10), mode ("fts5"|"vector"|"hybrid"), scope ("auto"|"chat"|"tool"|"all")
mem-agent_memory_add — Lưu memory mới. Args: title, content, tags, kind, type, session_id, source, project
mem-agent_memory_list — Danh sách memory gần đây. Args: limit (int, mặc định 20), scope ("all"|"chat"|"tool")
mem-agent_memory_get — Lấy chi tiết 1 memory. Args: id (int)
mem-agent_memory_delete — Xóa memory. Args: id (int)
mem-agent_memory_observe — Phân loại tool execution qua observer pipeline (DeepSeek v4-flash). Args: tool_name, tool_output, tool_input, session_id, source, cwd, user_prompt, tags
mem-agent_memory_summarize — Tóm tắt session. Args: session_id
mem-agent_index_stats — Thống kê bộ nhớ.

QUY TẮC:
1. Đầu session → đọc memory gần đây. Nếu injected chưa đủ, gọi memory_list scope="chat".
2. Mỗi turn: user hỏi quá khứ/preference/task cũ → LUÔN memory_search trước khi trả lời.
3. Trước khi edit file → memory_search với tên file/module/feature liên quan.
4. Sau fix bug/quyết định/convention quan trọng → memory_add đúng 1 lần.
5. Không hỏi user nhắc lại nếu chưa kiểm tra memory.
6. Injected memory đã đủ thì dùng luôn, không gọi tool thừa.
</mem-agent-instructions>`;

export function normalizeTag(tag: string): string { return tag.toLowerCase().replace(/[^a-z0-9._/-]+/g,"-").replace(/^-+|-+$/g,""); }
export function joinTags(tags: string[]): string { return [...new Set(tags.map(normalizeTag).filter(Boolean))].join(","); }
export function safeSlice(v: unknown, max: number): string { if (typeof v === "string") return v.slice(0,max); if (v==null) return ""; try{return JSON.stringify(v).slice(0,max)}catch{return""}; }
export function serializePayload(v: unknown, max: number): string { if (typeof v==="string") return v.slice(0,max); if (v==null) return ""; try{return JSON.stringify(v,null,2).slice(0,max)}catch{return safeSlice(v,max)}; }
export function oneLine(v: unknown, max: number): string { return safeSlice(v,max).replace(/\s+/g," ").trim(); }
export function summarizeModel(model: unknown): string { if (typeof model==="string") return model; if (!model||typeof model!=="object") return ""; const m=model as Record<string,unknown>; return [typeof m.providerID==="string"?m.providerID:"", typeof m.id==="string"?m.id:""].filter(Boolean).join("/"); }
export function extractFilePaths(args: Record<string,unknown>): string[] { const f:string[]=[]; for(const k of FILE_KEYS){const v=args[k]; if(typeof v==="string"&&v.length>0) f.push(v);} return f; }
export function extractTextParts(parts: unknown[]): string { return parts.filter(p=>p&&typeof p==="object").map(p=>p as Record<string,unknown>).filter(p=>p.type==="text"&&!p.synthetic&&!p.ignored).map(p=>typeof p.text==="string"?p.text:"").filter(Boolean).join("\n").trim(); }
export function extractFileParts(parts: unknown[]): string[] { return parts.filter(p=>p&&typeof p==="object").map(p=>p as Record<string,unknown>).filter(p=>p.type==="file").map(p=>{if(typeof p.filename==="string")return p.filename; if(typeof p.url==="string")return p.url; return"";}).filter(Boolean); }
export function summarizeParts(parts: unknown[]): string[] { return parts.filter(p=>p&&typeof p==="object").map(p=>{const pp=p as Record<string,unknown>; return typeof pp.type==="string"?pp.type:"";}).filter(Boolean); }
export function makeTitle(prefix: string, text: string): string { return oneLine(`${prefix}: ${oneLine(text,64)||"event"}`,80); }

const REPO_ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..", "..", "..");

export function resolveMemAgentBin(): string {
  const envBin = process.env.MEMAGENT_BIN?.trim(); if (envBin) return envBin;
  const releaseBin = path.resolve(REPO_ROOT, "target", "release", "mem-agent"); if (existsSync(releaseBin)) return releaseBin;
  const debugBin = path.resolve(REPO_ROOT, "target", "debug", "mem-agent"); if (existsSync(debugBin)) return debugBin;
  return "mem-agent";
}
export function resolveDbPath(): string { return process.env.MEMAGENT_DB?.trim() || path.resolve(REPO_ROOT, "memories.db"); }

export async function runMemAgent(args: string[], timeoutMs = 8000): Promise<RunResult> {
  return new Promise((resolve) => {
    const child = spawn(resolveMemAgentBin(), ["--db", resolveDbPath(), ...args], { env: process.env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "", stderr = "", settled = false;
    const finish = (result: RunResult) => { if (settled) return; settled = true; clearTimeout(timer); resolve(result); };
    const timer = setTimeout(() => { child.kill("SIGKILL"); finish({ ok: false, stdout, stderr: stderr || `Timed out after ${timeoutMs}ms`, code: null }); }, timeoutMs);
    child.stdout.on("data", (data) => { stdout += data.toString(); });
    child.stderr.on("data", (data) => { stderr += data.toString(); });
    child.on("error", (error) => finish({ ok: false, stdout, stderr: error.message, code: null }));
    child.on("close", (code) => finish({ ok: code === 0, stdout: stdout.trim(), stderr: stderr.trim(), code }));
  });
}

export async function addMemory(title: string, content: string, tags: string[], opts?: { kind?: string; obsType?: string; sessionId?: string; source?: string; project?: string }): Promise<void> {
  if (!AUTO_SAVE) return;
  const ct = oneLine(title, 80), cc = content.trim();
  if (!ct || !cc) return;
  const args = ["add", ct, cc, "--tags", joinTags(tags)];
  if (opts?.kind) args.push("--kind", opts.kind);
  if (opts?.obsType) args.push("--obs-type", opts.obsType);
  if (opts?.sessionId) args.push("--session-id", opts.sessionId);
  if (opts?.source) args.push("--source", opts.source);
  if (opts?.project) args.push("--project", opts.project);
  const result = await runMemAgent(args, 60000);
  if (!result.ok) memLog("SAVE", COLOR.save_err, `${ct} — ${result.stderr || result.stdout}`);
  else memLog("SAVE", COLOR.save_ok, `${ct} [${joinTags(tags)}]`);
}

export async function searchMemories(query: string, limit = RECALL_LIMIT): Promise<string> {
  const cq = safeSlice(query, 500).trim();
  if (!AUTO_RECALL || !cq) return "";
  const result = await runMemAgent(["search", cq, "--mode", "hybrid", "--scope", "auto", "-n", String(limit)], 30000);
  if (!result.ok || !result.stdout || result.stdout.includes("No results found")) { if (!result.ok) memLog("RECALL", COLOR.recall_err, cq); return ""; }
  memLog("RECALL", COLOR.recall_ok, `${cq} → ${result.stdout.split("\n").length} lines`);
  return result.stdout;
}

export async function listRecentMemories(limit = RECENT_MEMORY_LIMIT): Promise<string> {
  const result = await runMemAgent(["list", "--scope", "chat", "-n", String(limit)], 8000);
  if (!result.ok || !result.stdout || result.stdout.includes("No memories stored")) return "";
  memLog("RECENT", COLOR.recent, `loaded ${result.stdout.split("\n").length} items`);
  return result.stdout;
}

const backgroundTasks = new Set<Promise<unknown>>();
const recentCache = new Map<number, CacheEntry>();
const recallCache = new Map<string, CacheEntry>();
const pendingCacheJobs = new Map<string, Promise<void>>();
const recentInjected = new Set<string>();

function isFresh(entry: CacheEntry | undefined): boolean { return !!entry && Date.now() - entry.fetchedAt < CACHE_TTL_MS; }
function trackBackground(label: string, task: () => Promise<void>): void {
  const job = task().catch((error) => memLog("ERR", COLOR.save_err, `${label}: ${error instanceof Error ? error.message : String(error)}`)).finally(() => { backgroundTasks.delete(job); });
  backgroundTasks.add(job);
}
export function invalidateRecentCache(): void { recentCache.clear(); }
export function getCachedRecent(limit = RECENT_MEMORY_LIMIT): string { const e = recentCache.get(limit); return isFresh(e) ? e!.value : ""; }
function recallCacheKey(query: string, limit: number): string { return `${limit}:${query}`; }
export function getCachedRecall(query: string, limit: number): string { const e = recallCache.get(recallCacheKey(query, limit)); return isFresh(e) ? e!.value : ""; }

export function scheduleRecentRefresh(limit = RECENT_MEMORY_LIMIT): void {
  const key = `recent:${limit}`;
  if (pendingCacheJobs.has(key) || isFresh(recentCache.get(limit))) return;
  const job = (async () => { const v = await listRecentMemories(limit); recentCache.set(limit, { value: v, fetchedAt: Date.now() }); })().finally(() => { pendingCacheJobs.delete(key); });
  pendingCacheJobs.set(key, job); trackBackground(key, () => job);
}

export function scheduleRecallRefresh(query: string, limit: number): void {
  const cq = safeSlice(query, 500).trim(); if (!cq) return;
  const key = `recall:${recallCacheKey(cq, limit)}`;
  if (pendingCacheJobs.has(key) || isFresh(recallCache.get(recallCacheKey(cq, limit)))) return;
  const job = (async () => { const v = await searchMemories(cq, limit); recallCache.set(recallCacheKey(cq, limit), { value: v, fetchedAt: Date.now() }); })().finally(() => { pendingCacheJobs.delete(key); });
  pendingCacheJobs.set(key, job); trackBackground(key, () => job);
}

export function enqueueMemorySave(title: string, content: string, tags: string[], opts?: { kind?: string; obsType?: string; sessionId?: string; source?: string; project?: string }): void {
  trackBackground(`save:${title}`, async () => { await addMemory(title, content, tags, opts); invalidateRecentCache(); scheduleRecentRefresh(); });
}

export function sessionCleanup(sid: string, state: SessionState): void { state.trackedFiles.delete(sid); state.seenToolCallIds.delete(sid); state.seenMessageIds.delete(sid); state.contextInjected.delete(sid); state.recentInjected.delete(sid); state.turnPrompts.delete(sid); }

export interface SessionState { activeSessionId: string | null; projectPath: string; trackedFiles: Map<string, Set<string>>; seenToolCallIds: Map<string, Set<string>>; seenMessageIds: Map<string, Set<string>>; contextInjected: Set<string>; recentInjected: Set<string>; turnPrompts: Map<string, string>; }
export function createSessionState(projectPath: string): SessionState { return { activeSessionId: null, projectPath, trackedFiles: new Map(), seenToolCallIds: new Map(), seenMessageIds: new Map(), contextInjected: new Set(), recentInjected: new Set(), turnPrompts: new Map() }; }
export function fileSet(state: SessionState, sid: string): Set<string> { let s = state.trackedFiles.get(sid); if (!s) { s = new Set(); state.trackedFiles.set(sid, s); } return s; }
export function toolCallSet(state: SessionState, sid: string): Set<string> { let s = state.seenToolCallIds.get(sid); if (!s) { s = new Set(); state.seenToolCallIds.set(sid, s); } return s; }
export function messageIdSet(state: SessionState, sid: string): Set<string> { let s = state.seenMessageIds.get(sid); if (!s) { s = new Set(); state.seenMessageIds.set(sid, s); } return s; }
