// 将实际构建依赖的许可证随静态网站发布，保留各组件的版权声明。
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

export function syncLicenses(webRoot) {
  const repository = path.resolve(webRoot, "..");
  const sections = [
    "Toolbox 网页版 — 开源许可",
    "项目自有代码及图标：MIT",
    readFileSync(path.join(repository, "LICENSE"), "utf8"),
    "Vue 与 @vue/* 运行时：MIT",
    readFileSync(path.join(webRoot, "node_modules/vue/LICENSE"), "utf8"),
    "Rust/WebAssembly 依赖：以下许可来自 core/Cargo.lock 对应版本。",
  ];
  const registry = path.join(
    process.env.CARGO_HOME || path.join(os.homedir(), ".cargo"),
    "registry",
    "src",
  );
  const registries = existsSync(registry)
    ? readdirSync(registry, { withFileTypes: true })
        .filter((e) => e.isDirectory())
        .map((e) => path.join(registry, e.name))
    : [];
  const lock = readFileSync(path.join(webRoot, "core/Cargo.lock"), "utf8");
  for (const block of lock.split("[[package]]").slice(1)) {
    const name = block.match(/^name = "([^"]+)"/m)?.[1];
    const version = block.match(/^version = "([^"]+)"/m)?.[1];
    if (!name || !version || !block.includes("registry+")) continue;
    const folder = registries
      .map((r) => path.join(r, `${name}-${version}`))
      .find(existsSync);
    if (!folder)
      throw new Error(
        `未找到 ${name} ${version} 的许可，请先使用默认 Cargo 注册表下载依赖。`,
      );
    const files = readdirSync(folder, { withFileTypes: true }).filter(
      (e) => e.isFile() && /^(LICENSE|COPYING)([-._]|$)/i.test(e.name),
    );
    for (const file of files)
      sections.push(
        `${name} ${version} / ${file.name}`,
        readFileSync(path.join(folder, file.name), "utf8"),
      );
  }
  sections.push(
    "既有参考实现许可：",
    readFileSync(
      path.join(repository, "licenses/PsychroLib-LICENSE.txt"),
      "utf8",
    ),
  );
  writeFileSync(
    path.join(webRoot, "public/licenses.txt"),
    sections.join("\n\n"),
    "utf8",
  );
}
