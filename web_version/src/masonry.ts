/** 已有卡片保留列位置；新增卡片放到实际高度较低的列。 */
export function placeCards(heights: number[], previous: number[], columns: 1 | 2, gap = 12) {
  const assignments = previous.slice(0, heights.length);
  const bottoms = [0, 0];
  const positions = heights.map((height, index) => {
    let column = 0;
    if (columns === 2) {
      if (assignments[index] === undefined) assignments[index] = bottoms[1] < bottoms[0] ? 1 : 0;
      column = assignments[index];
    }
    const top = bottoms[column];
    bottoms[column] += height + gap;
    return { column, top };
  });
  return { assignments, positions, height: Math.max(0, ...bottoms) };
}
