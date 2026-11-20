<script setup lang="ts">
// 结果表支持筛选和分页；单位换算采用现代双列数值卡片，支持一键复制与切换源单位。
import { computed, ref, watch } from "vue";
import { display } from "../format";

const p = withDefaults(
  defineProps<{
    headings: string[];
    rows: unknown[][];
    unitTable?: boolean;
    sourceIndex?: number;
    total?: number;
    title?: string;
    compact?: boolean;
  }>(),
  { unitTable: false, sourceIndex: -1, total: 0 },
);

const emit = defineEmits<{
  copy: [text: string, label: string];
  swapUnit: [index: number];
}>();

const page = ref(1);
const query = ref("");
const size = computed(() => (p.unitTable ? 24 : 12));

const indexedRows = computed(() =>
  p.rows.map((row, index) => ({ row, index })),
);

const filtered = computed(() =>
  query.value.trim()
    ? indexedRows.value.filter(({ row }) =>
        row.some((v) =>
          String(v).toLowerCase().includes(query.value.trim().toLowerCase()),
        ),
      )
    : indexedRows.value,
);

const pages = computed(() =>
  Math.max(1, Math.ceil(filtered.value.length / size.value)),
);
const current = computed(() => Math.min(page.value, pages.value));
const visible = computed(() =>
  filtered.value.slice(
    (current.value - 1) * size.value,
    current.value * size.value,
  ),
);
watch(query, () => (page.value = 1));
</script>

<template>
  <section class="panel result-table-panel">
    <div class="section-heading">
      <div>
        <h2>{{ title || (unitTable ? "全部换算结果" : "数据结果") }}</h2>
        <p v-if="total > rows.length" class="muted">
          预览 {{ rows.length }} / {{ total }} 行
        </p>
        <p v-else-if="unitTable" class="muted">
          {{ filtered.length }} 项单位 · 点击卡片可复制数值
        </p>
        <p v-else-if="!compact" class="muted">
          {{ filtered.length }} 行结果{{ headings.length >= 6 ? " · 表格宽于视口时可左右滑动查看全部列" : "" }}
        </p>
        <p v-else-if="headings.length >= 6" class="muted">
          宽表可左右滑动查看全部 {{ headings.length }} 列
        </p>
      </div>
      <input
        v-if="unitTable"
        v-model="query"
        class="table-search"
        placeholder="筛选单位名称或符号…"
        aria-label="筛选换算单位"
      />
    </div>

    <!-- 单位换算：现代双列高对比度数值卡片网格 -->
    <template v-if="unitTable">
      <div v-if="visible.length" class="unit-cards-grid">
        <div
          v-for="item in visible"
          :key="item.index"
          :class="['unit-result-card', { 'is-source': item.index === sourceIndex }]"
          role="button"
          tabindex="0"
          :title="`点击复制：${display(item.row[0])} ${item.row[2] ?? ''}`"
          @click="emit('copy', display(item.row[0]), String(item.row[1] ?? '数值'))"
          @keydown.enter.prevent="emit('copy', display(item.row[0]), String(item.row[1] ?? '数值'))"
        >
          <div class="unit-card-head">
            <div class="unit-card-name">
              <strong>{{ item.row[1] }}</strong>
              <span v-if="item.row[2]" class="unit-card-sym">{{ item.row[2] }}</span>
            </div>
            <button
              v-if="item.index !== sourceIndex"
              type="button"
              class="unit-swap-btn"
              title="设此单位为输入源单位"
              :aria-label="`切换输入源单位为 ${item.row[1]}`"
              @click.stop="emit('swapUnit', item.index)"
            >
              ⇄
            </button>
          </div>
          <output class="unit-card-value">{{ display(item.row[0]) }}</output>
        </div>
      </div>
      <p v-else class="empty-row">没有匹配的结果</p>
    </template>

    <!-- 通用结果表格：严格横向滚动防裁切 -->
    <div
      v-else
      class="table-wrap"
      tabindex="0"
      aria-label="结果表格，可左右滚动"
    >
      <table>
        <thead>
          <tr>
            <th v-for="(heading, j) in headings" :key="j">{{ heading }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in visible" :key="item.index">
            <td
              v-for="(cell, k) in item.row"
              :key="k"
              :class="{ 'number-cell': typeof cell === 'number' }"
            >
              {{ display(cell) }}
            </td>
          </tr>
          <tr v-if="!visible.length">
            <td :colspan="headings.length" class="empty-row">没有匹配的结果</td>
          </tr>
        </tbody>
      </table>
    </div>

    <nav v-if="pages > 1" class="pagination" aria-label="结果分页">
      <button
        class="button subtle"
        :disabled="current <= 1"
        @click="page = current - 1"
      >
        上一页
      </button>
      <span>{{ current }} / {{ pages }}</span>
      <button
        class="button subtle"
        :disabled="current >= pages"
        @click="page = current + 1"
      >
        下一页
      </button>
    </nav>
  </section>
</template>
