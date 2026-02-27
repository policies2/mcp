#!/usr/bin/env node
"use strict";

const { test } = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("child_process");
const path = require("path");

const { getArtifactName, download } = require("./install.js");

// --- getArtifactName: all supported platforms ---

test("linux/x64 maps to linux-x86_64", () => {
  assert.equal(getArtifactName("linux", "x64"), "policy-mcp-linux-x86_64");
});

test("linux/arm64 maps to linux-aarch64", () => {
  assert.equal(getArtifactName("linux", "arm64"), "policy-mcp-linux-aarch64");
});

test("darwin/arm64 maps to macos-arm64", () => {
  assert.equal(getArtifactName("darwin", "arm64"), "policy-mcp-macos-arm64");
});

test("darwin/x64 maps to macos-x86_64", () => {
  assert.equal(getArtifactName("darwin", "x64"), "policy-mcp-macos-x86_64");
});

test("win32/x64 maps to windows-x86_64.exe", () => {
  assert.equal(getArtifactName("win32", "x64"), "policy-mcp-windows-x86_64.exe");
});

// --- getArtifactName: unsupported combinations ---

test("unsupported platform throws", () => {
  assert.throws(
    () => getArtifactName("freebsd", "x64"),
    /Unsupported platform\/arch: freebsd\/x64/
  );
});

test("unsupported arch throws", () => {
  assert.throws(
    () => getArtifactName("linux", "ia32"),
    /Unsupported platform\/arch: linux\/ia32/
  );
});

// --- download: redirect limit guard ---

test("download throws synchronously when redirectsLeft is 0", () => {
  assert.throws(
    () => download("https://example.com", "/tmp/unused", 0),
    /Too many redirects/
  );
});

// --- run.js: missing binary ---

test("run.js exits 1 with helpful message when binary is missing", () => {
  const result = spawnSync(process.execPath, ["run.js"], {
    cwd: __dirname,
    encoding: "utf8",
  });
  assert.equal(result.status, 1);
  assert.ok(
    result.stderr.includes("policy-mcp binary not found"),
    `expected ENOENT message, got: ${result.stderr}`
  );
});
