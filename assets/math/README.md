# 公式排版资源

`formula-catalog.json` 和 `formula-patterns.json` 是三端共享的公式及匹配规则。Rust、C++ 和 Web 使用各自的数学排版引擎在运行时生成公式，无需预生成的公式图片。

修改共享 JSON 后，在 `cpp_version` 目录运行 `node scripts/sync-math-catalog.mjs`，并用 `node scripts/sync-math-catalog.mjs --check` 检查生成的 C++ 数据是否同步；普通 C++ 构建无需 Node.js。
