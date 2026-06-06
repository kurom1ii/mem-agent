#!/usr/bin/env node
// mem-agent session-start hook
// Called when OpenCode starts a new session.
// Queries mem-agent for recent memories via MCP tools.

const { spawn } = require("child_process");
const path = require("path");

async function main() {
  const sessionId = process.env.OPENCODE_SESSION_ID || "unknown";
  console.log(`[mem-agent] Session started: ${sessionId}`);

  const memAgentBin = process.env.MEMAGENT_BIN || "mem-agent";
  const dbPath = process.env.MEMAGENT_DB || "memories.db";

  try {
    const result = spawn(memAgentBin, ["--db", dbPath, "list", "-n", "10"], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 5000,
    });

    let output = "";
    result.stdout.on("data", (data) => { output += data.toString(); });
    result.stderr.on("data", (data) => { console.error(`[mem-agent] ${data}`); });

    await new Promise((resolve) => result.on("close", resolve));

    if (output.trim()) {
      console.log(`[mem-agent] Recent memories:\n${output.trim().split("\n").slice(0, 15).join("\n")}`);
    } else {
      console.log("[mem-agent] No recent memories found. Use /remember to save something.");
    }
  } catch (e) {
    console.log(`[mem-agent] Could not query (ensure mem-agent is in PATH): ${e.message}`);
  }
}

main().catch(console.error);
