
import type { Plugin } from "@opencode-ai/plugin"
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs"
import { dirname } from "node:path"

function safeSlice(v: unknown, max: number): string {
  if (typeof v === "string") return v.slice(0, max)
  if (v == null) return ""
  try { return JSON.stringify(v).slice(0, max) } catch { return "" }
}

const LOG_PATH = "/home/kuromi/work/mywork/mem-agent/plugins-test/kuromi-plugin-opencode/log.log"
const SEP = "─".repeat(92)
const LABEL_WIDTH = 24
let sessionCount = 0
let sessionId = ""

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
} as const

const LABEL_COLORS: Record<string, string> = {
  PLUGIN_LOADED: ANSI.bold + ANSI.white,
  SESSION_CREATED: ANSI.bold + ANSI.green,
  SESSION_STATUS: ANSI.cyan,
  SESSION_COMPACTED: ANSI.magenta,
  SESSION_UPDATED: ANSI.blue,
  SESSION_DIFF: ANSI.yellow,
  SESSION_DELETED: ANSI.bold + ANSI.red,
  SESSION_ERROR: ANSI.bold + ANSI.red,
  MSG_UPDATED: ANSI.green,
  MSG_REMOVED: ANSI.red,
  PART_SUBTASK: ANSI.cyan,
  TOOL_RUNNING: ANSI.blue,
  TOOL_COMPLETED: ANSI.green,
  TOOL_ERROR: ANSI.red,
  PART_STEP_FINISH: ANSI.yellow,
  PART_REASONING: ANSI.magenta,
  PART_FILE: ANSI.white,
  PART_PATCH: ANSI.yellow,
  PART_COMPACTION: ANSI.magenta,
  PART_AGENT: ANSI.cyan,
  PART_RETRY: ANSI.yellow,
  FILE_EDITED: ANSI.blue,
  PERMISSION_UPDATED: ANSI.yellow,
  PERMISSION_REPLIED: ANSI.green,
  TODO_UPDATED: ANSI.green,
  COMMAND_EXECUTED: ANSI.cyan,
  CHAT_MESSAGE: ANSI.blue,
  CHAT_PARAMS: ANSI.cyan,
  SYSTEM_TRANSFORM: ANSI.magenta,
  TOOL_BEFORE: ANSI.yellow,
  CONFIG: ANSI.white,
  SECTION: ANSI.dim + ANSI.gray,
} as const

const LABEL_DESCRIPTIONS: Record<string, string> = {
  PLUGIN_LOADED: "plugin boot",
  SESSION_CREATED: "new session",
  SESSION_STATUS: "session state",
  SESSION_COMPACTED: "context compacted",
  SESSION_UPDATED: "metadata changed",
  SESSION_DIFF: "chat diff",
  SESSION_DELETED: "session closed",
  SESSION_ERROR: "session error",
  MSG_UPDATED: "message changed",
  MSG_REMOVED: "message removed",
  PART_SUBTASK: "spawn subagent",
  TOOL_RUNNING: "tool started",
  TOOL_COMPLETED: "tool finished",
  TOOL_ERROR: "tool failed",
  PART_STEP_FINISH: "reasoning step done",
  PART_REASONING: "model thinking",
  PART_FILE: "file referenced",
  PART_PATCH: "patch proposed",
  PART_COMPACTION: "compaction part",
  PART_AGENT: "agent chosen",
  PART_RETRY: "retry after error",
  FILE_EDITED: "file changed",
  PERMISSION_UPDATED: "permission prompt",
  PERMISSION_REPLIED: "permission answer",
  TODO_UPDATED: "todo changed",
  COMMAND_EXECUTED: "slash command",
  CHAT_MESSAGE: "chat payload",
  CHAT_PARAMS: "chat params",
  SYSTEM_TRANSFORM: "inject system prompt",
  TOOL_BEFORE: "pre-tool args",
  CONFIG: "config loaded",
  SECTION: "session marker",
} as const

mkdirSync(dirname(LOG_PATH), { recursive: true })

function nowStamp(): string {
  const d = new Date()
  const hh = String(d.getHours()).padStart(2, "0")
  const mm = String(d.getMinutes()).padStart(2, "0")
  const ss = String(d.getSeconds()).padStart(2, "0")
  const ms = String(d.getMilliseconds()).padStart(3, "0")
  return `${hh}:${mm}:${ss}.${ms}`
}

function padLabel(label: string): string {
  return label.padEnd(LABEL_WIDTH, " ")
}

function fmtValue(v: unknown): string {
  return safeSlice(v, 220).replace(/\s+/g, " ").trim()
}

function previewUnknown(value: unknown, depth = 0, seen = new Set<unknown>()): string {
  if (depth > 3 || value == null) return ""
  if (typeof value === "string") return value.trim() ? safeSlice(value, 140) : ""
  if (typeof value !== "object") return ""
  if (seen.has(value)) return ""

  seen.add(value)

  if (Array.isArray(value)) {
    for (const item of value) {
      const hit = previewUnknown(item, depth + 1, seen)
      if (hit) return hit
    }
    return ""
  }

  const obj = value as Record<string, unknown>
  const priorityKeys = [
    "text",
    "content",
    "prompt",
    "message",
    "input",
    "output",
    "body",
    "value",
    "parts",
  ]

  for (const key of priorityKeys) {
    if (!(key in obj)) continue
    const hit = previewUnknown(obj[key], depth + 1, seen)
    if (hit) return hit
  }

  for (const item of Object.values(obj)) {
    const hit = previewUnknown(item, depth + 1, seen)
    if (hit) return hit
  }

  return ""
}

function fmtFields(fields: Record<string, unknown>): string {
  return Object.entries(fields)
    .filter(([, value]) => value !== "" && value != null)
    .map(([key, value]) => `${ANSI.dim}${key}${ANSI.reset}=${fmtValue(value)}`)
    .join(` ${ANSI.gray}|${ANSI.reset} `)
}

function rawWrite(line: string): void {
  appendFileSync(LOG_PATH, `${line}\n`)
}

function logLine(label: string, fields: Record<string, unknown> = {}): void {
  const color = LABEL_COLORS[label] || ANSI.white
  const desc = LABEL_DESCRIPTIONS[label]
  const descText = desc ? ` ${ANSI.dim}(${desc})${ANSI.reset}` : ""
  const head = `${ANSI.gray}[${nowStamp()}]${ANSI.reset} ${color}${padLabel(label)}${ANSI.reset}${descText}`
  const body = fmtFields(fields)
  rawWrite(body ? `${head} ${ANSI.gray}>>${ANSI.reset} ${body}` : head)
}

function logSection(title: string): void {
  rawWrite(`${LABEL_COLORS.SECTION}${SEP}${ANSI.reset}`)
  logLine("SECTION", { title, session: sessionId || "-" })
  rawWrite(`${LABEL_COLORS.SECTION}${SEP}${ANSI.reset}`)
}

export const KuromiPlugin: Plugin = async (ctx) => {
  const project = ctx.worktree || "?"
  writeFileSync(LOG_PATH, "")
  logLine("PLUGIN_LOADED", {
    project,
    pid: process?.pid,
    log: LOG_PATH,
  })

  return {
    event: async ({ event }) => {
      const type = event.type
      const props = (event as any).properties || {}

      // --- SESSION LIFECYCLE ---

      if (type === "session.created") {
        sessionCount++
        const info = props.info || {}
        sessionId = safeSlice(info.id, 20)
        logSection("SESSION START")
        logLine("SESSION_CREATED", {
          id: sessionId,
          title: safeSlice(info.title, 80),
          count: sessionCount,
        })
      }

      if (type === "session.status") {
        const s = props.status || {}
        logLine("SESSION_STATUS", {
          type: safeSlice(s.type, 20),
          attempt: s.attempt ?? "",
        })
      }

      if (type === "session.compacted") {
        logLine("SESSION_COMPACTED")
      }

      if (type === "session.updated") {
        const info = props.info || {}
        logLine("SESSION_UPDATED", {
          title: safeSlice(info.title, 80),
        })
      }

      if (type === "session.diff") {
        const diffs = Array.isArray(props.diff) ? props.diff : []
        const files = diffs.map((d: any) => d.file).slice(0, 5).join(" | ")
        logLine("SESSION_DIFF", {
          count: diffs.length,
          files,
        })
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID
        logLine("SESSION_DELETED", {
          id: safeSlice(sid, 20),
        })
        logSection("SESSION END")
      }

      if (type === "session.error") {
        logLine("SESSION_ERROR", {
          err: safeSlice(props.error, 200),
        })
      }

      // --- MESSAGE EVENTS ---

      if (type === "message.updated") {
        const info = props.info || {}
        logLine("MSG_UPDATED", {
          role: safeSlice(info.role, 12),
          model: safeSlice(info.modelID, 30),
          provider: safeSlice(info.providerID, 20),
          finish: safeSlice(info.finish, 10),
          error: safeSlice(info.error, 100),
          text: previewUnknown(props),
        })
      }

      if (type === "message.removed") {
        logLine("MSG_REMOVED", {
          msgID: safeSlice(props.messageID, 20),
        })
      }

      // --- MESSAGE PART EVENTS ---

      if (type === "message.part.updated") {
        const part = props.part || {}

        if (part.type === "subtask") {
          logLine("PART_SUBTASK", {
            agent: safeSlice(part.agent, 20),
            desc: safeSlice(part.description, 80),
          })
        }

        if (part.type === "tool") {
          const state = part.state || {}
          const tool = safeSlice(part.tool, 15)
          const call = safeSlice(part.callID, 10)
          const status = safeSlice(state.status, 10)

          if (status === "completed") {
            const t = (state.time as any) || {}
            const dur = t.start && t.end ? `${t.end - t.start}ms` : "?"
            logLine("TOOL_COMPLETED", {
              tool,
              dur,
              title: safeSlice(state.title, 60),
            })
          }
          if (status === "error") {
            logLine("TOOL_ERROR", {
              tool,
              err: safeSlice(state.error, 150),
            })
          }
          if (status === "running") {
            logLine("TOOL_RUNNING", {
              tool,
              call,
            })
          }
        }

        if (part.type === "step-finish") {
          logLine("PART_STEP_FINISH", {
            reason: safeSlice(part.reason, 20),
          })
        }

        if (part.type === "reasoning") {
          logLine("PART_REASONING", {
            text: safeSlice((part as any).text, 100),
          })
        }

        if (part.type === "file") {
          logLine("PART_FILE", {
            file: safeSlice((part as any).filename, 60),
          })
        }

        if (part.type === "patch") {
          const pf = (part as any).files || []
          logLine("PART_PATCH", {
            files: pf.slice(0, 5).join(" | "),
          })
        }

        if (part.type === "compaction") {
          logLine("PART_COMPACTION", {
            auto: (part as any).auto ?? "",
          })
        }

        if (part.type === "agent") {
          logLine("PART_AGENT", {
            name: safeSlice((part as any).name, 20),
          })
        }

        if (part.type === "retry") {
          logLine("PART_RETRY", {
            attempt: (part as any).attempt ?? "",
          })
        }
      }

      // --- FILE / PERMISSION / TASK / COMMAND ---

      if (type === "file.edited") {
        logLine("FILE_EDITED", {
          file: safeSlice(props.file, 80),
        })
      }

      if (type === "permission.updated") {
        logLine("PERMISSION_UPDATED", {
          type: safeSlice(props.type, 15),
          pattern: safeSlice(props.pattern, 60),
        })
      }

      if (type === "permission.replied") {
        logLine("PERMISSION_REPLIED", {
          reply: safeSlice(props.response || props.reply, 20),
        })
      }

      if (type === "todo.updated") {
        const todos = Array.isArray(props.todos) ? props.todos : []
        const done = todos.filter((t: any) => t.status === "completed").length
        const active = todos.filter((t: any) => t.status !== "completed").length
        logLine("TODO_UPDATED", {
          total: todos.length,
          done,
          active,
        })
      }

      if (type === "command.executed") {
        logLine("COMMAND_EXECUTED", {
          name: safeSlice(props.name, 20),
          args: safeSlice(props.arguments, 60),
        })
      }
    },

    // HOOKS
    "chat.message": async (input, _o) => {
      const parts = (_o as any)?.parts || []
      logLine("CHAT_MESSAGE", {
        agent: safeSlice(input.agent, 15),
        model: safeSlice(input.model, 20),
        parts: parts.length,
        inputText: previewUnknown(input),
        outputText: previewUnknown(_o),
      })
    },

    "chat.params": async (input, _o) => {
      logLine("CHAT_PARAMS", {
        agent: safeSlice(input.agent, 15),
        model: safeSlice(input.model?.id || input.model, 30),
      })
    },

    "experimental.chat.system.transform": async (_i, output) => {
      if (!Array.isArray(output.system)) return
      logLine("SYSTEM_TRANSFORM", {
        blocks: output.system.length,
      })
    },

    "tool.execute.before": async (input, output) => {
      const args = (output as any)?.args || {}
      const keys = Object.keys(args).join(",").slice(0, 80)
      logLine("TOOL_BEFORE", {
        tool: safeSlice(input.tool, 15),
        args: keys,
      })
    },

    config: async (input) => {
      logLine("CONFIG", {
        model: safeSlice(input.model, 30),
        agent: safeSlice(input.agent?.id, 15),
      })
    },
  }
}
