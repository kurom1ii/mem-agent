import type { Plugin } from "@opencode-ai/plugin";
import * as fs from "fs";
import * as path from "path";

// =====================================================================
// kuromi-plugin-opencode — Hook Demo Plugin
// =====================================================================
// Plugin minh họa: hook TOÀN BỘ các hook có thể trong OpenCode.
// Mỗi hook ghi 1 dòng log vào file .kuromi-hooks.log
// trong WORKING DIRECTORY của project hiện tại.
//
// KHÔNG làm gì khác ngoài in log. Dùng để kiểm tra hook nào
// thực sự được kích hoạt trong từng tình huống.
// =====================================================================

const LOG_FILE = path.join(process.cwd(), ".kuromi-hooks.log");
const MAX_LOG_SIZE = 5 * 1024 * 1024; // 5MB
const SEPARATOR = "─".repeat(80);

function ensureLogFile(): void {
  const dir = path.dirname(LOG_FILE);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

  // Reset log file on each plugin load
  // Xóa log cũ, ghi header mới
  fs.writeFileSync(LOG_FILE, "");
}

/**
 * Ghi 1 dòng timestamp vào log file.
 * Format: [HH:MM:SS.mmm] HOOK_NAME │ key1=val1 key2=val2 ...
 */
function hookLog(hookName: string, data: Record<string, unknown> = {}): void {
  const now = new Date();
  const ts = now.toISOString().slice(11, 23).replace("T", " "); // HH:MM:SS.mmm

  const parts = Object.entries(data)
    .filter(([_, v]) => v !== undefined && v !== null && v !== "")
    .map(([k, v]) => {
      const val = typeof v === "object" ? JSON.stringify(v).slice(0, 200) : String(v);
      return `${k}=${val}`;
    })
    .join(" ");

  const line = `[${ts}] ${hookName.padEnd(35)} │ ${parts}\n`;

  try {
    fs.appendFileSync(LOG_FILE, line);

    // Rotate nếu file quá to
    const stat = fs.statSync(LOG_FILE);
    if (stat.size > MAX_LOG_SIZE) {
      fs.writeFileSync(LOG_FILE, "");
    }
  } catch {
    // silent fail — không để lỗi log làm crash plugin
  }
}

// =====================================================================
// PLUGIN ENTRY
// =====================================================================
export const KuromiPlugin: Plugin = async (ctx) => {
  ensureLogFile();

  hookLog("╔ PLUGIN_LOADED", {
    project: ctx.worktree || ctx.project?.id || process.cwd(),
    pid: process.pid,
    node: process.version,
  });

  return {
    // ==================================================================
    // 1. CONFIG — chạy 1 lần khi load config
    // ==================================================================
    config: async (input) => {
      hookLog("📋 config", {
        model_id: input.model?.id,
        agent_id: input.agent?.id,
        theme: input.theme,
        autoupdate: input.autoupdate,
      });
    },

    // ==================================================================
    // 2. EVENT HANDLER — Universal dispatcher, bắt TẤT CẢ sự kiện
    // ==================================================================
    event: async ({ event }) => {
      const type = event.type;
      const props = (event as any).properties || {};
      const ts = Date.now();

      // ─── SESSION LIFECYCLE ───

      if (type === "session.created") {
        const info = props.info || {};
        hookLog("🟢 session.created", {
          id: (info.id as string)?.slice(0, 12),
          title: info.title as string,
          parentID: (info.parentID as string)?.slice(0, 12),
        });
        hookLog(SEPARATOR, { msg: "NEW SESSION STARTED" });
      }

      if (type === "session.status") {
        const status = props.status || {};
        hookLog("⏳ session.status", {
          type: status.type as string,
          attempt: status.attempt,
          message: (status.message as string)?.slice(0, 100),
        });
      }

      if (type === "session.compacted") {
        hookLog("🗜️  session.compacted", {});
      }

      if (type === "session.updated") {
        const info = props.info || {};
        hookLog("📝 session.updated", {
          id: (info.id as string)?.slice(0, 12),
          title: info.title as string,
        });
      }

      if (type === "session.diff") {
        const diffs = Array.isArray(props.diff) ? props.diff : [];
        const files = diffs.map((d: any) => d.file).slice(0, 5);
        hookLog("📊 session.diff", {
          fileCount: diffs.length,
          files: files.join(" | "),
        });
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID;
        hookLog("🔴 session.deleted", {
          id: (sid as string)?.slice(0, 12),
        });
        hookLog(SEPARATOR, { msg: "SESSION ENDED" });
      }

      if (type === "session.error") {
        hookLog("💥 session.error", {
          error: String(props.error || "").slice(0, 200),
        });
      }

      // ─── MESSAGE EVENTS ───

      if (type === "message.updated") {
        const info = props.info || {};
        hookLog("💬 message.updated", {
          id: (info.id as string)?.slice(0, 12),
          role: info.role as string,
          model: info.modelID as string,
          mode: info.mode as string,
          finish: info.finish as string,
          cost: info.cost as number,
        });
      }

      if (type === "message.removed") {
        hookLog("🗑️  message.removed", {
          messageID: (props.messageID as string)?.slice(0, 12),
        });
      }

      // ─── MESSAGE PART EVENTS — CHI TIẾT NHẤT ───

      if (type === "message.part.updated") {
        const part = props.part || {};

        if (part.type === "subtask") {
          hookLog("🤖 part.subtask", {
            subtask_id: (part.id as string)?.slice(0, 12),
            agent: part.agent as string,
            description: (part.description as string)?.slice(0, 100),
            prompt: (part.prompt as string)?.slice(0, 100),
          });
        }

        if (part.type === "tool") {
          const state = part.state || {};
          const toolName = part.tool as string;
          const callId = (part.callID as string)?.slice(0, 8);
          const status = state.status as string;

          if (status === "running") {
            hookLog("🔧 tool.running", {
              tool: toolName,
              callID: callId,
            });
          }

          if (status === "completed") {
            const rawTime = (state.time as any) || {};
            const dur =
              rawTime.start && rawTime.end
                ? `${rawTime.end - rawTime.start}ms`
                : "?";
            hookLog("✅ tool.completed", {
              tool: toolName,
              callID: callId,
              duration: dur,
              title: (state.title as string)?.slice(0, 80),
              input: JSON.stringify(state.input || {}).slice(0, 150),
              output: JSON.stringify(state.output || {}).slice(0, 150),
            });
          }

          if (status === "error") {
            hookLog("❌ tool.error", {
              tool: toolName,
              callID: callId,
              error: String(state.error || "").slice(0, 200),
            });
          }
        }

        if (part.type === "step-finish") {
          hookLog("🏁 part.step-finish", {
            reason: part.reason as string,
            cost: (part as any).cost,
          });
        }

        if (part.type === "reasoning") {
          hookLog("🧠 part.reasoning", {
            text: ((part as any).text as string)?.slice(0, 150),
          });
        }

        if (part.type === "file") {
          hookLog("📄 part.file", {
            filename: (part as any).filename || (part as any).url,
          });
        }

        if (part.type === "patch") {
          hookLog("🔀 part.patch", {
            hash: (part as any).hash,
            files: ((part as any).files || []).join(" | "),
          });
        }

        if (part.type === "compaction") {
          hookLog("🗜️  part.compaction", {
            auto: (part as any).auto,
          });
        }

        if (part.type === "agent") {
          hookLog("👤 part.agent", {
            name: (part as any).name,
          });
        }

        if (part.type === "retry") {
          hookLog("🔄 part.retry", {
            attempt: (part as any).attempt,
            error: String((part as any).error || "").slice(0, 150),
          });
        }
      }

      // ─── FILE EVENTS ───

      if (type === "file.edited") {
        hookLog("✏️  file.edited", {
          file: props.file as string,
        });
      }

      // ─── PERMISSION EVENTS ───

      if (type === "permission.updated") {
        hookLog("🔐 permission.updated", {
          type: props.type as string,
          pattern: Array.isArray(props.pattern)
            ? props.pattern.join(", ")
            : String(props.pattern || ""),
          tool_call_id: (props.callID as string)?.slice(0, 8),
          title: props.title as string,
        });
      }

      if (type === "permission.replied") {
        hookLog("🔓 permission.replied", {
          permission_id: (props.permissionID || props.requestID || "") as string,
          response: (props.response || props.reply || "") as string,
        });
      }

      // ─── TASK EVENTS ───

      if (type === "todo.updated") {
        const todos = Array.isArray(props.todos) ? props.todos : [];
        const done = todos.filter((t: any) => t.status === "completed").length;
        const active = todos.filter((t: any) => t.status !== "completed").length;
        hookLog("✅ todo.updated", {
          total: todos.length,
          completed: done,
          inProgress: active,
        });
      }

      // ─── COMMAND EVENTS ───

      if (type === "command.executed") {
        hookLog("⌨️  command.executed", {
          name: props.name as string,
          arguments: (props.arguments || "") as string,
        });
      }
    },

    // ==================================================================
    // 3. CHAT MESSAGE — user gửi hoặc AI trả lời
    // ==================================================================
    "chat.message": async (input, output) => {
      const parts = output.parts || [];
      const types = parts
        .map((p: any) => p.type)
        .filter(Boolean)
        .join("+");
      hookLog("💬 chat.message", {
        agent: input.agent,
        model: input.model,
        parts: types || "empty",
        partCount: parts.length,
      });
    },

    // ==================================================================
    // 4. CHAT PARAMS — tham số model thay đổi
    // ==================================================================
    "chat.params": async (input, output) => {
      hookLog("⚙️  chat.params", {
        agent: input.agent,
        model: input.model ? `${input.model.providerID}/${input.model.id}` : "?",
        temperature: output?.temperature,
        topP: output?.topP,
        maxTokens: input.model?.limit?.output,
      });
    },

    // ==================================================================
    // 5. SYSTEM TRANSFORM — sửa system prompt trước khi gửi AI
    // ==================================================================
    "experimental.chat.system.transform": async (input, output) => {
      if (!Array.isArray(output.system)) return;

      // Demo: thêm 1 dòng nhỏ vào system prompt để xác nhận plugin hoạt động
      const marker = "\n<!-- kuromi-plugin-opencode: demo hook active -->\n";
      if (!output.system.some((s: string) => s.includes("kuromi-plugin-opencode"))) {
        output.system.push(marker);
      }

      hookLog("🔄 system.transform", {
        systemBlocks: output.system.length,
        injected: marker.length,
      });
    },

    // ==================================================================
    // 6. TOOL EXECUTE BEFORE — TRƯỚC KHI tool chạy
    // ==================================================================
    "tool.execute.before": async (input, output) => {
      const args = output.args || {};
      const argKeys = Object.keys(args).join(",");
      hookLog("⏩ tool.execute.before", {
        tool: input.tool,
        args: argKeys.slice(0, 100),
      });
    },

    // ==================================================================
    // 7. EXPERIMENTAL SESSION COMPACTING
    // ==================================================================
    "experimental.session.compacting": async (input, output) => {
      hookLog("🗜️  session.compacting", {
        contextBlocks: Array.isArray(output?.context)
          ? output.context.length
          : 0,
      });
    },
  };
};
