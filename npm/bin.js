#!/usr/bin/env node
"use strict";

const { execFileSync } = require("child_process");
const path = require("path");

const isWindows = process.platform === "win32";
const binName = isWindows ? "convex-doctor.exe" : "convex-doctor";
const binPath = path.join(__dirname, "bin", binName);

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  if (err.status != null) {
    process.exit(err.status);
  }
  console.error(`Failed to run convex-doctor: ${err.message}`);
  console.error("Try reinstalling: npm install -g convex-doctor");
  process.exit(1);
}
