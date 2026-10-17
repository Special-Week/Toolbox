// 变送器和 B 样条工具，采样预览分页显示，不建立嵌套滚轮区域。
#include "toolbox/gui.hpp"
namespace toolbox {
void renderOther(Page &p) {
    if (p.tool.kind == Kind::Transmitter) {
        p.heading(L"变送器量程换算");
        int quantity = p.select("quantity", L"被测量类型", {L"压力", L"温度", L"湿度"});
        std::vector<std::wstring> options;
        if (quantity == 0) {
            for (auto &u : units(Dimension::Pressure)) {
                options.push_back(u.code);
            }
        } else {
            options = quantity == 1 ? std::vector<std::wstring>{L"°C", L"°F", L"K"}
                                    : std::vector<std::wstring>{L"%RH", L"°C（露点）", L"g/m³"};
        }
        int unit = p.select("unit." + std::to_string(quantity), L"量程单位", options),
            direction = p.select("direction", L"计算方向", {L"被测量 → 输出信号", L"输出信号 → 被测量"}),
            function = p.select("function", L"传递特性", {L"线性", L"开方"}),
            signal = p.select("signal", L"输出信号单位", {L"mA", L"V", L"mV"});
        double low = 0, high = 100;
        if (quantity == 1) {
            if (unit == 1) {
                low = 32;
                high = 212;
            } else if (unit == 2) {
                low = 273.15;
                high = 373.15;
            }
        } else if (quantity == 2) {
            if (unit == 1) {
                low = -20;
                high = 60;
            } else if (unit == 2) {
                high = 30;
            }
        }
        std::string profile = std::to_string(quantity) + "." + std::to_string(unit);
        auto symbol = options[unit];
        const wchar_t *signals[] = {L"mA", L"V", L"mV"};
        double a = p.number(profile + ".low", L"被测量下限 (" + symbol + L")", low),
               b = p.number(profile + ".high", L"被测量上限 (" + symbol + L")", high),
               sl = p.number("signal.low", L"输出信号下限", 4),
               sh = p.number("signal.high", L"输出信号上限", 20);
        double input = p.number(direction == 0 ? profile + ".input" : "signal.input",
                                direction == 0 ? L"输入被测量 (" + symbol + L")"
                                               : L"输入信号 (" + std::wstring(signals[signal]) + L")",
                                direction == 0 ? (low + high) / 2 : 12);
        p.check();
        if (quantity == 2) {
            require(unit != 0 || (a >= 0 && b <= 100), "相对湿度量程应在 0～100 %RH 内");
            require(unit != 2 || a >= 0, "绝对湿度量程不能为负");
        }
        if (quantity == 1 || (quantity == 2 && unit == 1)) {
            require(a >= (quantity == 1 ? (unit == 1 ? -459.67 : (unit == 2 ? 0 : -273.15)) : -273.15),
                    "温度量程不能低于绝对零度");
        }
        Range process(a, b), output(sl, sh);
        auto result = direction == 0 ? transmitterForward(process, output, function == 1, input)
                                     : transmitterInverse(process, output, function == 1, input);
        p.metrics(L"换算结果",
                  {{L"被测量值", format(direction == 0 ? input : result.first) + L" " + symbol},
                   {L"输出信号", format(direction == 0 ? result.first : input) + L" " + signals[signal]},
                   {L"量程比例", format(result.second * 100) + L" %"}});
        return;
    }
    p.heading(L"插值参数");
    double minimum = p.number("minimum", L"曲线最小 x", 0),
           maximum = p.number("maximum", L"曲线最大 x", 2650), step = p.number("step", L"采样步进", .5);
    int count = p.count("q.count", L"Q 型值点", 9, 2, 500);
    p.note(L"x 必须严格递增；新增点需要填写 x 与 y。");
    const std::vector<std::pair<double, double>> defaults{{250, 2002},   {550, 6531},   {850, 12285},
                                                          {1150, 18708}, {1450, 25475}, {1750, 32170},
                                                          {2050, 38455}, {2350, 43954}, {2650, 48340}};
    p.ui.text({p.x + 60, p.y, (p.w - 64) / 2, 28}, L"x", 13, Ink, true);
    p.ui.text({p.x + 64 + (p.w - 64) / 2, p.y, (p.w - 64) / 2, 28}, L"y", 13, Ink, true);
    p.y += 30;
    auto raw = p.pairs("q", L"Q", count, defaults, 270);
    p.check();
    std::vector<Point> points;
    for (auto a : raw) {
        points.push_back({a.first, a.second});
    }
    auto spline = std::make_shared<Spline>(points);
    auto samples = sampleCount(minimum, maximum, step);
    std::vector<Point> curve;
    for (int i = 0; i <= 180; ++i) {
        double x = std::fma(i / 180., maximum - minimum, minimum);
        curve.push_back({x, spline->y(x)});
    }
    p.metrics(L"插值结果", {{L"Q 型值点", std::to_wstring(count) + L" 个"},
                            {L"采样点", std::to_wstring(samples) + L" 个"},
                            {L"端点条件", L"自然端点"}});
    p.button(L"导出完整 CSV", [owner = &p.app, spline, samples, minimum, maximum, step] {
        owner->exportCsv(L"bspline_results.csv", "x,y\r\n", samples, [=](std::size_t i) {
            double x = sampleX(minimum, maximum, step, samples, i);
            if (i > 0) {
                require(x > sampleX(minimum, maximum, step, samples, i - 1), "步进小于当前数值的可分辨精度");
            }
            return rawNumber(x) + "," + rawNumber(spline->y(x)) + "\r\n";
        });
    });
    p.heading(L"采样结果预览 · 前 " + std::to_wstring(std::min<std::size_t>(100, samples)) + L" / " +
              std::to_wstring(samples) + L" 行");
    int pages = int((std::min<std::size_t>(100, samples) + 9) / 10);
    std::vector<std::wstring> pageNames;
    for (int i = 0; i < pages; ++i) {
        pageNames.push_back(L"第 " + std::to_wstring(i + 1) + L" 页");
    }
    int page = p.select("preview.page", L"预览分页", pageNames);
    std::vector<std::vector<std::wstring>> rows;
    for (std::size_t i = page * 10;
         i < std::min<std::size_t>(std::min<std::size_t>(100, samples), page * 10 + 10); ++i) {
        double x = sampleX(minimum, maximum, step, samples, i);
        rows.push_back({format(x), format(spline->y(x))});
    }
    p.table({L"x", L"y"}, rows);
    p.chart(curve, points, [spline](double x) { return spline->y(x); });
}
} // namespace toolbox
