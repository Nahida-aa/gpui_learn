#!/usr/bin/env bun
// 构建 Web 资产：wasm-bindgen + 复制 index.html
// 替代 package.json 中过长的 build:assets 命令

import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { join } from 'path';

const wasmBin = "../../target/wasm32-unknown-unknown/debug/hello_world_01.wasm";
const outDir = "./dist";
const outName = "hello_world_01";
const indexSrc = "./index.html";
const indexDest = `${outDir}/index.html`;

// Ensure dist directory exists
if (!existsSync(outDir)) {
  mkdirSync(outDir, { recursive: true });
}

console.log("==> Running wasm-bindgen...");
execFileSync("wasm-bindgen", [
  wasmBin,
  "--target", "web",
  "--no-typescript",
  "--out-dir", outDir,
  "--out-name", outName,
]);

console.log("==> Copying index.html...");
copyFileSync(indexSrc, indexDest);

console.log("✓ Build assets complete. Outputs in ./dist/");
