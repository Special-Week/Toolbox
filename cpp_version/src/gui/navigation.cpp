// 首页、多标签、侧栏分类导航与快速检索；支持收藏置顶、分类专属点缀色与整卡点击。
#include "toolbox/gui.hpp"
#include <algorithm>
namespace toolbox {
namespace {
bool containsInsensitive(const std::wstring &text, const std::wstring &query) {
    if (query.empty())
        return true;
    const auto fold = [](wchar_t value) {
        return value >= L'A' && value <= L'Z' ? value + (L'a' - L'A') : value;
    };
    return std::search(text.begin(), text.end(), query.begin(), query.end(),
                       [&](wchar_t a, wchar_t b) { return fold(a) == fold(b); }) != text.end();
}
bool matchesSearch(const Tool &tool, const std::wstring &query) {
    return containsInsensitive(tool.name, query) || containsInsensitive(groupName(tool.group), query) ||
           containsInsensitive(tool.description, query) || containsInsensitive(wide(tool.id), query);
}
std::wstring trimQuery(const std::wstring &raw) {
    const auto first = raw.find_first_not_of(L" \t\r\n");
    if (first == std::wstring::npos)
        return L"";
    return raw.substr(first, raw.find_last_not_of(L" \t\r\n") - first + 1);
}
} // namespace

void App::navigation() {
    auto &ui = scene;
    const float headerH = 52.f;
    ui.clip = {0, headerH, width, height - headerH};
    if (width < 880) {
        float cell = width / groupCount;
        for (int i = 0; i < groupCount; ++i) {
            const auto accent = groupAccent(Group(i));
            const bool active = filter == i;
            Rect r{i * cell + 4, headerH + 8, cell - 8, 34};
            ui.fill(r, active ? accent.bg : White, 8);
            ui.border(r, active ? accent.fg : Border, 8);
            ui.text(r, shortGroupName(Group(i)), 12, active ? accent.fg : Ink, active, true);
            ui.hits.push_back({r, ui.clip, [this, i] {
                                   filter = filter == i ? -1 : i;
                                   open(-1);
                               }});
        }
        sideHeight_ = 0;
        return;
    }

    float sw = collapsed ? 76.f : 248.f;
    ui.fill({0, headerH, sw, height - headerH}, White);
    ui.line(sw, headerH, sw, height, Border);

    if (!collapsed) {
        ui.text({16, headerH + 10, 160, 28}, L"工具导航", 13, Muted, true);
    }
    Rect toggleRect{collapsed ? 16.f : sw - 42.f, headerH + 10, collapsed ? 44.f : 28.f, 28.f};
    ui.fill(toggleRect, SurfaceSubtle, 7);
    ui.border(toggleRect, Border, 7);
    ui.text(toggleRect, collapsed ? L"›" : L"«", 14, Muted, true, true);
    ui.hits.push_back({toggleRect, ui.clip, [this] {
                           collapsed = !collapsed;
                           refresh();
                       }});

    float listTop = headerH + 48.f;
    std::wstring filterQuery;
    if (!collapsed) {
        auto &sideSearch = state.value("__sidebar.filter", L"");
        ui.input("__sidebar.filter", {12, listTop, sw - 24, 32}, sideSearch);
        filterQuery = trimQuery(sideSearch);
        listTop += 42.f;
    }

    float y = listTop - sideScroll;
    ui.clip = {6, listTop, sw - 12, std::max(0.f, height - listTop - 6)};

    // 置顶“我的收藏”分组
    if (!collapsed && !favorites.empty() && filterQuery.empty()) {
        ui.text({16, y, sw - 32, 22}, L"★ 我的收藏", 11.5f, StarGold, true);
        y += 26.f;
        for (const auto &favId : favorites) {
            for (std::size_t i = 0; i < tools().size(); ++i) {
                if (tools()[i].id != favId)
                    continue;
                const auto &tool = tools()[i];
                const auto accent = groupAccent(tool.group);
                Rect row{12, y, sw - 24, 32};
                const bool sel = selected == int(i);
                if (sel) {
                    ui.fill(row, accent.bg, 7);
                    ui.fill({12, y + 6, 3, 20}, accent.fg, 2);
                }
                ui.text({22, y, sw - 64, 32}, tool.name, 12.5f, sel ? accent.fg : Ink, sel);
                Rect starRect{sw - 38, y + 3, 24, 26};
                ui.text(starRect, L"★", 12, StarGold, false, true);
                ui.hits.push_back({{12, y, sw - 52, 32}, ui.clip, [this, i] { open(int(i)); }});
                ui.hits.push_back({starRect, ui.clip, [this, id = tool.id, name = tool.name] {
                                       toggleFavorite(id, name);
                                   }});
                y += 35.f;
                break;
            }
        }
        ui.line(14, y + 4, sw - 14, y + 4, Border);
        y += 12.f;
    }

    for (int g = 0; g < groupCount; ++g) {
        const auto accent = groupAccent(Group(g));
        std::vector<std::size_t> matchedIndices;
        for (std::size_t i = 0; i < tools().size(); ++i) {
            if (int(tools()[i].group) == g && (filterQuery.empty() || matchesSearch(tools()[i], filterQuery))) {
                matchedIndices.push_back(i);
            }
        }
        if (!filterQuery.empty() && matchedIndices.empty())
            continue;

        if (collapsed) {
            const bool active = (selected >= 0 && int(tools()[selected].group) == g) || filter == g;
            Rect r{10, y, sw - 20, 44};
            ui.fill(r, active ? accent.bg : SurfaceSubtle, 9);
            ui.border(r, active ? accent.fg : Border, 9);
            ui.fill({10, y + 10, 3, 24}, accent.fg, 2);
            ui.text(r, shortGroupName(Group(g)), 12, active ? accent.fg : Ink, active, true);
            ui.hits.push_back({r, ui.clip, [this, g] {
                                   filter = filter == g ? -1 : g;
                                   open(-1);
                               }});
            y += 50;
        } else {
            const bool active = (selected >= 0 && int(tools()[selected].group) == g) || filter == g;
            const bool isExpanded = expanded[g] || !filterQuery.empty();
            Rect r{12, y, sw - 24, 38};
            ui.fill(r, active ? accent.bg : SurfaceSubtle, 8);
            ui.fill({18, y + 9, 6, 20}, accent.fg, 3);
            ui.text({32, y, sw - 92, 38}, groupName(Group(g)), 13.5f, active ? accent.fg : Ink, true);
            ui.text({sw - 64, y, 48, 38},
                    std::to_wstring(matchedIndices.size()) + (isExpanded ? L" ▾" : L" ▸"), 11.5f,
                    active ? accent.fg : Muted, false, true);
            ui.hits.push_back({r, ui.clip, [this, g] {
                                   expanded[g] = !expanded[g];
                                   refresh();
                               }});
            y += 42;
            if (isExpanded) {
                float start = y;
                for (std::size_t i : matchedIndices) {
                    const auto &tool = tools()[i];
                    const bool sel = selected == int(i);
                    const bool fav = isFavorite(tool.id);
                    Rect row{22, y, sw - 34, 32};
                    if (sel) {
                        ui.fill(row, accent.bg, 7);
                        ui.fill({22, y + 6, 3, 20}, accent.fg, 2);
                    }
                    ui.text({32, y, sw - 74, 32}, tool.name, 12.5f, sel ? accent.fg : Ink, sel);
                    Rect starRect{sw - 38, y + 3, 24, 26};
                    ui.text(starRect, fav ? L"★" : L"☆", 12, fav ? StarGold : Muted, false, true);
                    ui.hits.push_back({{22, y, sw - 60, 32}, ui.clip, [this, i] { open(int(i)); }});
                    ui.hits.push_back({starRect, ui.clip, [this, id = tool.id, name = tool.name] {
                                           toggleFavorite(id, name);
                                       }});
                    y += 35;
                }
                if (y > start) {
                    ui.line(18, start, 18, y - 4, Border);
                }
                y += 4;
            }
            y += 6;
        }
    }
    sideHeight_ = y + sideScroll - listTop;
    ui.clip = {0, 0, width, height};
}

void App::homepage() {
    const float headerH = 52.f;
    const float tabBarH = openTabs.empty() ? 0.f : 38.f;
    float left = width >= 880 ? (collapsed ? 76.f : 248.f) : 0;
    float top = (width >= 880 ? headerH : headerH + 48.f) + tabBarH;
    float w = std::min(width - left - 48.f, 1240.f);
    float x = left + std::max(24.f, (width - left - w) / 2.f);
    float y = top + 14.f - scroll;

    // Hero 徽章与主标题
    float badgeW = 248.f;
    scene.fill({x + (w - badgeW) / 2.f, y, badgeW, 24}, Soft, 12);
    scene.border({x + (w - badgeW) / 2.f, y, badgeW, 24}, Border, 12);
    scene.text({x + (w - badgeW) / 2.f, y, badgeW, 24}, L"MEASUREMENT & ENGINEERING", 10.5f, Blue, true, true);
    y += 32.f;
    scene.text({x, y, w, 40}, L"计量与工程计算，一处完成", width < 720 ? 22.f : 27.f, Ink, true, true);
    y += 40.f;
    scene.text({x, y, w, 28},
               std::to_wstring(groupCount) + L" 大分类 · " + std::to_wstring(tools().size()) +
                   L" 项工具 · 本机即时计算 · Ctrl+K 全局快搜",
               12.5f, Muted, false, true);
    y += 40.f;

    // 快捷访问条（收藏 + 最近使用）
    std::vector<int> quickIndices;
    auto pushUnique = [&](const std::string &id) {
        if (quickIndices.size() >= 8)
            return;
        for (std::size_t i = 0; i < tools().size(); ++i) {
            if (tools()[i].id == id) {
                if (std::find(quickIndices.begin(), quickIndices.end(), int(i)) == quickIndices.end())
                    quickIndices.push_back(int(i));
                return;
            }
        }
    };
    for (const auto &id : favorites)
        pushUnique(id);
    for (const auto &id : recent)
        pushUnique(id);

    if (!quickIndices.empty()) {
        auto barFill = scene.draws.size();
        scene.fill({x, y, w, 46}, White, 10);
        scene.border({x, y, w, 46}, Border, 10);
        scene.text({x + 14, y + 9, 64, 28}, L"快捷访问", 12, Muted, true);
        float qx = x + 82.f, qy = y + 9.f;
        for (int idx : quickIndices) {
            const auto &t = tools()[std::size_t(idx)];
            const auto accent = groupAccent(t.group);
            float cw = std::min(190.f, textWidth(t.name, 12) + 30.f);
            if (qx + cw > x + w - 14.f) {
                qx = x + 82.f;
                qy += 34.f;
            }
            Rect chip{qx, qy, cw, 28};
            scene.fill(chip, accent.bg, 14);
            scene.border(chip, Border, 14);
            scene.fill({qx + 9, qy + 11, 6, 6}, accent.fg, 3);
            scene.text({qx + 20, qy, cw - 26, 28}, t.name, 12, Ink);
            scene.hits.push_back({chip, scene.clip, [this, idx] { open(idx); }});
            qx += cw + 8.f;
        }
        float barH = (qy - y) + 37.f;
        scene.draws[barFill].rect.h = barH;
        scene.draws[barFill + 1].rect.h = barH;
        y += barH + 14.f;
    }

    // 分类胶囊筛选 + 搜索框
    std::vector<std::wstring> categories{L"全部工具 " + std::to_wstring(tools().size())};
    for (int i = 0; i < groupCount; ++i) {
        int n = 0;
        for (auto &t : tools()) {
            if (int(t.group) == i)
                ++n;
        }
        categories.push_back(shortGroupName(Group(i)) + L" " + std::to_wstring(n));
    }
    float bx = x;
    for (int i = 0; i < int(categories.size()); ++i) {
        float bw = std::max(72.f, textWidth(categories[i], 12) + 24.f);
        if (bx + bw > x + w) {
            y += 38.f;
            bx = x;
        }
        const bool active = filter == i - 1;
        Color fg = Blue, bg = Soft;
        if (i > 0) {
            const auto accent = groupAccent(Group(i - 1));
            fg = accent.fg;
            bg = accent.bg;
        }
        Rect pill{bx, y, bw, 32};
        scene.fill(pill, active ? bg : White, 16);
        scene.border(pill, active ? fg : Border, 16);
        scene.text(pill, categories[i], 12, active ? fg : Muted, active, true);
        scene.hits.push_back({pill, scene.clip, [this, i] {
                                  filter = i - 1;
                                  scroll = 0;
                                  refresh();
                              }});
        bx += bw + 8.f;
    }
    y += 44.f;

    auto &searchText = state.value("__dashboard.search", L"");
    scene.text({x, y, 76, 36}, L"筛选工具", 12.5f, Muted);
    scene.input("__dashboard.search", {x + 76, y, std::min(w - 76, 420.f), 36}, searchText);
    const std::wstring query = trimQuery(searchText);
    y += 50.f;

    // 整卡可点击工具网格（现代清爽卡片 + 右上角星标）
    int cols = std::clamp(int((w + 14) / 290), 1, 4);
    float gap = 14.f;
    float cell = (w - gap * (cols - 1)) / cols;
    float cardH = 122.f;
    int index = 0;
    for (std::size_t i = 0; i < tools().size(); ++i) {
        const auto &tool = tools()[i];
        if ((filter >= 0 && int(tool.group) != filter) || !matchesSearch(tool, query))
            continue;
        const auto accent = groupAccent(tool.group);
        const bool fav = isFavorite(tool.id);
        float cx = x + (index % cols) * (cell + gap), cy = y + (index / cols) * (cardH + gap);
        Rect card{cx, cy, cell, cardH};
        scene.fill(card, White, 11);
        scene.border(card, Border, 11);
        scene.fill({cx + 14, cy + 14, 38, 38}, accent.bg, 9);
        scene.text({cx + 14, cy + 14, 38, 38}, tool.icon.empty() ? L"工" : tool.icon, 16, accent.fg, true, true);
        scene.text({cx + 62, cy + 12, cell - 102, 24}, tool.name, 13.5f, Ink, true);
        scene.text({cx + 62, cy + 36, cell - 102, 18}, groupName(tool.group), 11, accent.fg);
        scene.text({cx + 14, cy + 62, cell - 28, 48}, tool.description, 12, Muted, false, false, true);

        // 先放整卡点击，再放右上角星标（hits 从后往前命中，星标优先级更高）
        scene.hits.push_back({card, scene.clip, [this, i] { open(int(i)); }});
        Rect starRect{cx + cell - 36, cy + 10, 26, 26};
        scene.text(starRect, fav ? L"★" : L"☆", 14, fav ? StarGold : Muted, false, true);
        scene.hits.push_back({starRect, scene.clip, [this, id = tool.id, name = tool.name] {
                                  toggleFavorite(id, name);
                              }});
        ++index;
    }
    if (index == 0) {
        scene.text({x, y + 14, w, 40}, L"没有找到匹配的工具，请换个关键词或分类。", 13, Muted, false, true);
        y += 58.f;
    }
    contentHeight_ = y + float((index + cols - 1) / cols) * (cardH + gap) + scroll - top + 30.f;
}
} // 命名空间 toolbox
