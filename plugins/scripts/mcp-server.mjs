#!/usr/bin/env node
import { spawnMemAgent, waitForExit } from "./_memagent.mjs";

async function main() {
  const child = spawnMemAgent(["mcp"], {
    stdio: "inherit",
  });

  const code = await waitForExit(child);
  process.exit(code);
}

main().catch((error) => {
  console.error(`[mem-agent] MCP wrapper failed: ${error.message}`);
  process.exit(1);
});
