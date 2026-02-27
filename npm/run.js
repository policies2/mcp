#!/usr/bin/env node
"use strict";

const { spawnSync } = require("child_process");
const path = require("path");

const isWindows = process.platform === "win32";
const binary = path.join(
  __dirname,
  "bin",
  isWindows ? "policy-mcp.exe" : "policy-mcp"
);

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      "policy-mcp binary not found. Try reinstalling: npm install @policies2/mcp"
    );
  } else {
    console.error("Failed to start policy-mcp:", result.error.message);
  }
  process.exit(1);
}

process.exit(result.status ?? 0);
