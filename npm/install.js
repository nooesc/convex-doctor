#!/usr/bin/env node
"use strict";

const https = require("https");
const http = require("http");
const fs = require("fs");
const path = require("path");
const PLATFORM_METADATA = "platform.json";

const VERSION = require("./package.json").version;

const PLATFORM_MAP = {
  "darwin-arm64": "convex-doctor-aarch64-darwin",
  "darwin-x64": "convex-doctor-x86_64-darwin",
  "linux-x64": "convex-doctor-x86_64-linux",
  "linux-arm64": "convex-doctor-aarch64-linux",
  "win32-x64": "convex-doctor-x86_64-windows.exe",
};

function getPlatformBinary() {
  const key = `${process.platform}-${process.arch}`;
  const binary = PLATFORM_MAP[key];
  if (!binary) {
    console.error(
      `Unsupported platform: ${process.platform}-${process.arch}\n` +
        `Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`
    );
    process.exit(1);
  }
  return binary;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const client = url.startsWith("https") ? https : http;
    client
      .get(url, { headers: { "User-Agent": "convex-doctor-npm" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return download(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`Download failed: HTTP ${res.statusCode} from ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

function isBinaryCompatible(outputPath, metadataPath, version, platformBinary, platformKey) {
  if (!fs.existsSync(outputPath) || !fs.existsSync(metadataPath)) {
    return false;
  }

  try {
    const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
    return (
      metadata.version === version &&
      metadata.platform === platformKey &&
      metadata.binary === platformBinary
    );
  } catch (err) {
    return false;
  }
}

async function main() {
  const binaryName = getPlatformBinary();
  const url = `https://github.com/nooesc/convex-doctor/releases/download/v${VERSION}/${binaryName}`;
  const platform = `${process.platform}-${process.arch}`;

  const binDir = path.join(__dirname, "bin");
  const isWindows = process.platform === "win32";
  const outputName = isWindows ? "convex-doctor.exe" : "convex-doctor";
  const outputPath = path.join(binDir, outputName);
  const metadataPath = path.join(binDir, PLATFORM_METADATA);

  // Skip if binary already exists and matches the requested platform/version.
  if (isBinaryCompatible(outputPath, metadataPath, VERSION, binaryName, platform)) {
    return;
  }

  for (const p of [outputPath, metadataPath]) {
    if (fs.existsSync(p)) {
      try {
        fs.unlinkSync(p);
      } catch (err) {
        // If cleanup fails, keep going; write will overwrite or fail with
        // a clear error, which is easier to diagnose.
      }
    }
  }

  console.log(`Downloading convex-doctor v${VERSION} for ${process.platform}-${process.arch}...`);

  fs.mkdirSync(binDir, { recursive: true });

  try {
    const data = await download(url);
    fs.writeFileSync(outputPath, data);
    fs.writeFileSync(
      metadataPath,
      JSON.stringify({
        version: VERSION,
        platform,
        binary: binaryName,
        writtenAt: new Date().toISOString(),
      })
    );
    if (!isWindows) {
      fs.chmodSync(outputPath, 0o755);
    }
    console.log("convex-doctor installed successfully.");
  } catch (err) {
    console.error(`Failed to download convex-doctor: ${err.message}`);
    console.error(`URL: ${url}`);
    console.error(
      "\nYou can install manually from: https://github.com/nooesc/convex-doctor/releases"
    );
    process.exit(1);
  }
}

main();
