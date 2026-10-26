<script setup lang="ts">
/** 图表仅作坐标映射；曲线采样、残差和拟合值由 Rust 提供。 */
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { decimal, display } from "../format";
const p = withDefaults(
  defineProps<{
    curve: [number, number][];
    points?: [number, number][];
    xLabel?: string;
    yLabel?: string;
    scatter?: boolean;
  }>(),
  { points: () => [], xLabel: "x", yLabel: "y", scatter: false },
);
const host = ref<HTMLElement>();
const width = ref(800);
const height = 300;
const hover = ref<[number, number] | null>(null);
const pinned = ref(false);
let touchStart: { x: number; y: number } | null = null;
watch(
  () => [p.curve, p.points],
  () => {
    hover.value = null;
    pinned.value = false;
  },
);
let observer: ResizeObserver | undefined;
onMounted(() => {
  observer = new ResizeObserver(
    ([e]) => (width.value = Math.max(220, e.contentRect.width)),
  );
  if (host.value) observer.observe(host.value);
});
onUnmounted(() => observer?.disconnect());
function axis(values: number[], target = 4) {
  if (!values.length)
    return {
      min: 0,
      max: 1,
      ticks: [0, 0.5, 1],
      base: 0,
      factor: 1,
      exponent: 0,
    };
  const rawMin = Math.min(...values),
    rawMax = Math.max(...values),
    rawSpan = rawMax - rawMin;
  const base =
    Number.isFinite(rawSpan) &&
    rawSpan > 0 &&
    Math.abs(rawMin) > 1e4 &&
    rawSpan < Math.abs(rawMin) * 1e-4
      ? rawMin
      : 0;
  const magnitude =
    rawSpan > 0 && Number.isFinite(rawSpan)
      ? rawSpan
      : Math.max(Math.abs(rawMin), Math.abs(rawMax), 1);
  const exponent =
    magnitude < 0.001 || magnitude >= 1e6
      ? Math.floor(Math.log10(magnitude))
      : 0;
  const factor = 10 ** exponent || Number.MIN_VALUE;
  const min = (rawMin - base) / factor,
    max = (rawMax - base) / factor;
  const span = max - min;
  if (!Number.isFinite(span))
    return {
      min: -1,
      max: 1,
      ticks: [-1, 0, 1],
      base: 0,
      factor: Math.max(Math.abs(rawMin), Math.abs(rawMax)),
      exponent,
    };
  const pad = span === 0 ? Math.max(Math.abs(min) * 0.05, 1) : span * 0.07;
  const low = min - pad,
    high = max + pad;
  const raw = (high - low) / target;
  const power = 10 ** Math.floor(Math.log10(raw));
  const fraction = raw / power;
  const step = ([1, 2, 2.5, 5, 10].find((x) => x >= fraction) ?? 10) * power;
  if (!Number.isFinite(step) || step <= 0)
    return { min: low, max: high, ticks: [min, max], base, factor, exponent };
  const ticks: number[] = [];
  let first = Math.ceil(low / step) * step;
  for (let j = 0; j < 10; j++) {
    const v = first + j * step;
    if (v > high + step * 1e-10) break;
    ticks.push(Number(v.toPrecision(12)));
  }
  return { min: low, max: high, ticks, base, factor, exponent };
}
const data = computed(() =>
  [...p.curve, ...p.points].filter((q) => q.every(Number.isFinite)),
);
const xa = computed(() =>
  axis(
    data.value.map((q) => q[0]),
    width.value < 500 ? 3 : 5,
  ),
);
const ya = computed(() => axis(data.value.map((q) => q[1])));
const left = computed(() =>
  Math.min(
    width.value * 0.4,
    Math.max(58, ...ya.value.ticks.map((v) => display(v).length * 6.8 + 20)),
  ),
);
const plot = computed(() => ({
  x: left.value,
  y: 22,
  w: Math.max(80, width.value - left.value - 22),
  h: height - 82,
}));
function px(x: number) {
  return (
    plot.value.x +
    (((x - xa.value.base) / xa.value.factor - xa.value.min) /
      (xa.value.max - xa.value.min)) *
      plot.value.w
  );
}
function py(y: number) {
  return (
    plot.value.y +
    (1 -
      ((y - ya.value.base) / ya.value.factor - ya.value.min) /
        (ya.value.max - ya.value.min)) *
      plot.value.h
  );
}
function tx(v: number) {
  return (
    plot.value.x +
    ((v - xa.value.min) / (xa.value.max - xa.value.min)) * plot.value.w
  );
}
function ty(v: number) {
  return (
    plot.value.y +
    (1 - (v - ya.value.min) / (ya.value.max - ya.value.min)) * plot.value.h
  );
}
function caption(label: string, a: ReturnType<typeof axis>) {
  return (
    label +
    (a.exponent ? ` · 刻度 ×10^${a.exponent}` : "") +
    (a.base ? ` · 基准 ${decimal(a.base)}` : "")
  );
}
const path = computed(() =>
  p.curve.map((q, j) => `${j ? "L" : "M"}${px(q[0])},${py(q[1])}`).join(" "),
);
const clipId = "chart-" + Math.random().toString(36).slice(2);
function move(event: PointerEvent) {
  if (event.pointerType === "touch" || pinned.value) return;
  inspect(event);
}
function inspect(event: PointerEvent) {
  const rect = (event.currentTarget as SVGElement).getBoundingClientRect();
  const x = ((event.clientX - rect.left) * width.value) / rect.width;
  const y = ((event.clientY - rect.top) * height) / rect.height;
  if (x < plot.value.x || x > plot.value.x + plot.value.w) {
    hover.value = null;
    return;
  }
  const candidates = p.scatter ? p.points : p.curve;
  const distance = (q: [number, number]) =>
    p.scatter ? Math.hypot(px(q[0]) - x, py(q[1]) - y) : Math.abs(px(q[0]) - x);
  hover.value = candidates.reduce<[number, number] | null>(
    (best, q) => (!best || distance(q) < distance(best) ? q : best),
    null,
  );
}
function press(event: PointerEvent) {
  touchStart =
    event.pointerType === "touch"
      ? { x: event.clientX, y: event.clientY }
      : null;
}
function release(event: PointerEvent) {
  // 只有轻点才固定读数；纵向拖动交由浏览器滚页，不抢占触摸手势。
  if (event.pointerType !== "touch" || !touchStart) return;
  const start = touchStart;
  touchStart = null;
  if (Math.hypot(event.clientX - start.x, event.clientY - start.y) > 10) return;
  pinned.value = true;
  inspect(event);
}
</script>
<template>
  <div ref="host" class="chart-host">
    <svg
      v-if="data.length"
      :viewBox="`0 0 ${width} ${height}`"
      :style="{ height: height + 'px' }"
      role="img"
      :aria-label="`${xLabel} 与 ${yLabel} 曲线`"
      @pointermove="move"
      @pointerdown="press"
      @pointerup="release"
      @pointercancel="touchStart = null"
      @pointerleave="!pinned && (hover = null)"
    >
      <defs>
        <clipPath :id="clipId">
          <rect :x="plot.x" :y="plot.y" :width="plot.w" :height="plot.h" />
        </clipPath>
      </defs>
      <g v-for="v in ya.ticks" :key="'y' + v">
        <line
          :x1="plot.x"
          :y1="ty(v)"
          :x2="plot.x + plot.w"
          :y2="ty(v)"
          class="grid-line"
        />
        <text
          :x="plot.x - 10"
          :y="ty(v) + 4"
          text-anchor="end"
          class="tick-label"
        >
          {{ display(v) }}
        </text>
      </g>
      <g v-for="v in xa.ticks" :key="'x' + v">
        <line
          :x1="tx(v)"
          :y1="plot.y"
          :x2="tx(v)"
          :y2="plot.y + plot.h"
          class="grid-line vertical"
        />
        <text
          :x="tx(v)"
          :y="plot.y + plot.h + 22"
          text-anchor="middle"
          class="tick-label"
        >
          {{ display(v) }}
        </text>
      </g>
      <text :x="plot.x" y="14" class="axis-label">
        {{ caption(yLabel, ya) }}
      </text>
      <text
        :x="plot.x + plot.w / 2"
        :y="height - 10"
        text-anchor="middle"
        class="axis-label"
      >
        {{ caption(xLabel, xa) }}
      </text>
      <g :clip-path="`url(#${clipId})`">
        <line
          v-if="scatter && ya.min <= 0 && ya.max >= 0"
          :x1="plot.x"
          :x2="plot.x + plot.w"
          :y1="py(0)"
          :y2="py(0)"
          class="zero-line"
        />
        <path v-if="!scatter" :d="path" class="curve-line" />
        <circle
          v-for="(q, j) in points"
          :key="j"
          :cx="px(q[0])"
          :cy="py(q[1])"
          r="3.5"
          :class="scatter ? 'residual-point' : 'data-point'"
        />
        <template v-if="hover">
          <line
            :x1="px(hover[0])"
            :x2="px(hover[0])"
            :y1="plot.y"
            :y2="plot.y + plot.h"
            class="crosshair"
          />
          <circle
            :cx="px(hover[0])"
            :cy="py(hover[1])"
            r="4"
            class="hover-point"
          />
        </template>
      </g>
    </svg>
    <div v-if="hover" class="chart-readout" aria-live="polite">
      <span>x = {{ display(hover[0]) }}</span
      ><span>y = {{ display(hover[1]) }}</span>
      <button
        v-if="pinned"
        type="button"
        class="chart-clear"
        aria-label="关闭图表读数"
        @click="
          pinned = false;
          hover = null;
        "
      >
        关闭
      </button>
    </div>
  </div>
</template>
