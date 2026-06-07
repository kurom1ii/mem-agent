#!/usr/bin/env node
// mem-agent session-end hook
// Called when OpenCode ends a session.
// Triggers memory stats summary.

import { spawnMemAgent, waitForExit } from "./_memagent.mjs";

async function main() {
  const sessionId = process.env.OPENCODE_SESSION_ID || "unknown";
  console.log(`[mem-agent] Session ended: ${sessionId}`);

  try {
    const result = spawnMemAgent(["stats"], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 5000,
    });

    let output = "";
    result.stdout.on("data", (data) => { output += data.toString(); });

    await waitForExit(result);

    if (output.trim()) {
      console.log(`[mem-agent] Final stats:\n${output.trim()}`);
    }
  } catch (e) {
    // silent fail — hooks shouldn't block session shutdown
  }
}

main().catch(() => {});
