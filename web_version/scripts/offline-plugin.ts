/** 构建后按文件内容生成离线版本，确保页面、Worker 与 Wasm 作为同一套资源缓存。 */
import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import type { Plugin } from "vite";

export function offlinePlugin(): Plugin {
  let directory = "";
  let source = "";
  return {
    name: "toolbox-offline",
    apply: "build",
    configResolved(config) {
      directory = path.resolve(config.root, config.build.outDir);
      source = path.resolve(config.root, "scripts/service-worker.js");
    },
    async closeBundle() {
      async function files(folder: string): Promise<string[]> {
        const entries = await readdir(folder, { withFileTypes: true });
        return (
          await Promise.all(
            entries.map((entry) =>
              entry.isDirectory()
                ? files(path.join(folder, entry.name))
                : [path.join(folder, entry.name)],
            ),
          )
        ).flat();
      }
      const names = (await files(directory))
        .map((file) => path.relative(directory, file).replaceAll("\\", "/"))
        .filter((file) => file !== "sw.js")
        .sort();
      const hash = createHash("sha256");
      for (const file of names) {
        hash.update(file);
        hash.update(await readFile(path.join(directory, file)));
      }
      const template = await readFile(source, "utf8");
      hash.update(template);
      await writeFile(
        path.join(directory, "sw.js"),
        template
          .replace(
            "__TOOLBOX_VERSION__",
            JSON.stringify(hash.digest("hex").slice(0, 16)),
          )
          .replace("__TOOLBOX_FILES__", JSON.stringify(names)),
        "utf8",
      );
    },
  };
}
