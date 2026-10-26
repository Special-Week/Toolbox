// 固定绑定工具版本，防止生成的 JavaScript 与 Rust/Wasm 协议不匹配。
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { mkdirSync } from "node:fs";
import path from "node:path";
import { syncLicenses } from "./sync-licenses.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    shell: false,
  });
  if (result.error) {
    console.error(
      `无法运行 ${command}，请按 README 安装 Rust 与 wasm-bindgen-cli。`,
    );
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
}
mkdirSync(path.join(root, "pkg"), { recursive: true });
const bindingVersion = spawnSync("wasm-bindgen", ["--version"], {
  encoding: "utf8",
});
if (
  bindingVersion.error ||
  bindingVersion.status !== 0 ||
  bindingVersion.stdout.trim() !== "wasm-bindgen 0.2.114"
) {
  console.error(
    "需要 wasm-bindgen-cli 0.2.114，请运行：cargo install wasm-bindgen-cli --version 0.2.114 --locked",
  );
  process.exit(1);
}
run("cargo", [
  "build",
  "--locked",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
  "--manifest-path",
  "core/Cargo.toml",
]);
run("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  "pkg",
  "--out-name",
  "toolbox_core",
  "core/target/wasm32-unknown-unknown/release/toolbox_web_core.wasm",
]);
syncLicenses(root);
