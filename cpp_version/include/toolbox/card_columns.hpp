#pragma once
#include <cstddef>
#include <vector>

namespace toolbox {
// 旧卡片保留原列，新增卡片按当前高度选择较短的一列。
inline int cardColumn(std::vector<int> &assignments, std::size_t index, float left, float right) {
    if (index < assignments.size()) return assignments[index];
    int column = right < left ? 1 : 0;
    assignments.push_back(column);
    return column;
}
} // namespace toolbox
