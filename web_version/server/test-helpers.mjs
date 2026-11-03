import { mkdtemp, readdir, unlink, rmdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

export const LINK = "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1059.1-2012";
export const NAME = "测量不确定度评定与表示";

// 包含正确对象偏移和交叉引用表，实际交给 PDF.js 解析。
export function pdf(pages = 2) {
  const objects = ["<< /Type /Catalog /Pages 2 0 R >>", `<< /Type /Pages /Kids [${Array.from({ length: pages }, (_, i) => `${i + 3} 0 R`).join(" ")}] /Count ${pages} >>`];
  for (let i = 0; i < pages; i++) objects.push("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>");
  let text = "%PDF-1.4\n";
  const offsets = [0];
  for (const [index, body] of objects.entries()) { offsets.push(text.length); text += `${index + 1} 0 obj\n${body}\nendobj\n`; }
  const start = text.length;
  text += `xref\n0 ${offsets.length}\n0000000000 65535 f \n` + offsets.slice(1).map(value => String(value).padStart(10, "0") + " 00000 n \n").join("");
  text += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${start}\n%%EOF\n`;
  return Buffer.from(text);
}

export function upstream(data = pdf()) {
  const calls = [];
  const fetchImpl = async (input, options) => {
    const url = new URL(input);
    calls.push({ url, ...options });
    if (url.pathname === "/resmea/standard/detail.html") return new Response(`<title>${NAME} - 国家计量技术规范全文公开系统</title>`, { headers: { "set-cookie": "JSESSIONID=test-session; Path=/; Secure; HttpOnly" } });
    if (url.pathname === "/resmea/view/stdonline") return new Response('var token = "dG9rZW4rLw=="; var enc = "ZW5jLys=";');
    if (url.pathname === "/resmea/view/onlinereading") return new Response(data, { headers: { "content-type": "application/pdf" } });
    throw new Error("Unexpected upstream URL: " + url);
  };
  return { fetchImpl, calls };
}

export async function temporary(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), "toolbox-pdf-test-"));
  t.after(async () => {
    // 测试目录只创建平铺文件，逐个清理，避免递归删除。
    for (const file of await readdir(directory)) await unlink(path.join(directory, file));
    await rmdir(directory);
  });
  return directory;
}
