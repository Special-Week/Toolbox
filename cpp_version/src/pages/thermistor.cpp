// 多项式的 A/B 系数独立保存和拟合；Steinhart–Hart 将二次项固定为零。
#include "sensors.hpp"
namespace toolbox {
void thermistorPage(Page &p) {
    int family = p.select("family", L"计算模型", {L"多项式 / Steinhart–Hart", L"β 模型"});
    if (family == 1) {
        int mode = p.select("beta.direction", L"计算方式", {L"电阻 → 温度", L"温度 → 电阻"});
        double r0 = p.number("beta.r0", L"标称电阻 R₀ (Ω)", 10000),
               t0 = p.number("beta.t0", L"参考温度 T₀ (°C)", 25),
               beta = p.number("beta.value", L"β 值 (K)", 3950),
               input = p.number(mode == 0 ? "beta.r" : "beta.t",
                                mode == 0 ? L"实测电阻 (Ω)" : L"目标温度 (°C)", mode == 0 ? 10000 : 25);
        p.check();
        BetaThermistor model(r0, t0, beta);
        double t = mode == 0 ? model.temperature(input) : input,
               r = mode == 0 ? input : model.resistance(input);
        p.metrics(L"计算结果", {{L"温度", format(t) + L" °C"}, {L"电阻", format(r) + L" Ω"}});
        return;
    }
    bool sh =
        p.select("polynomial.form", L"多项式形式", {L"完整多项式", L"Steinhart–Hart（去除二次项）"}) == 1;
    int order = p.select("polynomial.order", L"多项式阶数", {L"3 阶", L"4 阶", L"5 阶", L"6 阶"}) + 3,
        source = p.select("polynomial.source", L"系数设置", {L"手动输入系数", L"校准点最小二乘拟合"});
    PolynomialThermistor model;
    std::vector<std::pair<double, double>> points;
    if (source == 0) {
        p.note(L"A：1/T = Σ A[i]·ln(R)^i；B：ln(R) = Σ B[i]·(1/T)^i。T 用 K，R 用 Ω。");
        p.note(L"示例为 10 kΩ、25 °C、β=3950 K；实际使用请填写证书系数。");
        for (int family = 0; family < 2; ++family) {
            bool open = p.collapse(family == 0 ? "a.open" : "b.open",
                                   family == 0 ? L"电阻 → 温度系数 A" : L"温度 → 电阻系数 B");
            float oldY = p.y;
            auto oldClip = p.ui.clip;
            if (!open) {
                p.ui.clip = {0, 0, 0, 0};
            }
            Values coefficients(order + 1);
            for (int i = 0; i <= order; ++i) {
                if (!sh || i != 2) {
                    double d = i == 0 ? (family == 0 ? 1 / 298.15 - std::log(10000.) / 3950
                                                     : std::log(10000.) - 3950 / 298.15)
                                      : (i == 1 ? (family == 0 ? 1 / 3950. : 3950) : 0);
                    std::string k = (family == 0 ? "a." : "b.") + std::to_string(order) + "." +
                                    std::to_string(sh) + "." + std::to_string(i);
                    coefficients[i] =
                        p.number(k, (family == 0 ? L"A[" : L"B[") + std::to_wstring(i) + L"]", d);
                }
            }
            if (!open) {
                p.y = oldY;
            }
            p.ui.clip = oldClip;
            (family == 0 ? model.a : model.b) = coefficients;
        }
    } else {
        int n = p.count("polynomial.points", L"校准点", 7, sh ? order : order + 1, 30);
        BetaThermistor beta(10000, 25, 3950);
        std::vector<std::pair<double, double>> defaults;
        for (int i = 0; i < n; ++i) {
            double t = -20 + i * 20.;
            defaults.push_back({t, beta.resistance(t)});
        }
        p.note(L"左列为校准温度 (°C)，右列为实测电阻 (Ω)。");
        points = p.pairs("fit.point", L"点", n, defaults);
        p.check();
        model = PolynomialThermistor::fit(points, order, sh);
        if (p.collapse("fit.details", L"拟合系数与校准点残差")) {
            for (std::size_t i = 0; i < model.a.size(); ++i) {
                p.note(L"A[" + std::to_wstring(i) + L"] = " + format(model.a[i]));
                p.note(L"B[" + std::to_wstring(i) + L"] = " + format(model.b[i]));
            }
            for (auto [t, r] : points) {
                p.note(format(t) + L" °C 温度残差：" + format(t - model.temperature(r)) + L" °C");
            }
        }
    }
    TemperatureUnit unit{p.select("unit", L"温度单位", {L"°C", L"K", L"°F"})};
    int direction = p.select("direction", L"计算方式", {L"电阻 → 温度", L"温度 → 电阻"});
    double input = p.number(direction == 0 ? "resistance" : "temperature." + std::to_string(unit.index),
                            direction == 0 ? L"电阻 (Ω)" : L"温度 (" + unit.symbol() + L")",
                            direction == 0 ? 10000 : unit.display(25));
    double lo = p.number("range.min", L"校准温区下限 (°C)", -20),
           hi = p.number("range.max", L"校准温区上限 (°C)", 100);
    p.check();
    double t = direction == 0 ? model.temperature(input) : unit.celsius(input),
           r = direction == 0 ? input : model.resistance(t);
    require(lo > -273.15 && lo < hi && t >= lo && t <= hi,
            "校准温区须高于绝对零度、下限小于上限，计算温度须在温区内");
    double slope = model.slope(t);
    p.metrics(L"换算结果", {{L"温度", format(unit.display(t)) + L" " + unit.symbol()},
                            {L"电阻", format(r) + L" Ω"},
                            {L"电阻变化率", format(slope / unit.span()) + L" Ω/" + unit.symbol()}});
    p.division("ntc", {lo, hi}, unit, L"Ω", [model](double t) { return model.resistance(t); });
}
} // namespace toolbox
