#!/usr/bin/env node
"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");

const pkg = JSON.parse(
  fs.readFileSync(path.join(__dirname, "package.json"), "utf8")
);
const version = pkg.version;

function getArtifactName(platform = process.platform, arch = process.arch) {
  if (platform === "linux" && arch === "x64") return "policy-mcp-linux-x86_64";
  if (platform === "linux" && arch === "arm64") return "policy-mcp-linux-aarch64";
  if (platform === "darwin" && arch === "arm64") return "policy-mcp-macos-arm64";
  if (platform === "darwin" && arch === "x64") return "policy-mcp-macos-x86_64";
  if (platform === "win32" && arch === "x64") return "policy-mcp-windows-x86_64.exe";

  throw new Error(`Unsupported platform/arch: ${platform}/${arch}`);
}

function download(url, dest, redirectsLeft) {
  if (redirectsLeft === 0) {
    throw new Error("Too many redirects");
  }
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        resolve(download(res.headers.location, dest, redirectsLeft - 1));
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`Download failed with status ${res.statusCode}: ${url}`));
        return;
      }
      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", (err) => {
        fs.unlink(dest, () => {});
        reject(err);
      });
    }).on("error", reject);
  });
}

async function main() {
  const artifact = getArtifactName();
  const url = `https://github.com/policies2/mcp/releases/download/v${version}/${artifact}`;

  const binDir = path.join(__dirname, "bin");
  fs.mkdirSync(binDir, { recursive: true });

  const isWindows = process.platform === "win32";
  const dest = path.join(binDir, isWindows ? "policy-mcp.exe" : "policy-mcp");

  console.log(`Downloading policy-mcp v${version} for ${process.platform}/${process.arch}...`);
  await download(url, dest, 5);

  if (!isWindows) {
    fs.chmodSync(dest, 0o755);
  }

  console.log(`policy-mcp installed to ${dest}`);
}

if (require.main === module) {
  main().catch((err) => {
    console.error("Failed to install policy-mcp:", err.message);
    process.exit(1);
  });
}

module.exports = { getArtifactName, download };
