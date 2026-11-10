<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { Catalog, State, Tool } from "../types";
import type { Engine } from "../engine";
import { ARCHIVE_STORAGE_KEY, calibrationFields, readCalibrationArchives, writeCalibrationArchives, type CalibrationArchive } from "../calibration-archives";

const props = defineProps<{ tool: Tool; state: State; catalog: Catalog; engine: Engine }>();
const emit = defineEmits<{ apply: [archive: CalibrationArchive] }>();
const entries = ref<CalibrationArchive[]>([]);
const selected = ref("");
const error = ref("");
const message = ref("");
const dialogError = ref("");
const dialog = ref<HTMLDialogElement>();
const nameInput = ref<HTMLInputElement>();
const mode = ref<"save" | "overwrite" | "update" | "delete">("save");
const name = ref("");
const busy = ref(false);
let serial = 0;
let mounted = true;
let previousFocus: HTMLElement | null = null;
const selectedArchive = computed(() => entries.value.find(entry => entry.tool === props.tool.id && entry.name === selected.value));
const dirty = computed(() => {
  const entry = selectedArchive.value;
  return !!entry && Object.entries(entry.fields).some(([key, value]) => JSON.stringify(props.state[key]) !== JSON.stringify(value));
});
function local(): Storage {
  try { return window.localStorage; } catch { throw new Error("浏览器本地存储不可用。"); }
}
function reload() {
  try { entries.value = readCalibrationArchives(local(), props.catalog); error.value = ""; }
  catch (e) { entries.value = []; error.value = e instanceof Error ? e.message : "无法读取系数档案。"; }
}
function changed(event: StorageEvent) { if (event.key === ARCHIVE_STORAGE_KEY || event.key === null) reload(); }
onMounted(() => { reload(); window.addEventListener("storage", changed); });
onUnmounted(() => { mounted = false; ++serial; window.removeEventListener("storage", changed); });
watch(() => props.tool.id, () => { selected.value = ""; message.value = ""; reload(); });
async function openDialog(next: typeof mode.value) {
  previousFocus = document.activeElement as HTMLElement | null;
  mode.value = next;
  name.value = next === "save" ? "" : selected.value;
  dialogError.value = "";
  message.value = "";
  await nextTick();
  if (!dialog.value?.open) dialog.value?.showModal();
  if (next === "save") nameInput.value?.focus();
}
function closeDialog() {
  ++serial;
  busy.value = false;
  dialogError.value = "";
  dialog.value?.close();
  previousFocus?.focus();
}
function choose(value: string) {
  selected.value = value;
  const entry = selectedArchive.value;
  if (entry) { emit("apply", entry); message.value = `已调用档案：${entry.name}`; }
}
function nameError(value: string): string {
  if (!value.trim()) return "请输入档案名称。";
  return [...value.trim()].length > 100 ? "档案名称不能超过 100 个字。" : "";
}
async function submit() {
  const trimmed = name.value.trim();
  dialogError.value = nameError(trimmed);
  if (dialogError.value) return;
  try {
    const latest = readCalibrationArchives(local(), props.catalog);
    if (latest.some(entry => entry.tool === props.tool.id && entry.name === trimmed)) {
      name.value = trimmed; mode.value = "overwrite"; return;
    }
    await save(trimmed, false);
  } catch (e) { dialogError.value = e instanceof Error ? e.message : "读取档案失败。"; }
}
async function save(target: string, replace: boolean) {
  const version = ++serial;
  busy.value = true;
  dialogError.value = "";
  try {
    const fields = calibrationFields(props.tool, props.state, props.catalog);
    const probe: State = { ...fields, unit: "0", direction: "0", cold: "0", correction: "0" };
    if (props.tool.kind === "standard_couple") {
      const zone = props.catalog.zones[Number(fields.type)]?.[Number(fields.zone)];
      probe.value = Number(fields.source) === 1 && zone ? (zone.min + zone.max) / 2 : Number(fields.type) === 1 ? 1200 : 1000;
    } else probe.value = Number(fields.highRange) === 6 ? -20 : 20;
    await props.engine.request("calculate", { tool: props.tool.id, input: probe });
    if (!mounted || version !== serial) return;
    const latest = readCalibrationArchives(local(), props.catalog);
    const index = latest.findIndex(entry => entry.tool === props.tool.id && entry.name === target);
    if (index >= 0 && !replace) { name.value = target; mode.value = "overwrite"; return; }
    if (index < 0 && replace) throw new Error("所选档案不存在，请重新选择。");
    const archive = { tool: props.tool.id, name: target, fields };
    if (index < 0) latest.push(archive); else latest[index] = archive;
    writeCalibrationArchives(local(), latest);
    entries.value = latest; selected.value = target; error.value = "";
    closeDialog(); message.value = `已保存档案：${target}`;
  } catch (e) {
    if (mounted && version === serial) dialogError.value = e instanceof Error ? e.message : "保存失败。";
  } finally { if (mounted && version === serial) busy.value = false; }
}
function remove() {
  try {
    const latest = readCalibrationArchives(local(), props.catalog);
    const next = latest.filter(entry => entry.tool !== props.tool.id || entry.name !== name.value);
    if (next.length === latest.length) throw new Error("所选档案不存在，请重新选择。");
    writeCalibrationArchives(local(), next);
    entries.value = next; selected.value = ""; error.value = "";
    const deleted = name.value; closeDialog(); message.value = `已删除档案：${deleted}`;
  } catch (e) { dialogError.value = e instanceof Error ? e.message : "删除失败。"; }
}
function cancel() {
  if (mode.value === "overwrite" && !busy.value) { mode.value = "save"; dialogError.value = ""; nextTick(() => nameInput.value?.focus()); }
  else closeDialog();
}
</script>

<template>
  <section class="calibration-archives" aria-label="系数档案">
    <div class="archive-row">
      <label class="archive-label" :for="`archive-${tool.id}`">系数档案</label>
      <select :id="`archive-${tool.id}`" :value="selected" @change="choose(($event.target as HTMLSelectElement).value)">
        <option value="">选择已保存的档案</option>
        <option v-for="entry in entries.filter(item => item.tool === tool.id)" :key="entry.name" :value="entry.name">{{ entry.name }}</option>
      </select>
      <span v-if="dirty" class="archive-modified">已修改</span>
      <button class="button primary" type="button" @click="openDialog('save')">保存当前…</button>
      <template v-if="selectedArchive">
        <button class="button secondary" type="button" @click="openDialog('update')">更新选中…</button>
        <button class="button secondary" type="button" @click="openDialog('delete')">删除…</button>
      </template>
    </div>
    <p class="archive-hint">保存当前模型、温区及对应证书参数，不保存待换算数值。调用档案会替换当前证书参数；其他温区需另存。</p>
    <p v-if="error" class="archive-error" role="alert">{{ error }}</p>
    <p v-if="message && !dirty" class="archive-message" role="status">{{ message }}</p>
    <dialog ref="dialog" class="archive-dialog" aria-labelledby="archive-dialog-title" @cancel.prevent="closeDialog">
      <template v-if="mode === 'save'">
        <h2 id="archive-dialog-title">保存系数档案</h2>
        <label for="archive-name">档案名称</label>
        <input id="archive-name" ref="nameInput" v-model="name" maxlength="200" placeholder="例如：仪器编号或证书日期" autocomplete="off" @keydown.enter.prevent="submit" />
      </template>
      <template v-else>
        <h2 id="archive-dialog-title">{{ mode === 'delete' ? '删除系数档案' : mode === 'update' ? '更新系数档案' : '覆盖系数档案' }}</h2>
        <p v-if="mode === 'delete'">确定删除档案“{{ name }}”吗？</p>
        <p v-else-if="mode === 'update'">确定用当前证书参数更新档案“{{ name }}”吗？</p>
        <p v-else>档案“{{ name }}”已存在，确定用当前证书参数覆盖吗？</p>
      </template>
      <p v-if="dialogError" class="archive-error" role="alert">{{ dialogError }}</p>
      <div class="archive-actions">
        <button class="button secondary" type="button" @click="cancel">取消</button>
        <button v-if="mode === 'save'" class="button primary" type="button" :disabled="busy" @click="submit">保存</button>
        <button v-else-if="mode === 'delete'" class="button primary" type="button" @click="remove">确认删除</button>
        <button v-else class="button primary" type="button" :disabled="busy" @click="save(name, true)">{{ mode === 'update' ? '确认更新' : '确认覆盖' }}</button>
      </div>
    </dialog>
  </section>
</template>

<style scoped>
.calibration-archives { max-width: 960px; margin-bottom: 18px; }
.archive-row { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; }
.archive-label { color: #718099; font-size: 12px; }
.archive-row select { width: min(260px, 100%); min-height: 36px; height: 36px; }
.archive-row .button { min-width: 96px; min-height: 36px; }
.archive-hint, .archive-modified { color: #6f7e92; font-size: 12px; line-height: 1.6; }
.archive-error { color: #b74846; font-size: 13px; }
.archive-message { color: #2769dc; font-size: 13px; }
.archive-dialog { width: min(488px, calc(100vw - 32px)); max-height: calc(100dvh - 48px); overflow: auto; margin: auto; padding: 24px; border: 1px solid #dce3ed; border-radius: 10px; background: white; color: #233043; box-shadow: 0 12px 35px #142d4b2e; }
.archive-dialog::backdrop { background: #0006; }
.archive-dialog h2 { font-size: 19px; margin: 0 0 20px; }
.archive-dialog p { font-size: 14px; line-height: 1.8; overflow-wrap: anywhere; }
.archive-dialog label { display: block; color: #6f7e92; font-size: 13px; margin-bottom: 8px; }
.archive-actions { display: flex; justify-content: flex-end; gap: 12px; margin-top: 24px; }
.archive-actions .button { min-width: 112px; min-height: 40px; font-size: 14px; }
@media (max-width: 600px) { .archive-row select { flex: 1 1 180px; } .archive-dialog { padding: 20px; } }
</style>
