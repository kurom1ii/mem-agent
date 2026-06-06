#!/usr/bin/env node
// mem-agent notification hook
// Tracks permission prompts and user decisions
const toolName = process.env.OPENCODE_NOTIFICATION_TYPE || "";
if (toolName) {
  console.log(`[mem-agent] Notification: ${toolName}`);
}
