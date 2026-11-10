import { test } from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";

const { outputFiles } = await build({ entryPoints: ["src/masonry.ts"], bundle: true, write: false, format: "esm" });
const { placeCards } = await import(`data:text/javascript;base64,${Buffer.from(outputFiles[0].contents).toString("base64")}`);

test("new cards use the shorter column without moving existing cards", () => {
  const initial = placeCards([700, 200], [], 2);
  assert.deepEqual(initial.assignments, [0, 1]);
  const added = placeCards([700, 200, 150], initial.assignments, 2);
  assert.deepEqual(added.assignments, [0, 1, 1]);
  const changed = placeCards([100, 200, 150], added.assignments, 2);
  assert.deepEqual(changed.assignments, [0, 1, 1]);
  const next = placeCards([100, 200, 150, 120], changed.assignments, 2);
  assert.deepEqual(next.assignments, [0, 1, 1, 0]);
  assert.ok(next.positions[3].top >= 100);
});

test("single column keeps order and restores prior placement on expansion", () => {
  const wide = placeCards([400, 100, 120], [], 2);
  const narrow = placeCards([400, 100, 120, 80], wide.assignments, 1);
  assert.deepEqual(narrow.positions.map(p => p.column), [0, 0, 0, 0]);
  assert.deepEqual(narrow.assignments, wide.assignments);
  const expanded = placeCards([400, 100, 120, 80], narrow.assignments, 2);
  assert.deepEqual(expanded.assignments.slice(0, 3), wide.assignments);
  assert.equal(expanded.assignments[3], 1);
  const removed = [...expanded.assignments]; removed.splice(1, 1);
  assert.deepEqual(placeCards([400, 120, 80], removed, 2).assignments, removed);
});
