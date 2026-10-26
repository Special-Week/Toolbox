<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
defineProps<{ busy: boolean }>();
const emit = defineEmits<{ beforeUpdate: [] }>();
interface InstallPrompt extends Event {
  prompt(): Promise<unknown>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed" }>;
}
const available = ref(false);
const installed = ref(false);
const ready = ref(false);
const online = ref(true);
const update = ref(false);
const applying = ref(false);
const guide = ref(false);
const notice = ref("");
let prompt: InstallPrompt | null = null;
let registration: ServiceWorkerRegistration | undefined;
let disposed = false;
const observed = new Map<ServiceWorker, () => void>();
function installEvent(event: Event) {
  event.preventDefault();
  prompt = event as InstallPrompt;
}
function installedEvent() {
  installed.value = true;
  prompt = null;
  guide.value = false;
}
function connectivity() {
  online.value = navigator.onLine;
  if (online.value) void registration?.update().catch(() => {});
}
function changed() {
  if (applying.value) window.location.reload();
  else if (ready.value) update.value = true;
}
function foundUpdate() {
  const worker = registration?.installing;
  if (worker) observe(worker);
}
function observe(worker: ServiceWorker) {
  if (observed.has(worker)) return;
  const state = () => {
    if (
      worker.state === "installed" &&
      registration?.active &&
      registration.active !== worker
    )
      update.value = true;
    if (worker.state === "activated") ready.value = true;
    if (worker.state === "redundant")
      notice.value = "本次离线资源更新未完成，联网仍可正常使用。";
  };
  observed.set(worker, state);
  worker.addEventListener("statechange", state);
  state();
}
async function install() {
  if (!prompt) {
    guide.value = !guide.value;
    return;
  }
  const event = prompt;
  prompt = null;
  try {
    await event.prompt();
    await event.userChoice;
  } catch {
    guide.value = true;
  }
}
function applyUpdate() {
  if (!registration) return;
  // 父页面同步保存输入后才切换版本，避免刷新时丢失刚填写的数据。
  emit("beforeUpdate");
  applying.value = true;
  if (registration.waiting)
    registration.waiting.postMessage({ type: "APPLY_UPDATE" });
  else window.location.reload();
}
onMounted(async () => {
  if (
    !import.meta.env.PROD ||
    !window.isSecureContext ||
    !("serviceWorker" in navigator)
  )
    return;
  available.value = true;
  installed.value =
    window.matchMedia("(display-mode: standalone)").matches ||
    !!(navigator as Navigator & { standalone?: boolean }).standalone;
  connectivity();
  window.addEventListener("beforeinstallprompt", installEvent);
  window.addEventListener("appinstalled", installedEvent);
  window.addEventListener("online", connectivity);
  window.addEventListener("offline", connectivity);
  navigator.serviceWorker.addEventListener("controllerchange", changed);
  try {
    const base = new URL(import.meta.env.BASE_URL, location.href);
    registration = await navigator.serviceWorker.register(
      new URL("sw.js", base),
      { scope: base.pathname, updateViaCache: "none" },
    );
    if (disposed) return;
    update.value = !!registration.waiting && !!registration.active;
    if (registration.active) observe(registration.active);
    registration.addEventListener("updatefound", foundUpdate);
    foundUpdate();
    void registration.update().catch(() => {});
  } catch {
    notice.value = "离线缓存暂不可用，联网仍可正常使用。";
  }
});
onUnmounted(() => {
  disposed = true;
  window.removeEventListener("beforeinstallprompt", installEvent);
  window.removeEventListener("appinstalled", installedEvent);
  window.removeEventListener("online", connectivity);
  window.removeEventListener("offline", connectivity);
  navigator.serviceWorker?.removeEventListener("controllerchange", changed);
  registration?.removeEventListener("updatefound", foundUpdate);
  for (const [worker, callback] of observed)
    worker.removeEventListener("statechange", callback);
  observed.clear();
});
</script>
<template>
  <section v-if="available" class="pwa-status" aria-label="应用与离线状态">
    <div class="pwa-toolbar">
      <span role="status">{{
        notice ||
        (ready
          ? online
            ? "离线资源已就绪"
            : "离线使用中"
          : "正在准备离线资源…")
      }}</span>
      <button
        v-if="!installed"
        type="button"
        class="button subtle"
        @click="install"
      >
        添加到主屏幕
      </button>
    </div>
    <div v-if="update" class="pwa-update" role="status">
      <span>新版本已就绪，更新将保留当前输入。</span>
      <button
        type="button"
        class="button secondary"
        :disabled="busy || applying"
        @click="applyUpdate"
      >
        {{ applying ? "正在更新…" : "更新并刷新" }}
      </button>
    </div>
    <aside v-if="guide" class="install-guide">
      <strong>添加到主屏幕</strong>
      <p>iPhone / iPad：在 Safari 的分享菜单中选择“添加到主屏幕”。</p>
      <p>
        Android：在浏览器菜单中选择“安装应用”或“添加到主屏幕”。微信、QQ
        等内置浏览器可先选择“在浏览器打开”。
      </p>
      <p>
        离线资源准备完成后，可在断网时继续使用；浏览器清理网站数据后需要联网重新加载。
      </p>
      <button type="button" class="button subtle" @click="guide = false">
        关闭说明
      </button>
    </aside>
  </section>
</template>
