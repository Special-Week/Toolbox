// 从三端共用的公式目录生成 C++ 头文件；发布构建不需要 Node.js。
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const quote = (s) => {
  if (typeof s !== "string" || s.includes(')MATH"')) throw new Error("公式目录包含无法生成的文本");
  return `LR"MATH(${s})MATH"`;
};
for (const name of ["catalog", "patterns"]) {
  const input = fileURLToPath(new URL(`../../assets/math/formula-${name}.json`, import.meta.url));
  const output = fileURLToPath(new URL(`../include/toolbox/generated_formula_${name}.inc`, import.meta.url));
  const entries = JSON.parse(readFileSync(input, "utf8"));
  const generated = "// 由 scripts/sync-math-catalog.mjs 从共享公式目录生成。\n" +
    entries.map(([source, latex]) => `    {${quote(source)}, ${quote(latex)}},`).join("\n") + "\n";
  if (process.argv.includes("--check")) {
    if (readFileSync(output, "utf8") !== generated) throw new Error(`C++ 公式 ${name} 未同步`);
  } else writeFileSync(output, generated, "utf8");
}
