// 量规页面使用已有 DirectWrite 文本与原生编辑框；横向表格不截获整页纵向滚轮。
#include "toolbox/gauges.hpp"
#include "toolbox/gui.hpp"
#include <cstring>
#include <sstream>

namespace toolbox {
namespace {
using GaugeType = gauges::Type;
std::map<std::string, gauges::PageData> gaugeCache;
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
        p.ui.fill({p.x, p.y, p.w, height}, row % 2 == 0 ? White : TableStripe);
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
        p.note(L"提示：此表较宽，可使用下方滑块或左右按钮查看全部列。", Muted);
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
        p.ui.fill({track.x + travel * float(offset) / float(maximum), track.y, thumb, track.h}, BorderStrong, 5);
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
} // 命名空间
void recordUnifiedGaugeEdit(State &state, const std::string &key) {
    const std::string prefix = "gauge_unified.gauge.";
    if (key.compare(0, prefix.size(), prefix) != 0)
        return;
    const auto visible = gaugeCache.find("gauge_unified");
    if (visible == gaugeCache.end())
        return;
    for (const auto &field : visible->second.fields) {
        if (field.key != "size" && field.key != "tpi" && field.key != "series")
            continue;
        const auto fieldKey = prefix + field.key;
        std::string value;
        if (!field.select && key == fieldKey) {
            value = utf8(state.values.at(key));
        } else if (key == fieldKey + (field.select ? ".select" : ".suggestion")) {
            const int index = state.choices.at(key) - (field.select ? 0 : 1);
            if (index < 0 || std::size_t(index) >= field.options.size())
                return;
            value = field.options[std::size_t(index)].value;
        } else {
            continue;
        }
        const auto saved = prefix + "input.";
        state.values[saved + field.key] = wide(value);
        state.values[saved + "_threadChange"] = wide(field.key);
        if (field.key == "tpi") {
            const auto size = state.values.find(saved + "size");
            if (size != state.values.end()) {
                try {
                    const auto series = gauges::unSeries(utf8(size->second), parse(value));
                    if (!series.empty())
                        state.values[saved + "_lastSeries"] = wide(series);
                } catch (const std::exception &) {
                    // 不完整的牙数暂时无法识别系列。
                }
            }
        } else if (field.key == "series" && value != "非标 / 自定义") {
            state.values[saved + "_lastSeries"] = wide(value);
        }
        return;
    }
}

void renderGauges(Page &p) {
    auto type = typeOf(p.tool.kind);
    if (gaugeCache.find(p.tool.id) == gaugeCache.end()) {
        gauges::State saved;
        const auto prefix = p.key("gauge.input.");
        for (const auto &entry : p.state.values)
            if (entry.first.compare(0, prefix.size(), prefix) == 0)
                saved[entry.first.substr(prefix.size())] = utf8(entry.second);
        gaugeCache[p.tool.id] = gauges::page(type, std::move(saved), "");
    }
    auto &previous = gaugeCache[p.tool.id];
    auto input = previous.state;
    std::string changed;
    const auto pending = p.state.values.find(p.key("gauge.input._threadChange"));
    const bool hasThreadEdit = type == GaugeType::Unified && pending != p.state.values.end();
    if (hasThreadEdit) {
        changed = utf8(pending->second);
        p.state.values.erase(pending);
        input.erase("_threadChange");
        for (const auto *field : {"size", "tpi", "series", "_lastSeries"}) {
            const auto value = p.state.values.find(p.key("gauge.input.") + field);
            if (value != p.state.values.end())
                input[field] = utf8(value->second);
        }
    }
    for (const auto &field : previous.fields) {
        if (hasThreadEdit && (field.key == "size" || field.key == "tpi" || field.key == "series"))
            continue;
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
    // 旧记录可能同时存有尺寸和牙数的原始输入，却没有事件顺序。
    if (!hasThreadEdit && type == GaugeType::Unified && changed == "size" &&
        input["tpi"] != previous.state["tpi"]) {
        try {
            const auto series = gauges::unSeries(previous.state["size"], parse(input["tpi"]));
            if (!series.empty())
                input["_lastSeries"] = series;
        } catch (const std::exception &) {
            // 不完整的牙数继续沿用既有系列。
        }
    }
    if (!changed.empty())
        previous = gauges::page(type, std::move(input), changed);
    const auto &data = previous;
    // 保存规范字段值，避免恢复时把旧下拉索引解释成另一种规格。
    for (const auto &entry : data.state)
        p.state.values[p.key("gauge.input." + entry.first)] = wide(entry.second);
    p.beginSplit(0.40f);
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
            p.ui.text({p.x, p.y, p.w, 22}, wide(field.label), 12.5f, Muted);
            p.y += 24;
            float suggestionWidth = field.options.empty() ? 0 : std::min(156.f, p.w * .36f);
            p.ui.input(key, {p.x, p.y, p.w - (suggestionWidth ? suggestionWidth + 8 : 0), 34}, wide(current));
            if (suggestionWidth) {
                std::vector<std::wstring> labels{L"常用规格"};
                for (const auto &option : field.options) {
                    labels.push_back(wide(option.label));
                }
                p.state.choices[key + ".suggestion"] = 0;
                p.ui.combo(key + ".suggestion", {p.x + p.w - suggestionWidth, p.y, suggestionWidth, 34},
                           labels, 0);
            }
            p.y += 42;
        }
    }

    p.rightColumn();
    if (!data.error.empty()) {
        p.error(wide(data.error));
        p.endSplit();
    } else if (data.report) {
        const auto &report = *data.report;
        p.processSteps = report.process;
        p.heading(wide(report.title));
        std::string inputs = "输入参数\n";
        for (const auto &field : data.fields) {
            auto value = data.state.find(field.key);
            if (value != data.state.end()) {
                inputs += field.label + "：" + value->second + "\n";
            }
        }
        auto text = wide("工具标识：" + p.tool.id + "\n软件版本：Toolbox C++ " TOOLBOX_VERSION
                         "\n数值精度：计算尺寸按界面显示为五位小数。\n\n" + inputs + "\n" + report.text(type));
        p.button(L"复制完整结果", [owner = &p.app, text] {
            owner->copyToClipboard(text, L"已复制量规完整结果");
        });
        if (!report.notes.empty()) {
            std::vector<std::wstring> notes;
            for (const auto &note : report.notes)
                notes.push_back(wide(note));
            p.details("gauge.notes", L"⚠ 取值说明与适用限制 · " + std::to_wstring(notes.size()) + L" 项",
                      notes);
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
                p.ui.text({p.x + left, p.y, p.w - left, h}, wide(m.second.display), 12.5f, Ink, false, false, false,
                          true);
                p.y += h + 4;
            }
        }
        p.endSplit();

        // 宽结果表格统一 100% 全宽面板展示，自带横向滚动条，杜绝任何列裁切
        if (!report.tables.empty()) {
            auto panelFill = p.ui.draws.size();
            const float panelTop = p.y;
            const float outerX = p.x, outerW = p.w;
            p.ui.fill({outerX, panelTop, outerW, 0}, White, 10);
            p.ui.border({outerX, panelTop, outerW, 0}, Border, 10);
            p.x = outerX + 18.f;
            p.w = outerW - 36.f;
            p.y = panelTop + 12.f;
            for (std::size_t i = 0; i < report.tables.size(); ++i) {
                drawTable(p, report, report.tables[i], i);
            }
            const float panelH = (p.y + 12.f) - panelTop;
            p.ui.draws[panelFill].rect.h = panelH;
            p.ui.draws[panelFill + 1].rect.h = panelH;
            p.x = outerX;
            p.w = outerW;
            p.y = panelTop + panelH + 14.f;
        }
    } else {
        p.endSplit();
    }
}
void clearGaugeCache(const std::string &id) { gaugeCache.erase(id); }
} // 命名空间 toolbox
