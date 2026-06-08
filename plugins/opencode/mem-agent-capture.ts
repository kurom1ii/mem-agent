import type { Plugin } from "@opencode-ai/plugin";
import {
  MEMAGENT_INSTRUCTIONS,
  FILE_TOOLS,
  MAX_STASHED_FILES,
  MAX_MESSAGE_CONTENT,
  MAX_TOOL_CONTENT,
  RECALL_LIMIT,
  safeSlice,
  serializePayload,
  extractFilePaths,
  extractTextParts,
  extractFileParts,
  summarizeModel,
  makeTitle,
  enqueueMemorySave,
  scheduleRecentRefresh,
  scheduleRecallRefresh,
  getCachedRecent,
  getCachedRecall,
  fileSet,
  toolCallSet,
  messageIdSet,
  sessionCleanup,
  createSessionState,
  memLog,
  type SessionState,
} from "./hooks.js";

const COLOR = { inject: "\x1b[93m\x1b[1m", tool: "\x1b[96m" } as const;

let state: SessionState;

export const MemAgentPlugin: Plugin = async (ctx) => {
  const projectPath = ctx.worktree || (ctx.project as any)?.path || ctx.project?.id || "";
  state = createSessionState(projectPath);
  scheduleRecentRefresh();

  return {
    event: async ({ event }) => {
      const type = event.type;
      const props = (event as any).properties || {};

      if (type === "session.created") {
        const info = props.info as Record<string, unknown> | undefined;
        state.activeSessionId = (info?.id as string) || props.sessionID || null;
        if (!state.activeSessionId) return;

        fileSet(state, state.activeSessionId);
        state.seenToolCallIds.delete(state.activeSessionId);
        state.seenMessageIds.delete(state.activeSessionId);
        state.contextInjected.delete(state.activeSessionId);
        state.recentInjected.delete(state.activeSessionId);
        state.turnPrompts.delete(state.activeSessionId);
        scheduleRecentRefresh();
        return;
      }

      if (type === "session.deleted") {
        const sid = props.info?.id || props.sessionID || state.activeSessionId;
        if (!sid) return;
        sessionCleanup(sid, state);
        if (sid === state.activeSessionId) state.activeSessionId = null;
        return;
      }

      if (type === "message.updated") {
        const info = props.info as Record<string, unknown> | undefined;
        const sid = props.sessionID || (info?.sessionID as string) || state.activeSessionId;
        if (!info || !sid) return;
        if (info.role === "assistant") {
          state.turnPrompts.delete(sid);
        }
        return;
      }

      if (type === "message.part.updated") {
        const part = props.part as Record<string, unknown> | undefined;
        if (!part) return;
        const sid = (part.sessionID as string) || props.sessionID || state.activeSessionId;
        if (!sid) return;

        if (part.type === "tool") {
          const toolState = part.state as Record<string, unknown> | undefined;
          if (!toolState) return;
          const callId = part.callID as string;
          if (!callId) return;

          const seen = toolCallSet(state, sid);
          if (seen.has(callId)) return;

          if (toolState.status === "completed" || toolState.status === "error") {
            seen.add(callId);

            if (toolState.input && typeof toolState.input === "object") {
              for (const file of extractFilePaths(toolState.input as Record<string, unknown>)) {
                fileSet(state, sid).add(file);
              }
            }

            enqueueMemorySave(
              makeTitle("tool", `${part.tool || "tool"} ${toolState.title || callId}`),
              [
                `kind=tool_result`,
                `session=${sid}`,
                `project=${state.projectPath}`,
                `tool=${part.tool || ""}`,
                `status=${toolState.status}`,
                `call_id=${callId}`,
                `title=${safeSlice(toolState.title, 160)}`,
                "",
                `input:\n${serializePayload(toolState.input, MAX_TOOL_CONTENT)}`,
                "",
                `${toolState.status === "completed" ? "output" : "error"}:\n${serializePayload(
                  toolState.output ?? toolState.error,
                  MAX_TOOL_CONTENT,
                )}`,
              ].join("\n"),
              [
                "auto",
                "tool",
                typeof part.tool === "string" ? part.tool : "tool",
                toolState.status === "completed" ? "success" : "error",
              ],
            );
            memLog("TOOL", COLOR.tool, `${part.tool || "?"} ${safeSlice(toolState.title, 60)}`);
          }
          return;
        }

        if (part.type === "patch") {
          const files = Array.isArray((part as any).files) ? (part as any).files : [];
          for (const file of files) {
            if (typeof file === "string") fileSet(state, sid).add(file);
          }
          return;
        }

        if (part.type === "file") {
          const filename = (part as any).filename || (part as any).url;
          if (typeof filename === "string" && filename.length > 0) {
            fileSet(state, sid).add(filename);
          }
          return;
        }
      }

      if (type === "file.edited") {
        const sid = props.sessionID || state.activeSessionId;
        if (sid && typeof props.file === "string") {
          const stash = fileSet(state, sid);
          stash.add(props.file);
          if (stash.size > MAX_STASHED_FILES) {
            const keep = [...stash].slice(-MAX_STASHED_FILES);
            stash.clear();
            for (const file of keep) stash.add(file);
          }
          scheduleRecallRefresh(props.file, 3);
        }
      }
    },

    "chat.message": async (input, output) => {
      const sid = input.sessionID || state.activeSessionId;
      if (!sid) return;

      const message =
        output && typeof output === "object" && (output as any).message
          ? ((output as any).message as Record<string, unknown>)
          : undefined;
      const role = typeof message?.role === "string" ? message.role : "user";
      const messageId = typeof message?.id === "string" ? message.id : "";
      const parts = Array.isArray((output as any)?.parts) ? (output as any).parts : [];
      const text = extractTextParts(parts);
      const files = extractFileParts(parts);

      const stash = fileSet(state, sid);
      for (const file of files) stash.add(file);
      for (const file of files) scheduleRecallRefresh(file, 3);
      if (stash.size > MAX_STASHED_FILES) {
        const keep = [...stash].slice(-MAX_STASHED_FILES);
        stash.clear();
        for (const file of keep) stash.add(file);
      }

      if (!text) return;

      if (messageId) {
        const seen = messageIdSet(state, sid);
        const key = `${role}:${messageId}`;
        if (seen.has(key)) return;
        seen.add(key);
      }

      if (role === "assistant") {
        state.turnPrompts.delete(sid);
        return;
      }

      state.turnPrompts.set(sid, text);
      enqueueMemorySave(
        makeTitle("user", text),
        [
          `kind=prompt_submit`,
          `session=${sid}`,
          `project=${state.projectPath}`,
          `model=${summarizeModel(input.model)}`,
          `files=${files.join(", ")}`,
          "",
          safeSlice(text, MAX_MESSAGE_CONTENT),
        ].join("\n"),
        ["auto", "prompt", "user", "chat"],
        { kind: "prompt", sessionId: sid, source: "opencode", project: state.projectPath },
      );
      scheduleRecallRefresh(text, RECALL_LIMIT);
    },

    "experimental.chat.system.transform": async (input, output) => {
      const sid = input.sessionID || state.activeSessionId;
      if (!sid || !Array.isArray(output.system)) return;

      if (!state.contextInjected.has(sid)) {
        output.system.push(MEMAGENT_INSTRUCTIONS);
        state.contextInjected.add(sid);
        memLog("INJECT", COLOR.inject, `instructions → ${sid?.slice(0, 8)}...`);
      }

      const recent = getCachedRecent();
      if (recent && !state.recentInjected.has(sid)) {
        output.system.push(`<mem-agent-recent>\n${recent}\n</mem-agent-recent>`);
        state.recentInjected.add(sid);
        memLog("INJECT", COLOR.inject, `recent → ${sid?.slice(0, 8)}...`);
      } else {
        scheduleRecentRefresh();
      }

      const prompt = state.turnPrompts.get(sid)?.trim();
      if (prompt) {
        const recalled = getCachedRecall(prompt, RECALL_LIMIT);
        if (recalled) {
          output.system.push(
            `<mem-agent-recall>\nQuery: ${safeSlice(prompt, 240)}\n${recalled}\n</mem-agent-recall>`,
          );
          memLog("INJECT", COLOR.inject, `recall "${safeSlice(prompt, 40)}" → hit`);
        } else {
          scheduleRecallRefresh(prompt, RECALL_LIMIT);
          memLog("INJECT", COLOR.inject, `recall "${safeSlice(prompt, 40)}" → scheduling`);
        }
      }

      const stash = fileSet(state, sid);
      if (stash.size > 0) {
        const files = [...stash].slice(0, 5);
        const sections: string[] = [];

        for (const file of files) {
          const recalled = getCachedRecall(file, 3);
          if (recalled) {
            sections.push(`File: ${file}\n${recalled}`);
            stash.delete(file);
          } else {
            scheduleRecallRefresh(file, 3);
          }
        }

        if (sections.length > 0) {
          output.system.push(
            `<mem-agent-file-context>\n${sections.join("\n\n")}\n</mem-agent-file-context>`,
          );
        }
      }
    },

    "tool.execute.before": async (input, output) => {
      if (!FILE_TOOLS.has(input.tool)) return;
      const sid = input.sessionID || state.activeSessionId;
      if (!sid) return;

      const args = output.args as Record<string, unknown> | undefined;
      if (!args) return;

      const stash = fileSet(state, sid);
      for (const file of extractFilePaths(args)) stash.add(file);
      for (const file of extractFilePaths(args)) scheduleRecallRefresh(file, 3);
      if (stash.size > MAX_STASHED_FILES) {
        const keep = [...stash].slice(-MAX_STASHED_FILES);
        stash.clear();
        for (const file of keep) stash.add(file);
      }
    },
  };
};
