# OpenCode Plugin Hooks — Complete Reference

Tài liệu này liệt kê TOÀN BỘ hooks mà OpenCode Plugin API hỗ trợ.
Dựa trên source code OpenCode và kinh nghiệm từ agentmemory.

---

## I. Plugin Factory Entry

File `.ts` export function, OpenCode gọi function này với `input` object.

```ts
import type { Plugin } from "@opencode-ai/plugin"

export const MyPlugin: Plugin = async (ctx) => {
  // ctx.worktree  — thư mục project
  // ctx.project   — thông tin project
  // ctx.$         — Bun.$ shell helper (chỉ server plugin mới có)
  // ctx.client    — OpenCode SDK client
  // ctx.directory — thư mục làm việc

  return {
    // → trả về object chứa các hooks bên dưới
  }
}
```

---

## II. Named Hooks

### 1. `config`

**Khi chạy:** Một lần khi OpenCode load config (khởi động).

```ts
config: async (input) => {
  // input.model      — model config { id, providerID, limit, cost }
  // input.theme      — theme name
  // input.autoupdate — auto-update setting
  // input.agent      — agent config { id }
  // input.mcp        — MCP servers
  // input.provider   — LLM providers
  // input.permission — permission settings
}
```

---

### 2. `event` — Universal Event Dispatcher

**Khi chạy:** Mọi sự kiện session lifecycle đều đi qua đây.
Phân biệt bằng `event.type`. Đây là hook quan trọng nhất.

```ts
event: async ({ event }) => {
  const type = event.type       // string
  const props = event.properties // Record<string, unknown>

  switch (type) { ... }
}
```

#### 2.1 Session Lifecycle Events

| `event.type` | Khi nào xảy ra | `props` chứa |
|---|---|---|
| `session.created` | Tạo phiên mới | `info.id`, `info.title`, `info.parentID`, `info.version` |
| `session.status` | Trạng thái phiên thay đổi | `status.type` ("idle"/"active"), `status.attempt`, `status.message` |
| `session.compacted` | Context bị nén (compaction) | — |
| `session.updated` | Metadata phiên cập nhật | `info.title`, `info.parentID`, `info.summary.additions`, `info.summary.deletions`, `info.summary.files` |
| `session.diff` | Có diff giữa các lần chat | `diff[]` — mảng các `{ file, additions, deletions }` |
| `session.deleted` | Phiên kết thúc/đóng | `info.id` hoặc `sessionID` |
| `session.error` | Phiên gặp lỗi | `error` |

#### 2.2 Message Events

| `event.type` | Khi nào xảy ra | `props` chứa |
|---|---|---|
| `message.updated` | User gửi tin nhắn HOẶC AI trả lời | `info.role` ("user"/"assistant"), `info.id`, `info.modelID`, `info.providerID`, `info.mode`, `info.cost`, `info.tokens`, `info.time`, `info.finish`, `info.error` |
| `message.removed` | Tin nhắn bị xóa khỏi context | `messageID` |

#### 2.3 Message Part Events (`message.part.updated`)

Khi 1 phần của tin nhắn thay đổi (chi tiết nhất). `props.part.type` quyết định loại.

| `part.type` | Khi nào xảy ra | `part` chứa |
|---|---|---|
| `tool` | AI gọi 1 tool | `.tool`, `.callID`, `.state.status` ("running"/"completed"/"error"), `.state.input`, `.state.output`, `.state.time`, `.state.title` |
| `subtask` | AI spawn subagent | `.id`, `.agent`, `.description`, `.prompt` |
| `step-finish` | AI kết thúc 1 bước reasoning | `.messageID`, `.reason`, `.cost`, `.tokens` |
| `reasoning` | AI đang suy nghĩ (thinking) | `.text` |
| `file` | File được tạo/tham chiếu | `.filename` hoặc `.url` |
| `patch` | Code change được đề xuất | `.hash`, `.files[]` |
| `compaction` | Context bị nén | `.auto` (boolean) |
| `agent` | AI chọn agent khác | `.name` |
| `retry` | AI thử lại sau lỗi | `.attempt`, `.error` |

#### 2.4 File Events

| `event.type` | Khi nào xảy ra | `props` chứa |
|---|---|---|
| `file.edited` | File được chỉnh sửa (bằng tool Write/Edit) | `.file` (path), `.sessionID` |

#### 2.5 Permission Events

| `event.type` | Khi nào xảy ra | `props` chứa |
|---|---|---|
| `permission.updated` | OpenCode hiện prompt hỏi quyền (allow/deny tool, file, command) | `.type`, `.pattern`, `.callID`, `.title`, `.metadata` |
| `permission.replied` | User trả lời cho phép/từ chối | `.permissionID` hoặc `.requestID`, `.response` hoặc `.reply` |

#### 2.6 Task Events

| `event.type` | Khi nào xảy ra | `props` chứa |
|---|---|---|
| `todo.updated` | Todo list thay đổi (task completed/in-progress/cancelled) | `.todos[]` — mảng `{ content, status, priority }` |

#### 2.7 Command Events

| `event.type` | Khi nào xảy ra | `props` chứa |
|---|---|---|
| `command.executed` | User chạy slash command (/remember, /recall, ...) | `.name`, `.arguments` |

---

### 3. `chat.message`

**Khi chạy:** User gửi tin nhắn HOẶC AI trả lời xong.

```ts
"chat.message": async (input, output) => {
  // input.sessionID, input.agent, input.model, input.variant
  // output.parts[] — các phần của message (text, tool, file, reasoning...)
}
```

---

### 4. `chat.params`

**Khi chạy:** Tham số chat thay đổi (model, temperature, topP).

```ts
"chat.params": async (input, output) => {
  // input.agent, input.model
  // output.temperature, output.topP, output.max_output_tokens
}
```

---

### 5. `tool.execute.before`

**Khi chạy:** TRƯỚC KHI AI thực thi 1 tool (Write, Edit, Read, Glog, Grep, Bash, ...).
Cho phép can thiệp vào arguments của tool trước khi nó chạy.
Đây là hook duy nhất có thể SỬA ĐỔI arguments.

```ts
"tool.execute.before": async (input, output) => {
  // input.tool    — tên tool ("Write", "Edit", "Grep"...)
  // input.sessionID
  // output.args   — arguments (CÓ THỂ SỬA được)
}
```

---

### 6. `experimental.chat.system.transform`

**Khi chạy:** TRƯỚC KHI system prompt được gửi cho LLM.
Cho phép INJECT nội dung vào system prompt.

```ts
"experimental.chat.system.transform": async (input, output) => {
  // input.sessionID
  // output.system[] — MẢNG string. Bạn có thể push thêm!
  output.system.push("Bạn có quyền truy cập memory tools...")
}
```

⚠️ **Chỉ inject 1 lần mỗi session** — dùng Set để theo dõi session đã inject.

---

### 7. `experimental.session.compacting`

**Khi chạy:** Khi context của session sắp bị nén (compaction).
Cho phép inject context bổ sung trước khi nén.

```ts
"experimental.session.compacting": async (input, output) => {
  // input.sessionID
  // output.context[] — có thể push context bổ sung
}
```

---

## III. Tổng Quan Luồng Hook

```
OPEnCODE STARTUP
├── config()                                      [1 lần]

SESSION START
├── event: session.created
├── experimental.chat.system.transform            [inject instructions]
├── tool.execute.before                           [nếu có auto tool đầu session]

USER CHAT
├── chat.message
├── event: message.updated                        [user prompt]
├── event: message.part.updated (reasoning)       [AI đang thinking]
├── event: message.part.updated (tool.*)          [AI gọi tool]
├── tool.execute.before                           [trước mỗi tool call]
├── event: message.part.updated (tool.completed)  [sau tool call]
├── event: message.part.updated (subtask)         [subagent start]
├── event: message.part.updated (step-finish)     [kết thúc 1 step]
├── chat.message                                  [AI reply xong]

DURING SESSION
├── event: file.edited                            [file bị edit]
├── event: permission.updated                     [hỏi quyền]
├── event: permission.replied                     [trả lời quyền]
├── event: todo.updated                           [task list đổi]
├── event: command.executed                       [/command]
├── event: session.status                         [idle/active]
├── event: session.diff                           [diff giữa các chat]
├── event: session.compacted                      [context compact]
├── event: session.updated                        [metadata đổi]

SESSION END
├── event: session.deleted
```

## IV. So Sánh agentmemory vs kuromi-plugin

| Hook | agentmemory | kuromi-plugin |
|---|---|---|
| `config` | ✅ Capture model config | ✅ Log |
| `event: session.created` | ✅ POST /session/start | ✅ Log |
| `event: session.status` | ✅ POST /summarize khi idle | ✅ Log |
| `event: session.compacted` | ✅ POST /summarize | ✅ Log |
| `event: session.updated` | ✅ POST /observe | ✅ Log |
| `event: session.diff` | ✅ POST /observe | ✅ Log |
| `event: session.deleted` | ✅ POST /session/end | ✅ Log |
| `event: session.error` | ✅ POST /observe | ✅ Log |
| `event: message.updated` | ✅ Capture + llm_params | ✅ Log |
| `event: message.removed` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (tool)` | ✅ POST /observe + dedup | ✅ Log + dedup |
| `event: message.part.updated (subtask)` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (step-finish)` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (reasoning)` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (file)` | ✅ Track files | ✅ Log |
| `event: message.part.updated (patch)` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (compaction)` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (agent)` | ✅ POST /observe | ✅ Log |
| `event: message.part.updated (retry)` | ✅ POST /observe | ✅ Log |
| `event: file.edited` | ✅ Track files | ✅ Log |
| `event: permission.updated` | ✅ POST /observe | ✅ Log |
| `event: permission.replied` | ✅ POST /observe | ✅ Log |
| `event: todo.updated` | ✅ Capture | ✅ Log |
| `event: command.executed` | ✅ POST /observe | ✅ Log |
| `chat.message` | ✅ Capture prompt | ✅ Log |
| `chat.params` | ✅ Capture model params | ✅ Log |
| `system.transform` | ✅ Inject instructions + context | ✅ Log |
| `tool.execute.before` | ✅ Track file paths | ✅ Log |
| `session.compacting` | ❌ | ✅ Log |
