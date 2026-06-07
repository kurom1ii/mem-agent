#!/usr/bin/env node
// mem-agent post-tool-use hook
// Called after each tool execution to track context.

import { spawnMemAgent, waitForExit } from "./_memagent.mjs";

async function main() {
  const toolName = process.env.OPENCODE_TOOL_NAME || "";

  if (!["Write", "Edit", "Bash"].includes(toolName)) return;

  try {
    const result = spawnMemAgent(["stats"], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 3000,
    });

    let output = "";
    result.stdout.on("data", (data) => { output += data.toString(); });

    await waitForExit(result);

    const match = output.match(/Memory count:\s+(\d+)/);
    const count = match ? parseInt(match[1]) : 0;

    // Every 10 tool uses, remind about memory
    if (count > 0 && count % 10 === 0) {
      console.log(`[mem-agent] 📊 ${count} memories stored. Use /remember to save insights.`);
    }
  } catch (e) {
    // silent fail
  }
}

main().catch(() => {});
