#!/usr/bin/env node
// mem-agent session-start hook
// Called when OpenCode starts a new session.
// Queries mem-agent for recent memories via MCP tools.

import { resolveDbPath, resolveMemAgentBin, spawnMemAgent, waitForExit } from "./_memagent.mjs";

async function main() {
  const sessionId = process.env.OPENCODE_SESSION_ID || "unknown";
  console.log(`[mem-agent] Session started: ${sessionId}`);

  try {
    const result = spawnMemAgent(["list", "-n", "10"], {
      stdio: ["ignore", "pipe", "pipe"],
      timeout: 5000,
    });

    let output = "";
    result.stdout.on("data", (data) => { output += data.toString(); });
    result.stderr.on("data", (data) => { console.error(`[mem-agent] ${data}`); });

    await waitForExit(result);

    if (output.trim()) {
      console.log(`[mem-agent] Recent memories:\n${output.trim().split("\n").slice(0, 15).join("\n")}`);
    } else {
      console.log("[mem-agent] No recent memories found. Use /remember to save something.");
    }
  } catch (e) {
    console.log(
      `[mem-agent] Could not query ${resolveMemAgentBin()} --db ${resolveDbPath()}: ${e.message}`,
    );
  }
}

main().catch((error) => console.error(error));
