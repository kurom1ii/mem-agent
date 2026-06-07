import type { Plugin } from "@opencode-ai/plugin"

// =====================================================================
// kuromi-plugin-opencode — Hook Demo Plugin (OpenCode native runtime)
// =====================================================================
// Plugin minh họa: hook TOÀN BỘ các hook có thể trong OpenCode.
// Mỗi hook ghi 1 dòng timestamp vào file .kuromi-hooks.log trong CWD.
//
// ⚠️ OpenCode plugin chạy trong Go runtime (không phải Node.js).
//    Chỉ được dùng: $ (shell helper), ctx (project context).
//    KHÔNG được import fs, path, os, process, hay bất kỳ Node.js API nào.
//    Mọi I/O phải qua `$` chạy lệnh shell.
// =====================================================================

// Dùng echo append vào log file thay vì fs.appendFileSync
async function log(hookName: string, data: Record<string, string> = {}) {
  const ts = new Date().toISOString().slice(11, 23).replace("T", " ")
  const parts = Object.entries(data)
    .filter(([_, v]) => v !== undefined && v !== null && v !== "")
    .map(([k, v]) => `${k}=${v}`)
    .join(" ")
  const line = `[${ts}] ${hookName.padEnd(35)} │ ${parts}`
  try {
    // Dùng $ để echo append vào file log trong CWD
    await $`echo ${line} >> .kuromi-hooks.log`.quiet().nothrow()
  } catch {
    // silent fail — log không được làm crash plugin
  }
}

function safeSlice(v: unknown, max: number): string {
  if (typeof v === "string") return v.slice(0, max)
  if (v == null) return ""
  try { return JSON.stringify(v).slice(0, max) } catch { return "" }
}

const SEP = "─".repeat(60)
let sessionCount = 0

// =====================================================================
// PLUGIN ENTRY
// =====================================================================
export const KuromiPlugin: Plugin = async (ctx) => {
  const project = ctx.worktree || "."

  // Reset log file khi plugin load
  await $`rm -f .kuromi-hooks.log`.quiet().nothrow()
  await log("╔ PLUGIN_LOADED", { project })

  return {

    // ==================================================================
    // 1. EVENT HANDLER — bắt TẤT CẢ sự kiện session lifecycle
    // ==================================================================
    event: async ({ event }) => {
      const type = event.type
      const props = (event as any).properties || {}

      // ─── SESSION LIFECYCLE ─────────────────────────────

      if (type === "session.created") {
        sessionCount++
        const info = props.info || {}
        await log("🟢 session.created", {
          id: safeSlice(info.id, 12),
          title: safeSlice(info.title, 80),
          count: String(sessionCount),
        })
        await log(SEP, { msg: `SESSION #${sessionCount} STARTED` })
      }

      if (type === "session.status") {
        const s = props.status || {}
        await log("⏳ session.status", {
          type: safeSlice(s.type, 20),
          attempt: String(s.attempt ?? ""),
        })
      }

      if (type === "session.compacted") {
        await log("🗜️  session.compacted", {})
      }

      if (type === "session.updated") {
        const info = props.info || {}
        await log("📝 session.updated", { title: safeSlice(info.title, 80) })
      }

      if (type === "session.diff") {
        const diffs = Array.isArray(props.diff) ? props.diff : []
        const files = diffs.map((d: any) => d.file).slice(0, 5)
        await log("📊 session.diff", {
          count: String(diffs.length),
          files: files.join(" | "),
        })
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID
        await log("🔴 session.deleted", { id: safeSlice(sid, 12) })
        await log(SEP, { msg: "SESSION ENDED" })
      }

      if (type === "session.error") {
        await log("💥 session.error", {
          err: safeSlice(props.error, 200),
        })
      }

      // ─── MESSAGE EVENTS ────────────────────────────────

      if (type === "message.updated") {
        const info = props.info || {}
        if (info.role === "assistant") {
          await log("💬 message.updated", {
            role: "assistant",
            model: safeSlice(info.modelID, 30),
            mode: safeSlice(info.mode, 10),
            finish: safeSlice(info.finish, 10),
            cost: String(info.cost ?? ""),
          })
        }
      }

      if (type === "message.removed") {
        await log("🗑️  message.removed", {
          msgID: safeSlice(props.messageID, 12),
        })
      }

      // ─── MESSAGE PART EVENTS ───────────────────────────

      if (type === "message.part.updated") {
        const part = props.part || {}

        if (part.type === "subtask") {
          await log("🤖 part.subtask", {
            agent: safeSlice(part.agent, 20),
            desc: safeSlice(part.description, 80),
          })
        }

        if (part.type === "tool") {
          const state = part.state || {}
          const tool = safeSlice(part.tool, 15)
          const call = safeSlice(part.callID, 8)
          const status = safeSlice(state.status, 10)

          if (status === "completed") {
            const t = (state.time as any) || {}
            const dur = t.start && t.end ? `${t.end - t.start}ms` : "?"
            await log("✅ tool.completed", {
              tool,
              callID: call,
              dur,
              title: safeSlice(state.title, 60),
            })
          }
          if (status === "error") {
            await log("❌ tool.error", {
              tool,
              callID: call,
              err: safeSlice(state.error, 150),
            })
          }
          if (status === "running") {
            await log("🔧 tool.running", { tool, callID: call })
          }
        }

        if (part.type === "step-finish") {
          await log("🏁 part.step-finish", {
            reason: safeSlice(part.reason, 20),
          })
        }

        if (part.type === "reasoning") {
          await log("🧠 part.reasoning", {
            text: safeSlice((part as any).text, 100),
          })
        }

        if (part.type === "file") {
          await log("📄 part.file", {
            filename: safeSlice((part as any).filename, 60),
          })
        }

        if (part.type === "patch") {
          const pf = (part as any).files || []
          await log("🔀 part.patch", {
            files: pf.slice(0, 5).join(" | "),
          })
        }

        if (part.type === "compaction") {
          await log("🗜️  part.compaction", { auto: String((part as any).auto ?? "") })
        }

        if (part.type === "agent") {
          await log("👤 part.agent", { name: safeSlice((part as any).name, 20) })
        }

        if (part.type === "retry") {
          await log("🔄 part.retry", {
            attempt: String((part as any).attempt ?? ""),
          })
        }
      }

      // ─── FILE EVENTS ───────────────────────────────────

      if (type === "file.edited") {
        await log("✏️  file.edited", { file: safeSlice(props.file, 80) })
      }

      // ─── PERMISSION EVENTS ─────────────────────────────

      if (type === "permission.updated") {
        await log("🔐 permission.updated", {
          type: safeSlice(props.type, 15),
          pattern: safeSlice(props.pattern, 60),
        })
      }

      if (type === "permission.replied") {
        await log("🔓 permission.replied", {
          reply: safeSlice(props.response || props.reply, 20),
        })
      }

      // ─── TASK EVENTS ───────────────────────────────────

      if (type === "todo.updated") {
        const todos = Array.isArray(props.todos) ? props.todos : []
        const done = todos.filter((t: any) => t.status === "completed").length
        const active = todos.filter((t: any) => t.status !== "completed").length
        await log("✅ todo.updated", {
          total: String(todos.length),
          done: String(done),
          active: String(active),
        })
      }

      // ─── COMMAND EVENTS ────────────────────────────────

      if (type === "command.executed") {
        await log("⌨️  command.executed", {
          name: safeSlice(props.name, 20),
          args: safeSlice(props.arguments, 60),
        })
      }
    },

    // ==================================================================
    // 2. CHAT MESSAGE HOOK
    // ==================================================================
    "chat.message": async (input, _output) => {
      const parts = (_output as any)?.parts || []
      await log("💬 chat.message", {
        agent: safeSlice(input.agent, 15),
        model: safeSlice(input.model, 20),
        parts: String(parts.length),
      })
    },

    // ==================================================================
    // 3. CHAT PARAMS HOOK
    // ==================================================================
    "chat.params": async (input, _output) => {
      await log("⚙️  chat.params", {
        agent: safeSlice(input.agent, 15),
        model: safeSlice(input.model?.id || input.model, 30),
      })
    },

    // ==================================================================
    // 4. SYSTEM TRANSFORM HOOK
    // ==================================================================
    "experimental.chat.system.transform": async (input, output) => {
      if (!Array.isArray(output.system)) return
      await log("🔄 system.transform", {
        blocks: String(output.system.length),
      })
    },

    // ==================================================================
    // 5. TOOL EXECUTE BEFORE HOOK
    // ==================================================================
    "tool.execute.before": async (input, output) => {
      const args = (output as any)?.args || {}
      const keys = Object.keys(args).join(",").slice(0, 80)
      await log("⏩ tool.execute.before", {
        tool: safeSlice(input.tool, 15),
        args: keys,
      })
    },

    // ==================================================================
    // 6. CONFIG HOOK
    // ==================================================================
    config: async (input) => {
      await log("📋 config", {
        model: safeSlice(input.model?.id, 30),
        agent: safeSlice(input.agent?.id, 15),
      })
    },
  }
}
