// 固定绑定工具版本，防止生成的 JavaScript 与 Rust/Wasm 协议不匹配。
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { mkdirSync } from "node:fs";
import path from "node:path";
import { syncLicenses } from "./sync-licenses.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
function run(command, args, capture = false) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: capture ? ["inherit", "pipe", "inherit"] : "inherit",
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
    shell: false,
  });
  if (result.error) {
    console.error(
      `无法运行 ${command}，请按 README 安装 Rust 与 wasm-bindgen-cli。`,
    );
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
  return result.stdout;
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
const messages = run("cargo", [
  "build",
  "--locked",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
  "--manifest-path",
  "core/Cargo.toml",
  "--message-format=json-render-diagnostics",
], true);
let wasm;
for (const line of messages.split(/\r?\n/)) {
  if (!line.startsWith("{")) continue;
  const message = JSON.parse(line);
  if (message.reason === "compiler-artifact" && message.target.name === "toolbox_web_core") {
    wasm = message.filenames.find((file) => file.endsWith(".wasm"));
  }
}
if (!wasm) {
  console.error("Cargo 构建未报告 toolbox_web_core 的 Wasm 产物。");
  process.exit(1);
}
run("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  "pkg",
  "--out-name",
  "toolbox_core",
  wasm,
]);
syncLicenses(root);
