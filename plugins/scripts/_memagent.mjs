#!/usr/bin/env node
import { existsSync } from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(SCRIPT_DIR, "..", "..");

export function resolveMemAgentBin() {
  const envBin = process.env.MEMAGENT_BIN?.trim();
  if (envBin) return envBin;

  const releaseBin = path.join(REPO_ROOT, "target", "release", "mem-agent");
  if (existsSync(releaseBin)) return releaseBin;

  const debugBin = path.join(REPO_ROOT, "target", "debug", "mem-agent");
  if (existsSync(debugBin)) return debugBin;

  return "mem-agent";
}

export function resolveDbPath() {
  return process.env.MEMAGENT_DB?.trim() || path.join(REPO_ROOT, "memories.db");
}

export function spawnMemAgent(args, options = {}) {
  return spawn(resolveMemAgentBin(), ["--db", resolveDbPath(), ...args], {
    env: process.env,
    ...options,
  });
}

export function waitForExit(child) {
  return new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code) => resolve(code ?? 0));
  });
}

