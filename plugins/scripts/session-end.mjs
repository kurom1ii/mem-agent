#!/usr/bin/env node
// mem-agent session-end hook
// Called when OpenCode ends a session.
// Triggers memory stats summary.

const { spawn } = require("child_process");

async function main() {
  const sessionId = process.env.OPENCODE_SESSION_ID || "unknown";
  console.log(`[mem-agent] Session ended: ${sessionId}`);

  const memAgentBin = process.env.MEMAGENT_BIN || "mem-agent";
  const dbPath = process.env.MEMAGENT_DB || "memories.db";

  try {
    const result = spawn(memAgentBin, ["--db", dbPath, "stats"], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 5000,
    });

    let output = "";
    result.stdout.on("data", (data) => { output += data.toString(); });

    await new Promise((resolve) => result.on("close", resolve));

    if (output.trim()) {
      console.log(`[mem-agent] Final stats:\n${output.trim()}`);
    }
  } catch (e) {
    // silent fail — hooks shouldn't block session shutdown
  }
}

main().catch(() => {});
