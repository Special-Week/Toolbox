// 页面仅组织输入输出，数值算法集中在 models 与 core 中；单位换算采用现代双栏 + 双列卡片网格。
#include "toolbox/gui.hpp"
#include <algorithm>
namespace toolbox {
namespace {
bool unitMatches(const std::wstring &name, const std::wstring &code, const std::wstring &query) {
    if (query.empty())
        return true;
    auto fold = [](wchar_t v) { return v >= L'A' && v <= L'Z' ? wchar_t(v + (L'a' - L'A')) : v; };
    auto has = [&](const std::wstring &s) {
        return std::search(s.begin(), s.end(), query.begin(), query.end(),
                           [&](wchar_t a, wchar_t b) { return fold(a) == fold(b); }) != s.end();
    };
    return has(name) || has(code);
}
} // namespace

void renderPage(Page &p) {
    if (p.tool.kind == Kind::Unit) {
        p.beginSplit(0.38f);
        p.heading(L"换算参数");
        double value = p.number("value", L"输入数值", 1);
        const auto &list = units(p.tool.dimension);
        std::vector<std::wstring> labels;
        for (auto &u : list) {
            labels.push_back(u.name + L" · " + u.code);
        }
        int input = p.select("unit", L"输入单位", labels);
        std::wstring filterText = p.textInput("filter", L"筛选结果单位", L"");
        auto first = filterText.find_first_not_of(L" \t\r\n");
        std::wstring query =
            first == std::wstring::npos
                ? L""
                : filterText.substr(first, filterText.find_last_not_of(L" \t\r\n") - first + 1);
        p.note(L"点击右侧任意结果卡片可一键复制数值；点击右上角 ⇄ 可设该单位为输入源。");
        p.check();

        p.rightColumn();
        p.heading(format(value) + L" " + list[input].code + L"  全部换算结果");

        struct UnitItem {
            std::size_t index;
            double result;
        };
        std::vector<UnitItem> visible;
        for (std::size_t i = 0; i < list.size(); ++i) {
            double result = convert(p.tool.dimension, value, input, i);
            const auto &source = list[input];
            const auto &target = list[i];
            if (i)
                p.processSteps.push_back(L"");
            if (i == std::size_t(input)) {
                p.processSteps.push_back(target.name + L"：源单位和目标单位相同，直接返回 " + inputNumber(value) +
                                         L" " + target.code + L"。");
            } else if (p.tool.dimension == Dimension::Temperature) {
                double kelvin = std::max(0.0, std::fma(value, source.factor, source.offset));
                p.processSteps.push_back(target.name + L"：K = max(输入 × fₛ + oₛ, 0) = max(" + inputNumber(value) +
                                         L" × " + inputNumber(source.factor) + L" + " + inputNumber(source.offset) +
                                         L", 0) = " + p.processValue(kelvin) + L"；目标值 = (K − oₜ)/fₜ = (" +
                                         p.processValue(kelvin) + L" − " + inputNumber(target.offset) + L") / " +
                                         inputNumber(target.factor) + L" = " + p.processValue(result) + L" " +
                                         target.code + L"。");
            } else {
                p.processSteps.push_back(target.name + L"：目标值 = 输入值 × (源系数 / 目标系数) = " +
                                         inputNumber(value) + L" × (" + inputNumber(source.factor) + L" / " +
                                         inputNumber(target.factor) + L") = " + p.processValue(result) + L" " +
                                         target.code + L"。");
            }
            if (unitMatches(target.name, target.code, query)) {
                visible.push_back({i, result});
            }
        }

        if (visible.empty()) {
            p.note(L"没有匹配的单位，请尝试其他名称或符号。");
        } else {
            const int cols = p.w >= 420.f ? 2 : 1;
            const float gap = 10.f;
            const float cardW = (p.w - gap * (cols - 1)) / cols;
            const float cardH = 76.f;
            for (std::size_t pos = 0; pos < visible.size(); ++pos) {
                const auto &item = visible[pos];
                const auto &u = list[item.index];
                const bool isSource = (int(item.index) == input);
                const float cx = p.x + float(pos % cols) * (cardW + gap);
                const float cy = p.y + float(pos / cols) * (cardH + gap);
                const std::wstring formatted = formatUnit(item.result);
                Rect card{cx, cy, cardW, cardH};
                p.ui.fill(card, isSource ? Soft : SurfaceSubtle, 9);
                p.ui.border(card, isSource ? Blue : Border, 9);
                if (isSource) {
                    p.ui.fill({cx + 1, cy + 8, 3.5f, cardH - 16}, Blue, 2);
                }
                const float swapW = isSource ? 0.f : 26.f;
                p.ui.text({cx + 12, cy + 8, cardW - 24 - swapW, 20}, u.name + L"  [" + u.code + L"]", 11.5f,
                          isSource ? Blue : Muted, isSource);
                p.ui.text({cx + 12, cy + 32, cardW - 24, 34}, formatted, 16.5f, isSource ? Blue : Ink, true, false,
                          false, true);
                p.ui.hits.push_back({card, p.ui.clip,
                                     [owner = &p.app, formatted, name = u.name] {
                                         owner->copyToClipboard(formatted, L"已复制 " + name + L"：" + formatted);
                                     }});
                if (!isSource) {
                    Rect swapRect{cx + cardW - 32, cy + 6, 24, 24};
                    p.ui.fill(swapRect, White, 6);
                    p.ui.border(swapRect, Border, 6);
                    p.ui.text(swapRect, L"⇄", 11.5f, Muted, false, true);
                    p.ui.hits.push_back({swapRect, p.ui.clip,
                                         [owner = &p.app, unitKey = p.key("unit"), idx = int(item.index),
                                          name = u.name] {
                                             owner->state.choices[unitKey] = idx;
                                             owner->showToast(L"输入源单位已切换为：" + name);
                                         }});
                }
            }
            p.y += float((visible.size() + cols - 1) / cols) * (cardH + gap);
        }
        p.endSplit();
        return;
    }
    switch (p.tool.group) {
    case Group::Its90:
        renderSensors(p);
        break;
    case Group::Pressure:
        renderPressure(p);
        break;
    case Group::Humidity:
        renderHumidity(p);
        break;
    case Group::Errors:
        renderErrors(p);
        break;
    case Group::Other:
        renderOther(p);
        break;
    case Group::Gauges:
        renderGauges(p);
        break;
    default:
        break;
    }
    p.endSplit();
    if (p.app.exportJob.active) {
        auto progress = p.app.exportJob.progress.load(), total = p.app.exportJob.total.load();
        p.note(L"导出中 " + std::to_wstring(total ? progress * 100 / total : 0) + L"%");
        p.button(L"取消导出", [owner = &p.app] {
            owner->exportJob.cancel = true;
            owner->refresh();
        });
    } else {
        std::lock_guard<std::mutex> lock(p.app.exportJob.mutex);
        if (!p.app.exportJob.message.empty()) {
            p.note(p.app.exportJob.message);
        }
    }
}
} // 命名空间 toolbox
