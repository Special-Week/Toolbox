// 计算说明和计算过程共用同一套行内公式排版。
#include "toolbox/gui.hpp"
#include "toolbox/formula_layout.hpp"
#include "toolbox/math_engine.hpp"

namespace toolbox {
void Page::details(const std::string &field, const std::wstring &title,
                   const std::vector<std::wstring> &lines) {
    const auto k = key(field);
    const bool open = app.openDetails.count(k) != 0;
    const bool calculation = field == "calculation.process";
    const bool mathDetails = calculation || field == "calculation.details";
    const float padding = 18, inner = std::max(1.f, w - 2 * padding);
    struct Run { std::wstring text, latex; float width = 0, height = 0; bool paragraph = false; };
    struct Row { std::vector<Run> runs; float width = 0, height = 24; };
    std::vector<Row> rows;
    float height = 60;
    if (open) for (const auto &line : lines) {
        Row row;
        auto finish = [&] {
            height += row.height + (calculation ? 0 : 8);
            rows.push_back(std::move(row));
            row = Row{};
        };
        const auto displayed = mathDetails ? parenthesizeNegativeSubtrahends(line) : line;
        const auto tokens = mathDetails ? mathTokens(displayed) : std::vector<MathToken>{{displayed, L""}};
        for (const auto &token : tokens) {
            Run run;
            run.text = token.text;
            if (!token.latex.empty()) {
                if (const auto bitmap = renderMath(token.latex)) {
                    run.latex = token.latex;
                    run.width = bitmap->width / 2.f;
                    run.height = bitmap->height / 2.f;
                    if (run.width > inner) {
                        run.height *= inner / run.width;
                        run.width = inner;
                    }
                }
            }
            if (run.latex.empty()) {
                run.width = app.textWidth(run.text, 13);
                run.height = 24;
                if (run.width > inner) {
                    if (!row.runs.empty()) finish();
                    run.width = inner;
                    run.height = app.paragraphHeight(run.text, inner, 13);
                    run.paragraph = true;
                    row.height = std::max(row.height, run.height);
                    row.runs.push_back(std::move(run));
                    finish();
                    continue;
                }
            }
            if (!row.runs.empty() && row.width + run.width > inner) finish();
            row.width += run.width;
            row.height = std::max(row.height, run.height);
            row.runs.push_back(std::move(run));
        }
        if (!row.runs.empty() || tokens.empty()) finish();
    }
    ui.fill({x, y, w, height}, Soft, 8);
    ui.text({x + padding, y + 12, inner, 36}, (open ? L"▾  " : L"▸  ") + title, 14, Blue, true);
    ui.hits.push_back({{x, y, w, calculation ? 44.f : 60.f}, ui.clip, [owner = &app, k] {
        if (owner->openDetails.count(k)) owner->openDetails.erase(k);
        else owner->openDetails.insert(k);
        owner->refresh();
    }});
    float lineY = y + 52;
    for (const auto &row : rows) {
        float left = x + padding;
        for (const auto &run : row.runs) {
            if (!run.latex.empty()) {
                ui.math({left, lineY + (row.height - run.height) / 2, run.width, run.height}, run.text, run.latex);
            } else {
                ui.text({left, lineY, run.width, row.height}, run.text, 13, Ink, false, false, run.paragraph);
            }
            left += run.width;
        }
        lineY += row.height + (calculation ? 0 : 8);
    }
    y += height + 12;
}
}
