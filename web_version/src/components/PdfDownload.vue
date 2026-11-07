<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from "vue";
import { download } from "../format";
import { searchStandards, standards, type Standard } from "../standard-catalog";

const props = defineProps<{ active: boolean }>();
interface PdfResult { code: string; name: string; pages: number; filename: string; cache: "HIT" | "MISS" }
interface WritableFile { write(data: Blob): Promise<void>; close(): Promise<void>; abort(): Promise<void> }
type PickerWindow = Window & { showSaveFilePicker?: (options: unknown) => Promise<{ name: string; createWritable(): Promise<WritableFile> }> };
const url = ref("");
const query = ref("");
const page = ref(0);
const matches = computed(() => searchStandards(query.value));
const pageCount = computed(() => Math.max(1, Math.ceil(matches.value.length / 10)));
const visibleStandards = computed(() => matches.value.slice(page.value * 10, (page.value + 1) * 10));
const selectedStandard = computed(() => standards.find(standard => standard.url === url.value));
watch(query, () => { page.value = 0; });
const busy = ref(false);
const phase = ref("");
const error = ref("");
const message = ref("");
const result = ref<PdfResult | null>(null);
const warningOpen = ref(false);
const successPending = ref(false);
const successTitle = ref("PDF 下载已发起");
const warningDialog = ref<HTMLDialogElement>();
const successDialog = ref<HTMLDialogElement>();
let controller: AbortController | null = null;
let cachedBlob: Blob | null = null;
let previousFocus: HTMLElement | null = null;

function validLink(input: string) {
  const link = new URL(input.trim());
  const codes = link.searchParams.getAll("standno");
  if (link.origin !== "https://jjg.spc.org.cn" || link.username || link.password || /[\x00-\x1f]/.test(input) || input.length > 2000 ||
      link.pathname !== "/resmea/standard/detail.html" || codes.length !== 1 || !codes[0].trim() || !/^[A-Za-z0-9/ ._-]{1,100}$/.test(codes[0])) throw new Error("请输入有效的规范详情页链接。");
  return link.href;
}
async function requestDownload() {
  if (busy.value) return;
  error.value = "";
  try { validLink(url.value); }
  catch { error.value = "请从搜索结果中选择要下载的规范。"; return; }
  previousFocus = document.activeElement as HTMLElement | null;
  warningOpen.value = true;
  await nextTick();
  warningDialog.value?.showModal();
}
function chooseStandard(standard: Standard) {
  if (busy.value) return;
  url.value = standard.url;
  requestDownload();
}
function closeWarning() {
  warningOpen.value = false;
  warningDialog.value?.close();
  previousFocus?.focus();
}
async function start() {
  if (busy.value) return;
  closeWarning();
  busy.value = true;
  phase.value = "正在查询服务器缓存，未命中时将获取并保存 PDF…";
  error.value = ""; message.value = ""; result.value = null; cachedBlob = null;
  controller = new AbortController();
  const timer = setTimeout(() => controller?.abort(), 240000);
  try {
    const response = await fetch("/api/pdf?" + new URLSearchParams({ url: validLink(url.value) }), { signal: controller.signal, cache: "no-store" });
    if (!response.ok) {
      let detail = "";
      try { const body = await response.json(); detail = typeof body.error === "string" ? body.error : ""; } catch { /* 非 JSON 错误使用状态码提示。 */ }
      if (!detail && response.status === 404) throw new Error("当前网站未提供 PDF 下载接口（/api/pdf）。请使用完整构建包中的 python serve.py 启动服务，并确认已上传配套后端目录。");
      throw new Error(detail || "PDF 服务返回 HTTP " + response.status + "，请确认服务器已启动。");
    }
    if (!response.headers.get("content-type")?.toLowerCase().startsWith("application/pdf")) throw new Error("PDF 服务未返回文件，请确认已启动提供 /api/pdf 的工具箱服务器。");
    const pages = Number(response.headers.get("x-pdf-pages"));
    const cache = response.headers.get("x-pdf-cache");
    const code = decodeURIComponent(response.headers.get("x-pdf-code") || "");
    const name = decodeURIComponent(response.headers.get("x-pdf-name") || "");
    const filename = decodeURIComponent(response.headers.get("x-pdf-filename") || "");
    if (!code || !filename || !Number.isInteger(pages) || pages < 1 || (cache !== "HIT" && cache !== "MISS")) throw new Error("服务器返回的 PDF 信息不完整。");
    const blob = await response.blob();
    if (blob.size > 100 * 1024 * 1024 || await blob.slice(0, 5).text() !== "%PDF-") throw new Error("服务器返回的 PDF 文件无效。");
    cachedBlob = blob;
    result.value = { code, name, pages, filename, cache };
    browserSave();
  } catch (e) {
    error.value = e instanceof Error && e.name === "AbortError" ? "请求超时，请稍后重试；服务器完成的 PDF 仍会保留在缓存中。" : e instanceof Error ? e.message : "下载失败，请检查网络与服务器。";
  } finally { clearTimeout(timer); controller = null; busy.value = false; phase.value = ""; }
}
function browserSave() {
  if (!cachedBlob || !result.value) return;
  download(cachedBlob, result.value.filename);
  successTitle.value = "PDF 下载已发起";
  message.value = "已交给浏览器下载：" + result.value.filename + "。请在下载列表确认保存结果。";
  successPending.value = true;
  showSuccess();
}
async function saveAgain() {
  if (busy.value || !cachedBlob || !result.value) return;
  const picker = (window as PickerWindow).showSaveFilePicker;
  if (!picker) { browserSave(); return; }
  let writer: WritableFile | undefined;
  busy.value = true;
  try {
    const handle = await picker.call(window, { suggestedName: result.value.filename, types: [{ description: "PDF 文件", accept: { "application/pdf": [".pdf"] } }] });
    writer = await handle.createWritable();
    await writer.write(cachedBlob); await writer.close(); writer = undefined;
    error.value = ""; successTitle.value = "保存成功"; message.value = "已保存：" + handle.name;
    successPending.value = true; showSuccess();
  } catch (e) {
    if (writer) await writer.abort().catch(() => {});
    if (e instanceof Error && e.name === "AbortError") message.value = "已取消保存，可再次点击保存 PDF。";
    else error.value = e instanceof Error ? e.message : "无法保存到所选位置。";
  } finally { busy.value = false; }
}
async function showSuccess() {
  if (!props.active || !successPending.value) return;
  await nextTick();
  if (!successDialog.value?.open) successDialog.value?.showModal();
}
function closeSuccess() { successPending.value = false; successDialog.value?.close(); }
function reset() {
  if (busy.value) return;
  closeWarning(); closeSuccess();
  url.value = ""; query.value = ""; page.value = 0;
  error.value = ""; message.value = ""; result.value = null; cachedBlob = null;
}
watch(() => props.active, active => {
  if (active) showSuccess();
  else { closeWarning(); successDialog.value?.close(); }
});
onUnmounted(() => controller?.abort());
defineExpose({ reset, busy });
</script>

<template>
  <div class="pdf-tool">
    <section class="panel pdf-input-panel">
      <h2><label for="metrology-pdf-search">规范名称或编号</label></h2>
      <p class="pdf-muted">输入名称关键词或规范编号筛选，在结果中选择需要下载的规范。</p>
      <p class="pdf-muted">数据来源：<a href="https://jjg.spc.org.cn/" target="_blank" rel="noopener noreferrer">https://jjg.spc.org.cn/</a>，截至 2026-11 的规范目录。</p>
      <input id="metrology-pdf-search" v-model="query" class="pdf-link-input" type="search" :disabled="busy" autocomplete="off" spellcheck="false" />
      <div class="pdf-actions">
        <a class="button secondary pdf-button" href="https://jjg.spc.org.cn/resmea/" target="_blank" rel="noopener noreferrer">打开规范网站</a>
        <span v-if="busy" class="pdf-muted" role="status">{{ phase }}</span>
      </div>
    </section>
    <section v-if="query.trim()" class="pdf-search-results" aria-label="规范搜索结果">
      <h2 role="status">找到 {{ matches.length }} 条规范</h2>
      <p v-if="!matches.length" class="pdf-muted">未找到匹配的规范，请尝试其他名称或编号。</p>
      <div class="pdf-standard-list">
        <article v-for="standard in visibleStandards" :key="standard.code" class="panel pdf-standard">
          <div class="pdf-standard-info">
            <strong class="pdf-standard-code">{{ standard.code }}</strong>
            <h3>{{ standard.name }}</h3>
            <p class="pdf-standard-meta">{{ standard.category }}</p>
            <p class="pdf-standard-meta"><span :class="{ 'pdf-old-status': ['被代替', '废止'].includes(standard.status) }">状态：{{ standard.status }}</span>　发布：{{ standard.published || '未提供' }}　实施：{{ standard.effective || '未提供' }}</p>
          </div>
          <button type="button" class="button primary pdf-button" :disabled="busy" :aria-label="'下载 ' + standard.code + ' ' + standard.name" @click="chooseStandard(standard)">下载 PDF</button>
        </article>
      </div>
      <div v-if="matches.length" class="pdf-pagination">
        <button type="button" class="button secondary" :disabled="busy || page === 0" @click="page--">上一页</button>
        <span class="pdf-muted">第 {{ page + 1 }} / {{ pageCount }} 页</span>
        <button type="button" class="button secondary" :disabled="busy || page + 1 >= pageCount" @click="page++">下一页</button>
      </div>
      <p class="pdf-muted">规范状态与实施日期请以规范网站为准。</p>
    </section>
    <p v-if="error" class="notice pdf-error" role="alert">{{ error }}</p>
    <section v-if="result" class="pdf-results" aria-label="下载结果">
      <h2>下载结果</h2>
      <div class="pdf-metrics">
        <div class="panel"><span>规范编号</span><strong>{{ result.code }}</strong></div>
        <div class="panel"><span>规范名称</span><strong>{{ result.name || "未获取到名称，以文件内容为准" }}</strong></div>
        <div class="panel"><span>PDF 页数</span><strong>{{ result.pages }} 页</strong></div>
      </div>
      <p v-if="result.pages === 1" class="pdf-error">PDF 只有一页，请打开检查是否为网站提示文件。</p>
      <button type="button" class="button secondary pdf-button pdf-save" :disabled="busy" @click="saveAgain">保存 PDF…</button>
    </section>
    <p v-if="message" class="pdf-muted" role="status">{{ message }}</p>
    <section class="panel pdf-explanation">
      <h2>获取流程与接口说明</h2>
      <p>1. 按名称或编号搜索内置目录，选择规范后将其详情页链接发送给服务器。服务器按 standno 规范编号匹配缓存。</p>
      <p>2. 优先读取服务器本地 PDF；文件有效就直接发送，不访问规范网站。</p>
      <p>3. 未命中缓存时，服务器读取规范名称，再提交 a100（规范编号）和 ismobile=1，获取 token 与 enc。</p>
      <code>POST https://jjg.spc.org.cn/resmea/view/stdonline</code>
      <p>4. 服务器携带同一会话的 Cookie，使用 token 请求文件，enc 作为 myfoxit 请求头获取 PDF。</p>
      <code>GET https://jjg.spc.org.cn/resmea/view/onlinereading?token=…</code>
      <p>5. 校验 PDF 格式并解析真实页数，完整保存 PDF 和规范信息后发送给浏览器；相同规范的并发请求合并获取。</p>
      <p class="pdf-muted">PDF 缓存保存在服务器磁盘，重启后仍可使用，不会自动过期。浏览器下载后请确认本地保存结果。单个 PDF 上限 100 MB，请按网站的电子文本使用说明使用文件。</p>
    </section>
    <dialog ref="warningDialog" class="pdf-dialog" aria-labelledby="pdf-warning-title" @cancel.prevent="closeWarning">
      <h2 id="pdf-warning-title">下载提示</h2>
      <template v-if="selectedStandard">
        <p><strong>{{ selectedStandard.code }} {{ selectedStandard.name }}</strong></p>
        <p class="pdf-muted">{{ selectedStandard.status }} · 实施日期：{{ selectedStandard.effective || '未提供' }}</p>
      </template>
      <p>此链接将发送给工具箱服务器，优先使用本地缓存；未命中时由服务器访问规范网站。频繁获取不同规范可能触发网站限流（HTTP 429）。</p>
      <p class="pdf-muted">请按网站的电子文本使用说明使用下载文件。</p>
      <div class="pdf-dialog-actions"><button type="button" class="button secondary pdf-button" autofocus @click="closeWarning">取消</button><button type="button" class="button primary pdf-button" @click="start">知道了，继续下载</button></div>
    </dialog>
    <dialog ref="successDialog" class="pdf-dialog" aria-labelledby="pdf-success-title" @cancel.prevent="closeSuccess">
      <h2 id="pdf-success-title">{{ successTitle }}</h2>
      <p>{{ result?.code }} {{ result?.name }}</p><p>PDF 页数：{{ result?.pages }} 页</p><p>{{ message }}</p>
      <p v-if="result?.pages === 1" class="pdf-error">PDF 只有一页，请打开检查是否为网站提示文件。</p>
      <div class="pdf-dialog-actions"><button type="button" class="button primary pdf-button" autofocus @click="closeSuccess">确定</button></div>
    </dialog>
  </div>
</template>

<style scoped>
.pdf-tool { max-width: 960px; }
.pdf-tool h2 { font-size: 15px; margin: 0 0 10px; }
.pdf-input-panel { border-radius: 8px; padding: 18px; }
.pdf-search-results { margin: 18px 0; }
.pdf-standard-list { display: grid; grid-auto-rows: 1fr; gap: 10px; }
.pdf-standard { display: flex; align-items: center; justify-content: space-between; gap: 18px; padding: 16px; border-radius: 8px; margin: 0; }
.pdf-standard-info { min-width: 0; }
.pdf-standard-code { color: var(--blue); font-size: 14px; }
.pdf-standard h3 { margin: 6px 0; font-size: 15px; font-weight: 500; overflow-wrap: anywhere; }
.pdf-standard-meta { margin: 5px 0 0; color: var(--muted); font-size: 12px; line-height: 1.6; }
.pdf-old-status { color: #a36022; }
.pdf-pagination { display: flex; align-items: center; justify-content: center; gap: 14px; margin: 14px 0; }
.pdf-muted { color: var(--muted); font-size: 12px; line-height: 1.7; }
.pdf-link-input { display: block; width: 100%; min-height: 44px; margin: 12px 0; padding: 10px 12px; border: 1px solid #a9bddb; border-radius: 6px; background: white; font-size: 14px; }
.pdf-link-input:focus { border-color: var(--blue); outline: 2px solid #2769dc20; outline-offset: 1px; }
.pdf-actions { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.pdf-button { min-width: 112px; min-height: 40px; font-size: 14px; padding: 10px 16px; border-radius: 6px; }
.pdf-button.secondary { background: white; border-color: #cfd9e8; color: #233043; }
.pdf-explanation { background: var(--soft); border: 0; border-radius: 8px; padding: 18px; margin-top: 16px; font-size: 13px; line-height: 1.8; }
.pdf-explanation h2 { color: var(--blue); font-size: 14px; }
.pdf-explanation p { margin: 8px 0; }
.pdf-explanation code { display: block; overflow-wrap: anywhere; color: #61728b; font-size: 12px; }
.pdf-results h2 { margin-bottom: 10px; }
.pdf-save { margin-top: 12px; }
.pdf-metrics { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
.pdf-metrics .panel { margin: 0; padding: 14px; border-radius: 6px; }
.pdf-metrics span, .pdf-metrics strong { display: block; }
.pdf-metrics span { font-size: 12px; color: var(--muted); }
.pdf-metrics strong { font-size: 15px; font-weight: 500; color: var(--blue); margin-top: 6px; overflow-wrap: anywhere; }
.pdf-error { color: #b74846; margin: 12px 0; font-size: 13px; }
.pdf-dialog { width: min(488px, calc(100vw - 32px)); max-height: calc(100dvh - 48px); overflow: auto; margin: auto; padding: 24px; border: 1px solid #dce3ed; border-radius: 10px; background: white; color: #233043; box-shadow: 0 12px 35px #142d4b2e; }
.pdf-dialog::backdrop { background: #0006; }
.pdf-dialog h2 { font-size: 19px; margin: 0 0 20px; }
.pdf-dialog p { font-size: 14px; line-height: 1.8; margin: 8px 0; overflow-wrap: anywhere; }
.pdf-dialog-actions { display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px; flex-wrap: wrap; }
@media (max-width: 600px) { .pdf-metrics { grid-template-columns: 1fr; } .pdf-button { min-height: 44px; } .pdf-dialog { padding: 20px; } .pdf-standard { align-items: stretch; flex-direction: column; gap: 12px; } }
</style>
