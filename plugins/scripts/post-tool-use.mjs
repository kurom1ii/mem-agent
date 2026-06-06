#!/usr/bin/env node
// mem-agent post-tool-use hook
// Called after each tool execution to track context.

const fs = require("fs");
const path = require("path");

async function main() {
  const toolName = process.env.OPENCODE_TOOL_NAME || "";
  const sessionId = process.env.OPENCODE_SESSION_ID || "";

  if (!["Write", "Edit", "Bash"].includes(toolName)) return;

  const memAgentBin = process.env.MEMAGENT_BIN || "mem-agent";
  const dbPath = process.env.MEMAGENT_DB || "memories.db";

  try {
    const { spawn } = require("child_process");
    const result = spawn(memAgentBin, ["--db", dbPath, "stats"], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 3000,
    });

    let output = "";
    result.stdout.on("data", (data) => { output += data.toString(); });

    await new Promise((resolve) => result.on("close", resolve));

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
