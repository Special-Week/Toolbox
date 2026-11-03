import { fileURLToPath } from "node:url";

process.env.TOOLBOX_STATIC_DIRECTORY ||= fileURLToPath(new URL("./", import.meta.url));
process.env.TOOLBOX_PDF_CACHE ||= fileURLToPath(new URL("./.pdf-cache/", import.meta.url));
await import("./server/index.mjs");
