// 首页与分类导航共用工具目录，计数、筛选和页面入口始终保持一致。
#include "toolbox/gui.hpp"
namespace toolbox {
void App::navigation() {
    auto &ui = scene;
    ui.clip = {0, 68, width, height - 68};
    if (width < 880) {
        float cell = width / groupCount;
        for (int i = 0; i < groupCount; ++i) {
            ui.button(
                {i * cell + 4, 78, cell - 8, 36}, shortGroupName(Group(i)),
                [this, i] {
                    filter = filter == i ? -1 : i;
                    open(-1);
                },
                filter == i);
        }
        sideHeight_ = 0;
        return;
    }
    float sw = collapsed ? 92.f : 245.f;
    ui.fill({0, 68, sw, height - 68}, White);
    ui.line(sw, 68, sw, height, Border);
    if (!collapsed) {
        ui.text({15, 79, 170, 32}, L"工具导航", 14, Ink, true);
    }
    ui.button({collapsed ? 18.f : 198.f, 80, collapsed ? 56.f : 32.f, 30}, collapsed ? L"展开 ›" : L"«",
              [this] {
                  collapsed = !collapsed;
                  refresh();
              });
    float y = 124 - sideScroll;
    ui.clip = {8, 122, sw - 16, height - 130};
    if (!collapsed) {
        auto &search = state.value("navigation.search", L"");
        ui.input("navigation.search", {15, y, sw - 30, 34}, search);
        y += 50;
    }
    auto search = state.value("navigation.search", L"");
    for (int g = 0; g < groupCount; ++g) {
        int count = 0;
        for (auto &tool : tools()) {
            if (int(tool.group) == g) {
                ++count;
            }
        }
        if (collapsed) {
            ui.button(
                {12, y, sw - 24, 44}, shortGroupName(Group(g)),
                [this, g] {
                    filter = filter == g ? -1 : g;
                    open(-1);
                },
                filter == g);
            y += 50;
        } else {
            bool active = (selected >= 0 && int(tools()[selected].group) == g) || filter == g;
            Rect r{15, y, sw - 30, 42};
            ui.fill(r, active ? Soft : Background, 6);
            ui.text({27, y, sw - 90, 42}, groupName(Group(g)), 15, active ? Blue : Ink, true);
            ui.text({sw - 64, y, 48, 42}, std::to_wstring(count) + (expanded[g] ? L"  ⌄" : L"  ›"), 12,
                    active ? Blue : Muted, false, true);
            ui.hits.push_back({r, ui.clip, [this, g] {
                                   expanded[g] = !expanded[g];
                                   refresh();
                               }});
            y += 48;
            if (expanded[g] || !search.empty()) {
                float start = y;
                for (std::size_t i = 0; i < tools().size(); ++i) {
                    auto &tool = tools()[i];
                    if (int(tool.group) != g) {
                        continue;
                    }
                    if (!search.empty() && tool.name.find(search) == std::wstring::npos &&
                        tool.description.find(search) == std::wstring::npos) {
                        continue;
                    }
                    Rect row{15, y, sw - 30, 36};
                    if (selected == int(i)) {
                        ui.fill(row, Soft, 4);
                        ui.fill({26, y + 7, 3, 22}, Blue, 1);
                    }
                    ui.text({43, y, sw - 61, 36}, tool.name, 13, selected == int(i) ? Blue : Ink,
                            selected == int(i));
                    ui.hits.push_back({row, ui.clip, [this, i] { open(int(i)); }});
                    y += 40;
                }
                if (y > start) {
                    ui.line(27, start, 27, y - 4, Border);
                }
                y += 6;
            }
            y += 8;
        }
    }
    sideHeight_ = y + sideScroll - 124;
    ui.clip = {0, 0, width, height};
}
void App::homepage() {
    float left = width >= 880 ? (collapsed ? 92.f : 245.f) : 0, top = width >= 880 ? 82.f : 126.f;
    float x = left + 24, w = width - left - 48, y = top + 12 - scroll;
    scene.text({x, y, w, 48}, L"计量与工程计算，一处完成", width < 720 ? 23.f : 28.f, Ink, true, true);
    y += 48;
    scene.text({x, y, w, 40}, L"按工具分类快速查找。计算在本机完成，结果随参数即时更新。", 13, Muted, false,
               true);
    y += 58;
    std::vector<std::wstring> categories{L"全部工具"};
    for (int i = 0; i < groupCount; ++i) {
        int n = 0;
        for (auto &t : tools()) {
            if (int(t.group) == i) {
                ++n;
            }
        }
        categories.push_back(groupName(Group(i)) + L" " + std::to_wstring(n));
    }
    float bx = x;
    for (int i = 0; i < int(categories.size()); ++i) {
        float bw = std::max(76.f, float(categories[i].size()) * 12 + 14);
        if (bx + bw > x + w) {
            y += 38;
            bx = x;
        }
        scene.button(
            {bx, y, bw, 30}, categories[i],
            [this, i] {
                filter = i - 1;
                scroll = 0;
                refresh();
            },
            filter == i - 1);
        bx += bw + 6;
    }
    y += 44;
    auto &search = state.value("home.search", L"");
    scene.input("home.search", {x, y, std::min(w, 420.f), 34}, search);
    if (search.empty()) {
        scene.text({x + 9, y, std::min(w, 420.f) - 18, 34}, L"搜索名称或功能", 12, Muted);
    }
    y += 52;
    int cols = std::clamp(int((w + 14) / 284), 1, 4);
    float cell = (w - 14 * (cols - 1)) / cols;
    int index = 0;
    for (std::size_t i = 0; i < tools().size(); ++i) {
        auto &tool = tools()[i];
        if (filter >= 0 && int(tool.group) != filter) {
            continue;
        }
        if (!search.empty() && tool.name.find(search) == std::wstring::npos &&
            tool.description.find(search) == std::wstring::npos) {
            continue;
        }
        float cx = x + (index % cols) * (cell + 14), cy = y + (index / cols) * 176;
        scene.fill({cx, cy, cell, 162}, White, 8);
        scene.border({cx, cy, cell, 162});
        float offset = 0;
        if (!tool.icon.empty()) {
            scene.fill({cx + 14, cy + 14, 40, 40}, Soft, 5);
            scene.text({cx + 14, cy + 14, 40, 40}, tool.icon, 16, Blue, false, true);
            offset = 52;
        }
        scene.text({cx + 14 + offset, cy + 12, cell - 28 - offset, 29}, tool.name, 14, Ink, true);
        scene.text({cx + 14 + offset, cy + 40, cell - 28 - offset, 22}, groupName(tool.group), 11, Muted);
        scene.text({cx + 14, cy + 66, cell - 28, 48}, tool.description, 12, Muted);
        scene.button({cx + cell - 130, cy + 116, 116, 34}, L"打开工具 →", [this, i] { open(int(i)); });
        ++index;
    }
    if (index == 0) {
        scene.text({x, y, w, 60}, L"没有找到匹配的工具", 15, Muted, false, true);
    }
    contentHeight_ = y + float((index + cols - 1) / cols) * 176 + scroll - top + 24;
}
} // namespace toolbox
