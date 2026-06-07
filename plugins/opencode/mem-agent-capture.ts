import type { Plugin } from "@opencode-ai/plugin";

// =====================================================================
// mem-agent OpenCode Plugin — Kiến trúc Hook Engine
// =====================================================================
//
// OpenCode Plugin cung cấp 2 LOẠI hook chính:
//
// ┌─────────────────────────────────────────────────────────────┐
// │ 1. EVENT HANDLER — `event({ event })`                      │
// │    Bắt TẤT CẢ các sự kiện session lifecycle.               │
// │    event.type quyết định hook nào được kích hoạt.          │
// │                                                             │
// │ 2. CHAT HOOKS — các hàm riêng biệt:                        │
// │    • "chat.message"  → khi user gửi tin nhắn               │
// │    • "chat.params"   → khi model/tham số thay đổi          │
// │    • "tool.execute.before" → TRƯỚC KHI tool chạy           │
// │    • "experimental.chat.system.transform" → sửa system prompt│
// │    • "config"        → khi config được load                │
// └─────────────────────────────────────────────────────────────┘
//
// === LUỒNG HOẠT ĐỘNG ĐIỂN HÌNH ===
//
// 1. OpenCode khởi động → config() hook chạy
// 2. User mở session mới → event "session.created"
// 3. system.transform hook chạy → ta nhồi MEMAGENT_INSTRUCTIONS
//    vào system prompt. Từ lúc này AI BIẾT có mem-agent tools.
// 4. User chat → "chat.message" hook chạy → ta track file
// 5. AI gọi tool (vd: Write, Edit) → "tool.execute.before" chạy
//    → ta track file paths để sau này enrich context
// 6. Tool chạy xong → event "message.part.updated" type="tool"
//    → ta log kết quả, track thêm files
// 7. User đóng session → event "session.deleted"
//    → ta dọn dẹp state
// =====================================================================

// ─── Các hằng số cấu hình ───
const FILE_TOOLS = new Set(["Read", "Write", "Edit", "Glob", "Grep", "Bash"]);
const FILE_KEYS = ["filePath", "file_path", "path", "file", "pattern"];
const MAX_STASHED_FILES = 30;
const DEBUG = process.env.MEMAGENT_DEBUG === "1";

function log(...args: unknown[]): void {
  if (DEBUG) console.log("[mem-agent]", ...args);
}
function errlog(...args: unknown[]): void {
  if (DEBUG) console.error("[mem-agent]", ...args);
}

// ─── Helpers ───
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

// =====================================================================
// MEMAGENT_INSTRUCTIONS — Nhồi vào system prompt qua system.transform
// =====================================================================
//
// Đây là CỐT LÕI của plugin. Khi được inject vào system prompt,
// AI sẽ biết:
//   - Có những MCP tool nào (memory_search, memory_add, ...)
//   - Cách dùng từng tool (args, khi nào nên gọi)
//   - Best practices (gọi memory_list đầu session, memory_search
//     trước khi edit file, memory_add sau quyết định quan trọng)
//
// ⚠️ KHÁC BIỆT VỚI AGENTMEMORY:
//   Agentmemory inject CONTEXT ĐỘNG (gọi API /session/start để
//   lấy memories liên quan). Chúng ta inject STATIC INSTRUCTIONS.
//   Để có context động, cần sửa system.transform để gọi MCP tool
//   memory_list và nhồi kết quả vào đây.
// =====================================================================
const MEMAGENT_INSTRUCTIONS = `<mem-agent-instructions>
Bạn có quyền truy cập mem-agent — bộ nhớ dài hạn cho AI agent.
Hãy CHỦ ĐỘNG dùng các MCP tool dưới đây.

CÔNG CỤ CÓ SẴN (dùng đúng tên với prefix "mem-agent_"):

mem-agent_memory_search — Tìm kiếm hybrid (BM25 + vector).
  Args: query (string, bắt buộc), limit (int, mặc định 10),
        mode ("fts5"|"vector"|"hybrid")
  → Dùng khi: user hỏi "nhớ gì về...", cần context trước khi sửa file,
    muốn biết lịch sử project.

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
2. Trước khi edit file → gọi memory_search với tên file.
3. Sau quyết định quan trọng → gọi memory_add.
4. Khi user hỏi về quá khứ → LUÔN gọi memory_search trước khi trả lời.
</mem-agent-instructions>`;

// =====================================================================
// SESSION STATE — theo dõi trạng thái phiên làm việc
// =====================================================================
let activeSessionId: string | null = null;
let projectPath: string | null = null;
const trackedFiles = new Map<string, Set<string>>();   // session → files
const seenToolCallIds = new Map<string, Set<string>>(); // tránh duplicate
const contextInjected = new Set<string>();              // session đã inject

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
  if (contextInjected.size > 50) {
    const oldest = [...contextInjected].slice(0, 10);
    for (const k of oldest) contextInjected.delete(k);
  }
}

// =====================================================================
// PLUGIN ENTRY POINT
// =====================================================================
// Plugin factory — OpenCode gọi hàm này khi load plugin.
// `ctx` chứa thông tin project (worktree, project.id).
// Trả về object với các hook handler.
export const MemAgentPlugin: Plugin = async (ctx) => {
  projectPath = ctx.worktree || ctx.project?.id || process.cwd();
  log(`Plugin loaded — project: ${projectPath}`);

  return {
    // ==================================================================
    // EVENT HANDLER — Universal event dispatcher
    // ==================================================================
    // TẤT CẢ sự kiện session lifecycle đều vào đây.
    // Phân biệt bằng `event.type`. Mỗi loại có `properties` riêng.
    // Đây là hook QUAN TRỌNG NHẤT — nó bắt mọi thứ xảy ra.
    event: async ({ event }) => {
      const type = event.type;
      const props = (event as any).properties || {};

      // ─── SESSION LIFECYCLE ───────────────────────────────

      // session.created — KHI TẠO PHIÊN MỚI
      // Chạy 1 lần duy nhất lúc bắt đầu session.
      // → props.info: { id, title, parentID, version }
      // → Ta khởi tạo trackedFiles, reset state cho session mới.
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

      // session.status — KHI TRẠNG THÁI PHIÊN THAY ĐỔI
      // → props.status: { type: "idle" | "active" | ... }
      // → Khi idle, agentmemory chạy summarize. Ta chỉ log.
      if (type === "session.status") {
        const status = props.status as Record<string, unknown> | undefined;
        const sid = props.sessionID || activeSessionId;
        if (sid && status) {
          log(`Session status → ${status.type} (${sid.slice(0, 8)}...)`);
        }
      }

      // session.compacted — KHI CONTEXT BỊ NÉN
      // OpenCode tự động nén context khi quá dài.
      // → Memories đã lưu trong SQLite không bị ảnh hưởng.
      if (type === "session.compacted") {
        const sid = props.sessionID || activeSessionId;
        log(`Context compacted → session ${sid?.slice(0, 8)}`);
      }

      // session.updated — KHI METADATA PHIÊN THAY ĐỔI
      // → props.info: { title, parentID, summary }
      if (type === "session.updated") {
        const info = props.info as Record<string, unknown> | undefined;
        const sid = (info?.id as string) || props.sessionID || activeSessionId;
        if (sid) log(`Session updated: ${sid.slice(0, 8)}`);
      }

      // session.diff — KHI CÓ DIFF GIỮA CÁC LẦN CHAT
      // → props.diff: [{ file, additions, deletions }]
      // → Ta track files đã thay đổi để enrich context sau.
      if (type === "session.diff") {
        const sid = props.sessionID || activeSessionId;
        if (sid && Array.isArray(props.diff)) {
          const diffs = props.diff as Array<Record<string, unknown>>;
          for (const d of diffs) {
            if (typeof d.file === "string") fileSet(sid).add(d.file);
          }
        }
      }

      // session.deleted — KHI PHIÊN KẾT THÚC
      // → Dọn dẹp toàn bộ state của session.
      // → Agentmemory chạy summarize + consolidate ở đây.
      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID || activeSessionId;
        if (sid) {
          log(`Session ended → cleaning up ${sid}`);
          trackedFiles.delete(sid);
          seenToolCallIds.delete(sid);
          contextInjected.delete(sid);
          if (sid === activeSessionId) activeSessionId = null;
          pruneMaps();
        }
      }

      // session.error — KHI PHIÊN GẶP LỖI
      if (type === "session.error") {
        const sid = props.sessionID || activeSessionId;
        if (sid) errlog(`Session error: ${safeSlice(props.error, 500)}`);
      }

      // ─── MESSAGE EVENTS ──────────────────────────────────

      // message.updated — KHI TIN NHẮN ĐƯỢC CẬP NHẬT
      // → props.info.role: "user" | "assistant"
      // → Agentmemory dùng để capture cả user prompt và AI response.
      // → Ta chỉ log để debug.
      if (type === "message.updated") {
        const info = props.info as Record<string, unknown> | undefined;
        if (info?.role === "assistant") {
          log(`Assistant reply → id=${(info.id as string)?.slice(0, 8)}`);
        }
      }

      // message.removed — KHI TIN NHẮN BỊ XÓA KHỎI CONTEXT
      if (type === "message.removed") {
        log(`Message removed: ${props.messageID}`);
      }

      // message.part.updated — KHI 1 PHẦN CỦA TIN NHẮN THAY ĐỔI
      // Đây là hook CHI TIẾT NHẤT. Mỗi message có nhiều "parts":
      //
      //   part.type = "tool"       → AI gọi tool (Write, Bash, Grep...)
      //     part.state.status: "running" → "completed" | "error"
      //     → Ta capture input/output của tool, track files
      //
      //   part.type = "subtask"    → AI spawn subagent
      //   part.type = "step-finish"→ AI kết thúc 1 bước reasoning
      //   part.type = "reasoning"  → AI đang suy nghĩ (thinking)
      //   part.type = "file"       → AI tạo/reference file
      //   part.type = "patch"      → AI đề xuất thay đổi code
      //   part.type = "compaction" → Context bị nén
      //   part.type = "agent"      → AI chọn agent khác
      //   part.type = "retry"      → AI thử lại
      //
      if (type === "message.part.updated") {
        const part = props.part as Record<string, unknown> | undefined;
        if (!part) return;
        const sid = (part.sessionID as string) || props.sessionID || activeSessionId;
        if (!sid) return;

        // ── SUBTASK: AI spawn subagent ──
        if (part.type === "subtask") {
          log(`🤖 Subagent started: ${part.id} (agent=${part.agent})`);
          return;
        }

        // ── TOOL: AI gọi tool (Write, Edit, Bash, Grep...) ──
        if (part.type === "tool") {
          const state = part.state as Record<string, unknown> | undefined;
          if (!state) return;
          const callId = part.callID as string;
          if (!callId) return;
          const toolName = part.tool as string;

          // Tool completed — capture input/output
          if (state.status === "completed") {
            const callSet = toolCallSet(sid);
            if (callSet.has(callId)) return; // tránh duplicate
            callSet.add(callId);
            log(`✅ Tool done: ${toolName} (${callId.slice(0, 8)})`);

            // Track file paths từ tool arguments
            if (FILE_TOOLS.has(toolName) && state.input) {
              const args = state.input as Record<string, unknown>;
              for (const fp of extractFilePaths(args)) {
                fileSet(sid).add(fp);
              }
            }
          }

          // Tool failed — log lỗi
          if (state.status === "error") {
            const callSet = toolCallSet(sid);
            if (callSet.has(callId)) return;
            callSet.add(callId);
            errlog(`❌ Tool failed: ${toolName} → ${safeSlice(state.error, 200)}`);
          }
          return;
        }

        // ── STEP-FINISH: AI kết thúc 1 bước ──
        if (part.type === "step-finish") {
          log(`Step finished → reason: ${part.reason}`);
        }

        // ── REASONING: AI đang suy nghĩ ──
        if (part.type === "reasoning") {
          log(`Thinking... ${safeSlice((part as any).text, 100)}`);
        }

        // ── FILE: file được tạo/tham chiếu ──
        if (part.type === "file") {
          const filename = (part as any).filename || (part as any).url;
          if (filename) fileSet(sid).add(filename);
        }

        // ── PATCH: thay đổi code ──
        if (part.type === "patch") {
          const pf = (part as any).files || [];
          for (const f of pf) fileSet(sid).add(f);
        }
      }

      // ─── FILE EVENTS ─────────────────────────────────────

      // file.edited — KHI FILE ĐƯỢC CHỈNH SỬA (ngoài tool Write/Edit)
      // → props.file: đường dẫn file
      // → Ta thêm vào tracked set để sau enrich context
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
          log(`File edited: ${props.file} (${stash.size} files tracked)`);
        }
      }

      // ─── PERMISSION EVENTS ───────────────────────────────

      // permission.updated — KHI OPEnCODE HỎI QUYỀN
      // → props.type: loại permission (tool, file, command...)
      // → props.pattern: pattern cần quyền
      if (type === "permission.updated") {
        const sid = props.sessionID || activeSessionId;
        if (sid) log(`Permission asked: ${props.type} → ${props.pattern}`);
      }

      // permission.replied — KHI USER TRẢ LỜI CHO PHÉP/TỪ CHỐI
      if (type === "permission.replied") {
        log(`Permission reply: ${props.response || props.reply}`);
      }

      // ─── TASK EVENTS ─────────────────────────────────────

      // todo.updated — KHI TODO LIST THAY ĐỔI
      // → props.todos: [{ content, status, priority }]
      // → Agentmemory capture để biết task nào đã hoàn thành
      if (type === "todo.updated") {
        const todos = Array.isArray(props.todos) ? props.todos : [];
        const completed = todos.filter((t: any) => t.status === "completed");
        const active = todos.filter((t: any) => t.status !== "completed");
        if (completed.length > 0) {
          log(`Tasks: ${completed.length} done, ${active.length} remaining`);
        }
      }

      // ─── COMMAND EVENTS ──────────────────────────────────

      // command.executed — KHI USER CHẠY SLASH COMMAND
      // → props.name: tên command (/remember, /recall...)
      // → props.arguments: tham số
      if (type === "command.executed") {
        log(`Command: /${props.name}`);
      }
    },

    // ==================================================================
    // CHAT MESSAGE HOOK
    // ==================================================================
    // Chạy khi user gửi tin nhắn HOẶC AI trả lời.
    // input: { sessionID, agent, model, variant }
    // output: { parts: [...] } — các phần của tin nhắn
    // → Agentmemory dùng để capture user prompt + AI response.
    // → Ta chỉ log để debug, không capture nội dung chat.
    "chat.message": async (input, _output) => {
      const sid = input.sessionID || activeSessionId;
      if (sid) log(`💬 Chat → session ${sid.slice(0, 8)}`);
    },

    // ==================================================================
    // CHAT PARAMS HOOK
    // ==================================================================
    // Chạy khi tham số chat thay đổi (model, temperature, ...)
    // → Agentmemory capture để biết model nào đang dùng.
    "chat.params": async (input, _output) => {
      if (input.model) {
        log(`Model: ${input.model.providerID}/${input.model.id}`);
      }
    },

    // ==================================================================
    // SYSTEM TRANSFORM HOOK — QUAN TRỌNG NHẤT
    // ==================================================================
    // Chạy TRƯỚC KHI system prompt được gửi cho AI.
    // → output.system: mảng string, ta có thể push thêm context.
    //
    // ĐÂY LÀ NƠI TA INJECT MEMAGENT INSTRUCTIONS:
    // 1. Kiểm tra session đã được inject chưa (tránh duplicate)
    // 2. Push MEMAGENT_INSTRUCTIONS → AI biết có memory tools
    // 3. Push tracked files context → AI biết files nào đang active
    // 4. Đánh dấu session đã inject
    //
    // ⚠️ CẢI TIẾN TƯƠNG LAI:
    // Có thể gọi MCP tool memory_list ở đây, lấy kết quả,
    // và inject vào system prompt để AI thấy context THỰC TẾ.
    // Hiện tại chỉ inject STATIC instructions.
    "experimental.chat.system.transform": async (input, output) => {
      const sid = input.sessionID || activeSessionId;
      if (!sid || !Array.isArray(output.system)) return;

      // Chỉ inject 1 lần mỗi session
      if (!contextInjected.has(sid)) {
        // (1) Nhồi instructions — AI biết tool nào có sẵn
        output.system.push(MEMAGENT_INSTRUCTIONS);

        // (2) Nhồi context về files đang được track
        const stash = fileSet(sid);
        if (stash.size > 0) {
          const files = [...stash].slice(0, 10);
          output.system.push(
            `\n<mem-agent-context>\n` +
            `Files đang active: ${files.join(", ")}\n` +
            `Dùng mem-agent_memory_search để kiểm tra context cũ\n` +
            `của các file này TRƯỚC KHI chỉnh sửa.\n` +
            `</mem-agent-context>\n`
          );
        }

        contextInjected.add(sid);
        pruneMaps();
        log(`Context injected → session ${sid.slice(0, 8)} (${stash.size} files)`);
      }
    },

    // ==================================================================
    // TOOL EXECUTE BEFORE HOOK
    // ==================================================================
    // Chạy TRƯỚC KHI AI thực thi 1 tool.
    // → Chỉ hook cho FILE_TOOLS (Write, Edit, Read, Grep, ...)
    // → Ta extract file paths từ args để track.
    // → Mục đích: biết AI đang làm việc với file nào để
    //   sau này enrich context với memory liên quan.
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

    // ==================================================================
    // CONFIG HOOK
    // ==================================================================
    // Chạy 1 lần khi OpenCode load config (model, theme, agents, ...)
    // → Agentmemory capture để biết user dùng model gì, theme gì.
    config: async (input) => {
      log(`Config: model=${input.model?.id}, agent=${input.agent?.id}`);
    },
  };
};
