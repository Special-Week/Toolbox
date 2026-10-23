// 通用分析/测量数据拟合页面；使用输入量与响应量，不绑定特定仪器或样品。
#include "curve_fit.hpp"
#include <cstring>

namespace toolbox {
namespace {
constexpr std::size_t rowsPerPage = 10;
using DataPoints = std::vector<std::pair<double, double>>;
const std::array<std::pair<const wchar_t *, const wchar_t *>, 6> example{{
    {L"0", L"0.197"},
    {L"5", L"0.208"},
    {L"10", L"0.231"},
    {L"20", L"0.263"},
    {L"40", L"0.338"},
    {L"60", L"0.439"},
}};

std::wstring trimmed(const std::wstring &text) {
    auto first = text.find_first_not_of(L" \t\r\n\f\v");
    return first == std::wstring::npos
               ? L""
               : text.substr(first, text.find_last_not_of(L" \t\r\n\f\v") - first + 1);
}

std::wstring caption(std::wstring name, const wchar_t *fallback, const wchar_t *variable,
                     const std::wstring &unit) {
    name = trimmed(name);
    auto suffix = trimmed(unit);
    return (name.empty() ? fallback : name) + std::wstring(L" ") + variable +
           (suffix.empty() ? L"" : L" (" + suffix + L")");
}

std::wstring optionalNumber(std::optional<double> value, const wchar_t *missing) {
    return value ? format(*value) : missing;
}

std::wstring equation(const CurveFit &fit, bool fullPrecision) {
    std::wstring text = L"y = ";
    for (std::size_t power = fit.coefficients.size(); power-- > 0;) {
        double value = fit.coefficients[power];
        if (power + 1 != fit.coefficients.size()) {
            text += value < 0 ? L" - " : L" + ";
        } else if (value < 0) {
            text += L"-";
        }
        text += fullPrecision ? inputNumber(std::abs(value)) : format(std::abs(value));
        if (power == 1) {
            text += L" × x";
        } else if (power > 1) {
            text += L" × x^" + std::to_wstring(power);
        }
    }
    return text;
}

std::wstring report(const CurveFit &fit, const DataPoints &points, const std::wstring &xLabel,
                    const std::wstring &yLabel, const Tool &tool) {
    auto numberOr = [](std::optional<double> value, const wchar_t *missing) {
        return value ? inputNumber(*value) : std::wstring(missing);
    };
    std::wstring text =
        L"校准曲线拟合\r\n" + equation(fit, true) + L"\r\n输入：" + xLabel + L"\r\n响应：" + yLabel + L"\r\n";
    for (std::size_t power = 0; power < fit.coefficients.size(); ++power) {
        text += L"K" + std::to_wstring(power) + L" = " + inputNumber(fit.coefficients[power]) + L"\r\n";
    }
    text += L"相关系数 r = " + numberOr(fit.correlation, L"未定义") + L"\r\n决定系数 R² = " +
            numberOr(fit.rSquared, L"未定义") + L"\r\nRMSE = " + inputNumber(fit.rmse) +
            L"\r\n残差标准差 = " + numberOr(fit.residualSd, L"无剩余自由度") +
            L"\r\n\r\n序号\tx\ty\t拟合 y\t残差 y-拟合 y\r\n";
    for (std::size_t index = 0; index < points.size(); ++index) {
        text += std::to_wstring(index + 1) + L"\t" + inputNumber(points[index].first) + L"\t" +
                inputNumber(points[index].second) + L"\t" + inputNumber(fit.fitted[index]) + L"\t" +
                inputNumber(fit.residuals[index]) + L"\r\n";
    }
    for (const auto &section : calculationNotes(tool)) {
        text += L"\r\n" + section.first + L"\r\n" + section.second + L"\r\n";
    }
    return text;
}

void copyText(HWND owner, const std::wstring &text) {
    // 剪贴板成功接管内存后置空句柄；任一失败路径均自动释放内存并关闭剪贴板。
    struct Allocation {
        HGLOBAL memory = nullptr;
        ~Allocation() {
            if (memory) {
                GlobalFree(memory);
            }
        }
    } allocation;
    std::size_t bytes = (text.size() + 1) * sizeof(wchar_t);
    allocation.memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
    require(allocation.memory != nullptr, "无法分配剪贴板内存");
    void *buffer = GlobalLock(allocation.memory);
    require(buffer != nullptr, "无法写入剪贴板内存");
    std::memcpy(buffer, text.c_str(), bytes);
    GlobalUnlock(allocation.memory);
    require(OpenClipboard(owner) != FALSE, "剪贴板正忙，请稍后重试");
    struct ClipboardGuard {
        ~ClipboardGuard() { CloseClipboard(); }
    } guard;
    require(EmptyClipboard() != FALSE, "无法清空剪贴板");
    require(SetClipboardData(CF_UNICODETEXT, allocation.memory) != nullptr, "无法复制拟合结果");
    allocation.memory = nullptr;
}

std::pair<std::size_t, std::size_t> pageRange(Page &p, const std::string &field, std::size_t count) {
    if (count <= rowsPerPage) {
        return {0, count};
    }
    int pages = int((count + rowsPerPage - 1) / rowsPerPage);
    auto key = p.key(field);
    int &page = p.state.choice(key, 0);
    page = std::clamp(page, 0, pages - 1);
    auto button = [&](float x, const wchar_t *label, bool enabled, int step) {
        Rect rect{x, p.y, 80, 32};
        if (enabled) {
            p.ui.button(rect, label, [owner = &p.app, key, step, pages] {
                auto &value = owner->state.choices[key];
                value = std::clamp(value + step, 0, pages - 1);
                owner->refresh();
            });
        } else {
            p.ui.fill(rect, Soft, 5);
            p.ui.text(rect, label, 13, Muted, false, true);
        }
    };
    button(p.x, L"上一页", page > 0, -1);
    button(p.x + 88, L"下一页", page + 1 < pages, 1);
    bool compact = p.w < 420;
    p.ui.text({compact ? p.x : p.x + 180, p.y + (compact ? 36.f : 0.f), compact ? p.w : p.w - 180, 32},
              L"第 " + std::to_wstring(page + 1) + L" / " + std::to_wstring(pages) + L" 页 · 共 " +
                  std::to_wstring(count) + L" 组",
              12, Muted);
    p.y += compact ? 76 : 44;
    auto start = std::size_t(page) * rowsPerPage;
    return {start, std::min(start + rowsPerPage, count)};
}

DataPoints inputPoints(Page &p, std::size_t count, const std::wstring &xLabel, const std::wstring &yLabel) {
    auto range = pageRange(p, "input.page", count);
    float cell = (p.w - 64) / 2;
    p.ui.text({p.x, p.y, 44, 32}, L"序号", 13, Ink, true);
    p.ui.text({p.x + 60, p.y, cell, 32}, xLabel, 13, Ink, true);
    p.ui.text({p.x + 64 + cell, p.y, cell, 32}, yLabel, 13, Ink, true);
    p.y += 36;
    DataPoints points;
    for (std::size_t index = 0; index < count; ++index) {
        const auto defaults = index < example.size() ? example[index] : std::make_pair(L"", L"");
        auto xKey = p.key("point." + std::to_string(index) + ".x");
        auto yKey = p.key("point." + std::to_string(index) + ".y");
        auto &x = p.state.value(xKey, defaults.first);
        auto &y = p.state.value(yKey, defaults.second);
        if (index >= range.first && index < range.second) {
            p.ui.text({p.x, p.y, 44, 36}, std::to_wstring(index + 1), 13, Blue);
            p.ui.input(xKey, {p.x + 60, p.y, cell, 36}, x);
            p.ui.input(yKey, {p.x + 64 + cell, p.y, cell, 36}, y);
            p.y += 44;
        }
        // 隐藏页同样参与读取和拟合；先生成可见控件，再报告全部数据中的首个错误。
        auto read = [&](const std::wstring &text, const wchar_t *variable) {
            try {
                return parse(utf8(text));
            } catch (const std::exception &error) {
                if (p.inputError.empty()) {
                    p.inputError =
                        L"第 " + std::to_wstring(index + 1) + L" 组 " + variable + L"：" + wide(error.what());
                }
                return std::numeric_limits<double>::quiet_NaN();
            }
        };
        points.push_back({read(x, L"x"), read(y, L"y")});
    }
    return points;
}

void resultRows(Page &p, const CurveFit &fit, const DataPoints &points) {
    p.heading(L"各点拟合结果与残差");
    auto range = pageRange(p, "result.page", points.size());
    int columns = p.w < 600 ? 2 : 4;
    float cell = (p.w - 16) / columns;
    for (std::size_t index = range.first; index < range.second; ++index) {
        const std::array<std::wstring, 4> labels{{
            std::to_wstring(index + 1) + L".  x = " + format(points[index].first),
            L"y = " + format(points[index].second),
            L"拟合 y = " + format(fit.fitted[index]),
            L"残差 = " + format(fit.residuals[index]),
        }};
        std::vector<float> heights;
        float height = 16;
        for (int start = 0; start < 4; start += columns) {
            float rowHeight = 24;
            for (int column = 0; column < columns; ++column) {
                rowHeight =
                    std::max(rowHeight, p.app.paragraphHeight(labels[start + column], cell - 8, 12.5f));
            }
            heights.push_back(rowHeight);
            height += rowHeight + 4;
        }
        p.ui.fill({p.x, p.y, p.w, height}, index % 2 == 0 ? Soft : White, 4);
        float rowY = p.y + 8;
        for (std::size_t row = 0; row < heights.size(); ++row) {
            for (int column = 0; column < columns; ++column) {
                p.ui.text({p.x + 8 + column * cell, rowY, cell - 8, heights[row]},
                          labels[row * columns + column], 12.5f, Ink, false, false, true);
            }
            rowY += heights[row] + 4;
        }
        p.y += height + 6;
    }
}
} // namespace

void renderCurveFit(Page &p) {
    p.heading(L"拟合设置与校准数据");
    auto kind = CurveFitKind(p.select(
        "model", L"拟合模型", {L"一次（含常数项）", L"一次（过原点）", L"二次多项式", L"三次多项式"}));
    if (p.collapse("axis.labels", L"坐标名称与单位（可选）")) {
        p.textInput("x_name", L"输入量名称", L"输入量");
        p.textInput("x_unit", L"输入量单位", L"");
        p.textInput("y_name", L"响应量名称", L"响应量");
        p.textInput("y_unit", L"响应量单位", L"");
    }
    auto xLabel = caption(p.state.value(p.key("x_name"), L"输入量"), L"输入量", L"x",
                          p.state.value(p.key("x_unit"), L""));
    auto yLabel = caption(p.state.value(p.key("y_name"), L"响应量"), L"响应量", L"y",
                          p.state.value(p.key("y_unit"), L""));
    int count = p.count("points", L"校准数据", 6, 2, int(CurveFit::maximumPoints));
    p.note(L"每行一组 x、y，各列单位保持一致。");
    auto points = inputPoints(p, std::size_t(count), xLabel, yLabel);
    p.check();
    CurveFit fit(points, kind);

    p.y += 12;
    const std::wstring fittedEquation = equation(fit, false);
    float innerWidth = p.w - 28;
    float equationHeight = p.app.paragraphHeight(fittedEquation, innerWidth, 17);
    float cardHeight = 14 + 36 + 8 + equationHeight + 14;
    p.ui.fill({p.x, p.y, p.w, cardHeight}, Soft, 7);
    p.ui.text({p.x + 14, p.y + 14, 90, 34}, L"拟合方程", 14, Ink, true);
    p.ui.button({p.x + 116, p.y + 14, 124, 34}, L"复制拟合结果",
                [owner = &p.app, text = report(fit, points, xLabel, yLabel, p.tool)] {
                    copyText(owner->handle(), text);
                });
    p.ui.text({p.x + 14, p.y + 58, innerWidth, equationHeight}, fittedEquation, 17, Blue, false, false, true);
    p.y += cardHeight + 12;
    const wchar_t *coefficientNames[] = {L"K0（常数项）", L"K1（一次项）", L"K2（二次项）", L"K3（三次项）"};
    Metrics coefficients;
    for (std::size_t power = fit.coefficients.size(); power-- > 0;) {
        coefficients.push_back({coefficientNames[power], format(fit.coefficients[power])});
    }
    p.metrics(L"方程系数", coefficients);
    p.metrics(L"相关性与拟合质量",
              {{L"线性相关系数 r（x 与 y）", optionalNumber(fit.correlation, L"响应量无变化，未定义")},
               {L"决定系数 R²", optionalNumber(fit.rSquared, L"响应量无变化，未定义")},
               {L"均方根误差 RMSE", format(fit.rmse)},
               {L"残差标准差", optionalNumber(fit.residualSd, L"无剩余自由度")},
               {L"最大绝对残差", format(fit.maximumResidual)},
               {L"剩余自由度 n-p", std::to_wstring(points.size() - CurveFit::parameterCount(kind))}});
    if (!fit.residualSd) {
        p.note(L"无剩余自由度，残差标准差不计算。");
    }
    p.heading(L"校准曲线");
    drawCurveFitChart(p, fit, points, xLabel, yLabel, false);
    if (p.collapse("residual.chart", L"残差图")) {
        drawCurveFitChart(p, fit, points, xLabel, L"残差（与响应量同单位）", true);
    }
    resultRows(p, fit, points);
}
} // namespace toolbox
