<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { display } from "../format";
const p = withDefaults(
  defineProps<{
    headings: string[];
    rows: unknown[][];
    unitTable?: boolean;
    total?: number;
    title?: string;
    compact?: boolean;
  }>(),
  { unitTable: false, total: 0 },
);
const page = ref(1);
const query = ref("");
const size = computed(() => (p.unitTable ? 20 : 10));
const filtered = computed(() =>
  query.value.trim()
    ? p.rows.filter((row) =>
        row.some((v) =>
          String(v).toLowerCase().includes(query.value.trim().toLowerCase()),
        ),
      )
    : p.rows,
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
        <p v-else-if="!compact" class="muted">
          {{ filtered.length }} {{ unitTable ? "项单位" : "行结果" }}
        </p>
      </div>
      <input
        v-if="unitTable"
        v-model="query"
        class="table-search"
        placeholder="筛选单位名称或符号"
        aria-label="筛选换算单位"
      />
    </div>
    <div v-if="unitTable" class="mobile-unit-results">
      <div v-for="(row, j) in visible" :key="j" class="mobile-unit-row">
        <div>
          <strong>{{ row[1] }}</strong
          ><span>{{ row[2] }}</span>
        </div>
        <output>{{ display(row[0]) }}</output>
      </div>
      <p v-if="!visible.length" class="empty-row">没有匹配的结果</p>
    </div>
    <div
      :class="['table-wrap', { 'desktop-unit-table': unitTable }]"
      tabindex="0"
      :aria-label="unitTable ? '单位换算结果表' : '结果表格，可左右滚动'"
    >
      <table>
        <thead>
          <tr>
            <th v-for="(heading, j) in headings" :key="j">{{ heading }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, j) in visible" :key="j">
            <td
              v-for="(cell, k) in row"
              :key="k"
              :class="{
                'number-cell': typeof cell === 'number',
                'unit-symbol': unitTable && k === 2,
              }"
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
        上一页</button
      ><span>{{ current }} / {{ pages }}</span
      ><button
        class="button subtle"
        :disabled="current >= pages"
        @click="page = current + 1"
      >
        下一页
      </button>
    </nav>
  </section>
</template>
