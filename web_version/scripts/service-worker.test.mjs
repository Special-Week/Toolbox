import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import vm from "node:vm";

const scope = "https://toolbox.example/app/";
const prefix = "toolbox-web:" + scope + ":";
const template = await readFile(new URL("./service-worker.js", import.meta.url), "utf8");

// 模拟 Cache API 的 Vary 匹配：预缓存请求没有 Origin，模块脚本请求带 Origin。
class StaticCache {
  entries = new Map();
  put(url, body) {
    this.entries.set(url, {
      request: new Request(url),
      response: new Response(body, { headers: { Vary: "Origin" } }),
    });
  }
  async match(input, options = {}) {
    const request = typeof input === "string" ? new Request(input) : input;
    const entry = this.entries.get(request.url);
    if (!entry) return undefined;
    if (!options.ignoreVary &&
        request.headers.get("Origin") !== entry.request.headers.get("Origin")) return undefined;
    return entry.response.clone();
  }
}

function worker() {
  const listeners = new Map();
  const storage = new Map();
  const requests = [];
  const context = {
    URL, Request, Response,
    self: {
      registration: { scope },
      addEventListener: (name, listener) => listeners.set(name, listener),
    },
    caches: {
      async open(name) {
        if (!storage.has(name)) storage.set(name, new StaticCache());
        return storage.get(name);
      },
      async keys() { return [...storage.keys()]; },
    },
    fetch: async request => {
      requests.push(request.url);
      return new Response("<html>server fallback after old asset removal</html>");
    },
  };
  vm.runInNewContext(
    template
      .replace("__TOOLBOX_VERSION__", JSON.stringify("current"))
      .replace("__TOOLBOX_FILES__", JSON.stringify(["index.html", "assets/current.js"])),
    context,
  );
  return {
    requests,
    async cache(version) { return context.caches.open(prefix + version); },
    async request(path, options = {}) {
      const request = new Request(new URL(path, scope), {
        headers: { Origin: new URL(scope).origin },
        ...options,
      });
      let response;
      listeners.get("fetch")({ request, respondWith(value) { response = value; } });
      return response;
    },
  };
}

test("module scripts use the complete cached version despite Vary: Origin", async () => {
  const app = worker();
  const cache = await app.cache("current");
  cache.put(new URL("assets/current.js", scope).href, "current script");
  const response = await app.request("assets/current.js");
  assert.equal(await response.text(), "current script");
  assert.deepEqual(app.requests, []);
});

test("open old pages can still load assets from a retained previous version", async () => {
  const app = worker();
  const cache = await app.cache("previous");
  cache.put(new URL("assets/previous.js", scope).href, "previous script");
  const response = await app.request("assets/previous.js");
  assert.equal(await response.text(), "previous script");
  assert.deepEqual(app.requests, []);
});

test("non-static requests keep the normal network path", async () => {
  const app = worker();
  assert.equal(await app.request("api/results"), undefined);
  assert.equal(await app.request("assets/current.js", { method: "POST" }), undefined);
  assert.deepEqual(app.requests, []);
});
