// 量规页面使用已有 DirectWrite 文本与原生编辑框；横向表格不截获整页纵向滚轮。
#include "toolbox/gauges.hpp"
#include "toolbox/gui.hpp"
#include <cstring>
#include <sstream>

namespace toolbox {
namespace {
using GaugeType = gauges::Type;
GaugeType typeOf(Kind kind) {
    switch (kind) {
    case Kind::GaugeMetric:
        return GaugeType::Metric;
    case Kind::GaugeTrapezoidal:
        return GaugeType::Trapezoidal;
    case Kind::GaugeUnified:
        return GaugeType::Unified;
    case Kind::GaugePipe:
        return GaugeType::Pipe;
    case Kind::GaugeRebar:
        return GaugeType::Rebar;
    default:
        return GaugeType::Smooth;
    }
}
int indexOf(const std::vector<gauges::Option> &options, const std::string &value) {
    for (std::size_t i = 0; i < options.size(); ++i) {
        if (options[i].value == value) {
            return int(i);
        }
    }
    return 0;
}
void copy(HWND owner, const std::wstring &text) {
    // 只有 SetClipboardData 成功后才转移内存所有权，失败路径均释放内存。
    struct Allocation {
        HGLOBAL handle = nullptr;
        ~Allocation() {
            if (handle) {
                GlobalFree(handle);
            }
        }
    } allocation;
    const auto bytes = (text.size() + 1) * sizeof(wchar_t);
    allocation.handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
    require(allocation.handle != nullptr, "无法分配剪贴板内存");
    auto buffer = GlobalLock(allocation.handle);
    require(buffer != nullptr, "无法写入剪贴板内存");
    std::memcpy(buffer, text.c_str(), bytes);
    GlobalUnlock(allocation.handle);
    require(OpenClipboard(owner) != FALSE, "剪贴板正忙，请稍后重试");
    struct Guard {
        ~Guard() { CloseClipboard(); }
    } guard;
    require(EmptyClipboard() != FALSE, "无法清空剪贴板");
    require(SetClipboardData(CF_UNICODETEXT, allocation.handle) != nullptr, "无法复制量规结果");
    allocation.handle = nullptr;
}
float textWidth(Page &p, const std::string &text) {
    std::istringstream lines(text);
    std::string line;
    float width = 0;
    while (std::getline(lines, line)) {
        width = std::max(width, p.app.textWidth(wide(line), 12.5f));
    }
    return width;
}
Rect intersect(Rect a, Rect b) {
    const float x = std::max(a.x, b.x), y = std::max(a.y, b.y);
    return {x, y, std::max(0.f, std::min(a.x + a.w, b.x + b.w) - x),
            std::max(0.f, std::min(a.y + a.h, b.y + b.h) - y)};
}
void drawTable(Page &p, const gauges::Report &report, const gauges::Table &table, std::size_t index) {
    p.heading(wide(table.title));
    std::vector<float> widths;
    for (std::size_t column = 0; column < table.headings.size(); ++column) {
        float width = std::max(56.f, textWidth(p, table.headings[column]));
        // 相同列结构的工作规和校对规统一列宽，并使缺失值与数值居中对齐。
        for (const auto &other : report.tables) {
            if (other.headings == table.headings) {
                for (const auto &row : other.rows) {
                    width = std::max(width, textWidth(p, row[column].display));
                }
            }
        }
        widths.push_back(width + 24);
    }
    float total = std::accumulate(widths.begin(), widths.end(), 0.f);
    bool scrolling = total > p.w;
    if (!scrolling) {
        for (auto &width : widths) {
            width += (p.w - total) / float(widths.size());
        }
    }
    const std::string key = p.key("gauge.table." + std::to_string(index) + ".x");
    int &offset = p.state.choice(key, 0);
    // DPI 缩放可能只溢出不足一个逻辑像素，向上取整以免滚动轨道出现零分母。
    int maximum = int(std::ceil(std::max(0.f, total - p.w)));
    offset = std::clamp(offset, 0, maximum);
    Rect original = p.ui.clip;
    p.ui.clip = intersect(original, {p.x, p.y, p.w, 100000});
    float x = p.x - offset;
    p.ui.fill({p.x, p.y, p.w, 38}, Soft, 4);
    for (std::size_t i = 0; i < table.headings.size(); ++i) {
        p.ui.text({x + 6, p.y, widths[i] - 12, 38}, wide(table.headings[i]), 12, Muted, true, true);
        x += widths[i];
    }
    p.y += 40;
    for (std::size_t row = 0; row < table.rows.size(); ++row) {
        float height = 38;
        for (const auto &cell : table.rows[row]) {
            height = std::max(
                height, 18.f + 18.f * float(1 + std::count(cell.display.begin(), cell.display.end(), '\n')));
        }
        if (row % 2 == 0) {
            p.ui.fill({p.x, p.y, p.w, height}, White);
        }
        x = p.x - offset;
        for (std::size_t i = 0; i < table.rows[row].size(); ++i) {
            p.ui.text({x + 6, p.y, widths[i] - 12, height}, wide(table.rows[row][i].display), 12.5f,
                      i ? Blue : Ink, false, true);
            x += widths[i];
        }
        p.y += height + 3;
    }
    p.ui.clip = original;
    if (scrolling) {
        p.ui.button({p.x, p.y, 28, 24}, L"‹", [owner = &p.app, key] {
            owner->state.choices[key] = std::max(0, owner->state.choices[key] - 160);
            owner->refresh();
        });
        p.ui.button({p.x + p.w - 28, p.y, 28, 24}, L"›", [owner = &p.app, key, maximum] {
            owner->state.choices[key] = std::min(maximum, owner->state.choices[key] + 160);
            owner->refresh();
        });
        const Rect track{p.x + 36, p.y + 6, p.w - 72, 12};
        float thumb = std::max(30.f, track.w * p.w / total), travel = track.w - thumb;
        p.ui.fill(track, Soft, 5);
        p.ui.fill({track.x + travel * float(offset) / float(maximum), track.y, thumb, track.h}, Border, 5);
        p.ui.hits.push_back(
            {track, original, [owner = &p.app, key, track, thumb, travel, maximum] {
                 if (travel > 0) {
                     owner->state.choices[key] =
                         int(std::clamp((owner->mouseX - track.x - thumb / 2) / travel, 0.f, 1.f) * maximum);
                 }
                 owner->refresh();
             }});
        p.y += 32;
    }
    p.y += 8;
}
} // namespace
void renderGauges(Page &p) {
    // 缓存仅保存本次会话的字段和文本；原生控件通过稳定键复用，输入时不会重建编辑框。
    static std::map<std::string, gauges::PageData> cache;
    auto &previous = cache[p.tool.id];
    auto input = previous.state;
    std::string changed;
    for (const auto &field : previous.fields) {
        auto key = p.key("gauge." + field.key);
        const auto saved = input[field.key];
        if (field.select) {
            int selection = p.state.choice(key + ".select", indexOf(field.options, saved));
            if (selection >= 0 && std::size_t(selection) < field.options.size()) {
                input[field.key] = field.options[selection].value;
            }
        } else {
            input[field.key] = utf8(p.state.value(key, wide(saved)));
            int &suggestion = p.state.choice(key + ".suggestion", 0);
            if (suggestion > 0 && std::size_t(suggestion) <= field.options.size()) {
                input[field.key] = field.options[suggestion - 1].value;
                suggestion = 0;
            }
        }
        if (input[field.key] != saved && changed.empty()) {
            changed = field.key;
        }
    }
    auto type = typeOf(p.tool.kind);
    auto data = gauges::page(type, std::move(input), changed);
    p.heading(L"计算参数");
    for (const auto &field : data.fields) {
        const auto key = p.key("gauge." + field.key);
        auto value = data.state.find(field.key);
        std::string current = value == data.state.end() ? "" : value->second;
        if (field.select) {
            std::vector<std::wstring> labels;
            for (const auto &option : field.options) {
                labels.push_back(wide(option.label));
            }
            if (labels.empty()) {
                labels.push_back(L"当前参数无可用选项");
            }
            p.state.choices[key + ".select"] = indexOf(field.options, current);
            p.select("gauge." + field.key + ".select", wide(field.label), labels);
        } else {
            p.state.values[key] = wide(current);
            p.ui.text({p.x, p.y, p.w, 24}, wide(field.label), 13, Muted);
            p.y += 26;
            float suggestionWidth = field.options.empty() ? 0 : std::min(180.f, p.w * .32f);
            p.ui.input(key, {p.x, p.y, p.w - (suggestionWidth ? suggestionWidth + 8 : 0), 36}, wide(current));
            if (suggestionWidth) {
                std::vector<std::wstring> labels{L"常用规格"};
                for (const auto &option : field.options) {
                    labels.push_back(wide(option.label));
                }
                p.state.choices[key + ".suggestion"] = 0;
                p.ui.combo(key + ".suggestion", {p.x + p.w - suggestionWidth, p.y, suggestionWidth, 36},
                           labels, 0);
            }
            p.y += 46;
        }
    }
    if (!data.error.empty()) {
        p.note(wide(data.error), Red);
    } else if (data.report) {
        const auto &report = *data.report;
        p.heading(wide(report.title));
        auto text = wide(report.text(type));
        p.button(L"复制完整结果", [owner = p.app.handle(), text] {
            try {
                copy(owner, text);
            } catch (const std::exception &e) {
                MessageBoxW(owner, wide(e.what()).c_str(), L"复制结果", MB_OK | MB_ICONINFORMATION);
            }
        });
        for (const auto &note : report.notes) {
            p.note(wide(note));
        }
        Metrics summary;
        for (std::size_t i = 0; i < std::min<std::size_t>(6, report.metrics.size()); ++i) {
            summary.push_back({wide(report.metrics[i].first), wide(report.metrics[i].second.display)});
        }
        p.metrics(type == GaugeType::Unified ? L"工件尺寸与公差" : L"基本尺寸与参数", summary);
        if (report.metrics.size() > 6 && p.collapse("gauge.details", L"几何与公差取值详情")) {
            for (std::size_t i = 6; i < report.metrics.size(); ++i) {
                const auto &m = report.metrics[i];
                float left = p.w * .55f, h = m.second.display.find('\n') == std::string::npos ? 30.f : 46.f;
                p.ui.text({p.x, p.y, left - 12, h}, wide(m.first), 12, Muted);
                p.ui.text({p.x + left, p.y, p.w - left, h}, wide(m.second.display), 12.5f, Ink);
                p.y += h + 4;
            }
        }
        for (std::size_t i = 0; i < report.tables.size(); ++i) {
            drawTable(p, report, report.tables[i], i);
        }
    }
    previous = std::move(data);
}
} // namespace toolbox
