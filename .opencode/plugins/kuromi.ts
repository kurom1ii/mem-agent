/// <reference types="bun" />

import type { Plugin } from "@opencode-ai/plugin"

// Kuromi hook demo — chỉ dùng console.error cho server plugin
// Server plugin KHÔNG có $ (Bun shell helper)

function safeSlice(v: unknown, max: number): string {
  if (typeof v === "string") return v.slice(0, max)
  if (v == null) return ""
  try { return JSON.stringify(v).slice(0, max) } catch { return "" }
}

const SEP = "─".repeat(60)
let sessionCount = 0
let sessionId = ""

export const KuromiPlugin: Plugin = async (ctx) => {
  const project = ctx.worktree || "?"
  console.error(`[KUROMI] ╔ PLUGIN_LOADED │ project=${project} │ pid=${process?.pid}`)

  return {
    // ==================================================================
    // EVENT HANDLER — bắt TẤT CẢ sự kiện session lifecycle
    // ==================================================================
    event: async ({ event }) => {
      const type = event.type
      const props = (event as any).properties || {}

      // ─── SESSION LIFECYCLE ───
      if (type === "session.created") {
        sessionCount++
        const info = props.info || {}
        sessionId = safeSlice(info.id, 20)
        console.error(`[KUROMI] 🟢 session.created │ id=${sessionId} title=${safeSlice(info.title, 80)} count=${sessionCount}`)
        console.error(`[KUROMI] ${SEP}`)
      }

      if (type === "session.status") {
        const s = props.status || {}
        console.error(`[KUROMI] ⏳ session.status │ type=${safeSlice(s.type, 20)} attempt=${s.attempt ?? ""}`)
      }

      if (type === "session.compacted") {
        console.error(`[KUROMI] 🗜️  session.compacted`)
      }

      if (type === "session.updated") {
        const info = props.info || {}
        console.error(`[KUROMI] 📝 session.updated │ title=${safeSlice(info.title, 80)}`)
      }

      if (type === "session.diff") {
        const diffs = Array.isArray(props.diff) ? props.diff : []
        const files = diffs.map((d: any) => d.file).slice(0, 5).join(" | ")
        console.error(`[KUROMI] 📊 session.diff │ count=${diffs.length} files=${files}`)
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID
        console.error(`[KUROMI] 🔴 session.deleted │ id=${safeSlice(sid, 20)}`)
        console.error(`[KUROMI] ${SEP}`)
      }

      if (type === "session.error") {
        console.error(`[KUROMI] 💥 session.error │ err=${safeSlice(props.error, 200)}`)
      }

      // ─── MESSAGE EVENTS ───
      if (type === "message.updated") {
        const info = props.info || {}
        if (info.role === "assistant") {
          console.error(`[KUROMI] 💬 message.updated │ role=assistant model=${safeSlice(info.modelID, 30)} finish=${safeSlice(info.finish, 10)}`)
        }
      }

      if (type === "message.removed") {
        console.error(`[KUROMI] 🗑️  message.removed │ msgID=${safeSlice(props.messageID, 20)}`)
      }

      // ─── MESSAGE PART EVENTS ───
      if (type === "message.part.updated") {
        const part = props.part || {}

        if (part.type === "subtask") {
          console.error(`[KUROMI] 🤖 part.subtask │ agent=${safeSlice(part.agent, 20)} desc=${safeSlice(part.description, 80)}`)
        }

        if (part.type === "tool") {
          const state = part.state || {}
          const tool = safeSlice(part.tool, 15)
          const call = safeSlice(part.callID, 10)
          const status = safeSlice(state.status, 10)

          if (status === "completed") {
            const t = (state.time as any) || {}
            const dur = t.start && t.end ? `${t.end - t.start}ms` : "?"
            console.error(`[KUROMI] ✅ tool.completed │ tool=${tool} dur=${dur} title=${safeSlice(state.title, 60)}`)
          }
          if (status === "error") {
            console.error(`[KUROMI] ❌ tool.error │ tool=${tool} err=${safeSlice(state.error, 150)}`)
          }
          if (status === "running") {
            console.error(`[KUROMI] 🔧 tool.running │ tool=${tool} call=${call}`)
          }
        }

        if (part.type === "step-finish") {
          console.error(`[KUROMI] 🏁 part.step-finish │ reason=${safeSlice(part.reason, 20)}`)
        }

        if (part.type === "reasoning") {
          console.error(`[KUROMI] 🧠 part.reasoning │ text=${safeSlice((part as any).text, 100)}`)
        }

        if (part.type === "file") {
          console.error(`[KUROMI] 📄 part.file │ file=${safeSlice((part as any).filename, 60)}`)
        }

        if (part.type === "patch") {
          const pf = (part as any).files || []
          console.error(`[KUROMI] 🔀 part.patch │ files=${pf.slice(0, 5).join(" | ")}`)
        }

        if (part.type === "compaction") {
          console.error(`[KUROMI] 🗜️  part.compaction │ auto=${(part as any).auto ?? ""}`)
        }

        if (part.type === "agent") {
          console.error(`[KUROMI] 👤 part.agent │ name=${safeSlice((part as any).name, 20)}`)
        }

        if (part.type === "retry") {
          console.error(`[KUROMI] 🔄 part.retry │ attempt=${(part as any).attempt ?? ""}`)
        }
      }

      // ─── FILE / PERMISSION / TASK / COMMAND ───
      if (type === "file.edited") {
        console.error(`[KUROMI] ✏️  file.edited │ file=${safeSlice(props.file, 80)}`)
      }

      if (type === "permission.updated") {
        console.error(`[KUROMI] 🔐 permission.updated │ type=${safeSlice(props.type, 15)} pattern=${safeSlice(props.pattern, 60)}`)
      }

      if (type === "permission.replied") {
        console.error(`[KUROMI] 🔓 permission.replied │ reply=${safeSlice(props.response || props.reply, 20)}`)
      }

      if (type === "todo.updated") {
        const todos = Array.isArray(props.todos) ? props.todos : []
        const done = todos.filter((t: any) => t.status === "completed").length
        const active = todos.filter((t: any) => t.status !== "completed").length
        console.error(`[KUROMI] ✅ todo.updated │ total=${todos.length} done=${done} active=${active}`)
      }

      if (type === "command.executed") {
        console.error(`[KUROMI] ⌨️  command.executed │ name=${safeSlice(props.name, 20)} args=${safeSlice(props.arguments, 60)}`)
      }
    },

    // HOOKS
    "chat.message": async (input, _o) => {
      const parts = (_o as any)?.parts || []
      console.error(`[KUROMI] 💬 chat.message │ agent=${safeSlice(input.agent, 15)} model=${safeSlice(input.model, 20)} parts=${parts.length}`)
    },

    "chat.params": async (input, _o) => {
      console.error(`[KUROMI] ⚙️  chat.params │ agent=${safeSlice(input.agent, 15)} model=${safeSlice(input.model?.id || input.model, 30)}`)
    },

    "experimental.chat.system.transform": async (_i, output) => {
      if (!Array.isArray(output.system)) return
      console.error(`[KUROMI] 🔄 system.transform │ blocks=${output.system.length}`)
    },

    "tool.execute.before": async (input, output) => {
      const args = (output as any)?.args || {}
      const keys = Object.keys(args).join(",").slice(0, 80)
      console.error(`[KUROMI] ⏩ tool.execute.before │ tool=${safeSlice(input.tool, 15)} args=${keys}`)
    },

    config: async (input) => {
      console.error(`[KUROMI] 📋 config │ model=${safeSlice(input.model?.id, 30)} agent=${safeSlice(input.agent?.id, 15)}`)
    },
  }
}
