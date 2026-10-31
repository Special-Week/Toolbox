/* 由构建插件填充版本和资源表；开发服务器不注册此 Worker。 */
const VERSION = __TOOLBOX_VERSION__;
const FILES = __TOOLBOX_FILES__;
const PREFIX = "toolbox-web:" + self.registration.scope + ":";
const CACHE = PREFIX + VERSION;
const URLS = FILES.map((file) => new URL(file, self.registration.scope).href);
const ASSETS = new Set(URLS);
const INDEX = new URL("index.html", self.registration.scope).href;
const ASSET_PATH = new URL("assets/", self.registration.scope).pathname;

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE);
      try {
        await cache.addAll(
          URLS.map(
            (url) =>
              new Request(url, { cache: "reload", credentials: "same-origin" }),
          ),
        );
      } catch (error) {
        // 资源没有全部缓存时不得启用半成品版本；继续使用上一套可用资源。
        await caches.delete(CACHE);
        throw error;
      }
    })(),
  );
});
self.addEventListener("message", (event) => {
  if (event.data?.type === "APPLY_UPDATE") event.waitUntil(self.skipWaiting());
});
self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      // 保留两个旧版本供其他已打开页面使用，不触及同域其他应用的缓存。
      const old = (await caches.keys()).filter(
        (name) => name.startsWith(PREFIX) && name !== CACHE,
      );
      await Promise.all(old.slice(0, -2).map((name) => caches.delete(name)));
      await self.clients.claim();
    })(),
  );
});
self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  const scope = new URL(self.registration.scope);
  if (
    request.method !== "GET" ||
    url.origin !== scope.origin ||
    !url.pathname.startsWith(scope.pathname)
  )
    return;
  if (
    request.mode === "navigate" &&
    (url.pathname === scope.pathname ||
      url.pathname === new URL(INDEX).pathname)
  ) {
    // 同一活动版本的 HTML 和哈希资源配套；更新由页面提示后主动切换。
    event.respondWith(
      caches
        .open(CACHE)
        .then(async (cache) => (await cache.match(INDEX, { ignoreVary: true })) || fetch(request)),
    );
    return;
  }
  if (!ASSETS.has(url.href) && !url.pathname.startsWith(ASSET_PATH)) return;
  event.respondWith(
    (async () => {
      const current = await caches.open(CACHE);
      // 这里只缓存构建生成的静态文件，按完整 URL 匹配同一份内容。
      // 模块请求可能带 Origin，而预缓存请求没有；忽略这类 Vary 差异，
      // 防止更新移除旧资源后误走网络并取到 HTML 回退页面。
      const response = await current.match(request, { ignoreVary: true });
      if (response) return response;
      for (const name of (await caches.keys()).filter((name) =>
        name.startsWith(PREFIX),
      )) {
        const previous = await (await caches.open(name)).match(request, { ignoreVary: true });
        if (previous) return previous;
      }
      return fetch(request);
    })(),
  );
});
