// 多项式的 A/B 系数独立保存和拟合；Steinhart–Hart 将二次项固定为零。
#include "sensors.hpp"
namespace toolbox {
void thermistorPage(Page &p) {
    p.beginSplit(0.46f);
    p.heading(L"热敏电阻参数");
    int family = p.select("family", L"计算模型", {L"多项式 / Steinhart–Hart", L"β 模型"});
    if (family == 1) {
        int mode = p.select("beta.direction", L"计算方式", {L"电阻 → 温度", L"温度 → 电阻"});
        double r0 = p.number("beta.r0", L"标称电阻 R₀ (Ω)", 10000),
               t0 = p.number("beta.t0", L"参考温度 T₀ (°C)", 25),
               beta = p.number("beta.value", L"β 值 (K)", 3950),
               input = p.number(mode == 0 ? "beta.r" : "beta.t",
                                mode == 0 ? L"实测电阻 (Ω)" : L"目标温度 (°C)", mode == 0 ? 10000 : 25);
        p.rightColumn();
        p.check();
        BetaThermistor model(r0, t0, beta);
        double t = mode == 0 ? model.temperature(input) : input,
               r = mode == 0 ? input : model.resistance(input);
        p.metrics(L"计算结果", {{L"温度", format(t) + L" °C"}, {L"电阻", format(r) + L" Ω"}});
        p.endSplit();
        double t0k = t0 + 273.15;
        if (mode == 0) {
            double inverse = 1 / t0k + (std::log(r) - std::log(r0)) / beta;
            p.processSteps.push_back(L"β 模型：T₀ = t₀ + 273.15 = " + inputNumber(t0) + L" + 273.15 = " + p.processValue(t0k) + L" K；R₀ = " + inputNumber(r0) + L" Ω，β = " + inputNumber(beta) + L" K。");
            p.processSteps.push_back(L"1/T = 1/T₀ + [ln(R) − ln(R₀)]/β = 1/" + p.processValue(t0k) + L" + [ln(" + inputNumber(r) + L") − ln(" + inputNumber(r0) + L")]/" + inputNumber(beta) + L" = " + p.processValue(inverse) + L" K⁻¹。");
            p.processSteps.push_back(L"温度 t = 1/(1/T) − 273.15 = 1/" + p.processValue(inverse) + L" − 273.15 = " + p.processValue(t) + L" °C。");
        } else {
            double logR = std::log(r0) + beta * (1 / (t + 273.15) - 1 / t0k);
            p.processSteps.push_back(L"β 模型：T = t + 273.15 = " + inputNumber(t) + L" + 273.15 = " + p.processValue(t + 273.15) + L" K；T₀ = " + p.processValue(t0k) + L" K。");
            p.processSteps.push_back(L"ln(R) = ln(R₀) + β(1/T − 1/T₀) = ln(" + inputNumber(r0) + L") + " + inputNumber(beta) + L" × (1/" + p.processValue(t + 273.15) + L" − 1/" + p.processValue(t0k) + L") = " + p.processValue(logR) + L"；R = exp(ln R) = " + p.processValue(r) + L" Ω。");
        }
        return;
    }
    bool sh =
        p.select("polynomial.form", L"多项式形式", {L"完整多项式", L"Steinhart–Hart（去除二次项）"}) == 1;
    int order = p.select("polynomial.order", L"多项式阶数", {L"3 阶", L"4 阶", L"5 阶", L"6 阶"}) + 3,
        source = p.select("polynomial.source", L"系数设置", {L"手动输入系数", L"校准点最小二乘拟合"});
    PolynomialThermistor model;
    std::vector<std::pair<double, double>> points;
    if (source == 0) {
        p.note(L"请填写实际证书系数；温度用 K，电阻用 Ω。");
        p.details("coefficient.info", L"系数与公式说明",
                  {L"A：1/T = Σ A[i]·ln(R)^i；B：ln(R) = Σ B[i]·(1/T)^i。T 用 K，R 用 Ω。",
                   L"示例为 10 kΩ、25 °C、β=3950 K；实际使用请填写证书系数。"});
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
    p.rightColumn();
    p.check();
    double t = direction == 0 ? model.temperatureInBounds(input, lo, hi) : unit.celsiusInBounds(input, {lo, hi}),
           r = direction == 0 ? input : model.resistance(t);
    require(lo > -273.15 && lo < hi && t >= lo && t <= hi,
            "校准温区须高于绝对零度、下限小于上限，计算温度须在温区内");
    double slope = model.slope(t);
    p.metrics(L"换算结果", {{L"温度", format(unit.display(t)) + L" " + unit.symbol()},
                            {L"电阻", format(r) + L" Ω"},
                            {L"电阻变化率", format(slope / unit.span()) + L" Ω/" + unit.symbol()}});
    p.endSplit();
    p.processSteps.push_back(L"热敏电阻 " + std::to_wstring(order) + L" 阶" + (sh ? L" Steinhart–Hart（二次项固定为零）" : L"完整多项式") + L"；温度使用 K、电阻使用 Ω。");
    if (source == 1)
        p.processSteps.push_back(L"校准点分别拟合 A：x=ln(R)、y=1/T 和 B：x=1/T、y=ln(R)；分别用列缩放 QR 求最小二乘解。");
    const auto &used = direction == 0 ? model.a : model.b;
    double variable = direction == 0 ? std::log(r) : 1 / (t + 273.15);
    p.processSteps.push_back(std::wstring(direction == 0 ? L"电阻 → 温度：1/T = ΣAᵢ[ln(R)]ⁱ；ln(R) = " : L"温度 → 电阻：ln(R) = ΣBᵢ(1/T)ⁱ；1/T = ") + p.processValue(variable) + L"；系数按升幂为：");
    for (std::size_t i = 0; i < used.size(); ++i)
        p.processSteps.push_back(std::wstring(direction == 0 ? L"A[" : L"B[") + std::to_wstring(i) + L"] = " + inputNumber(used[i]) + L"。");
    double evaluated = polynomial(used, variable);
    if (direction == 0)
        p.processSteps.push_back(L"多项式结果 1/T = " + p.processValue(evaluated) + L" K⁻¹；温度 = 1/(1/T) − 273.15 = " + p.processValue(t) + L" °C。");
    else
        p.processSteps.push_back(L"多项式结果 ln(R) = " + p.processValue(evaluated) + L"；R = exp(ln R) = " + p.processValue(r) + L" Ω。");
    if (direction == 0) {
        p.processSteps.push_back(L"变化率使用温度 → 电阻的 B 系数；A、B 独立拟合或输入，可能不是精确的互逆函数。");
        for (std::size_t i = 0; i < model.b.size(); ++i)
            p.processSteps.push_back(L"B[" + std::to_wstring(i) + L"] = " + inputNumber(model.b[i]) + L"。");
    }
    const double kelvin = t + 273.15;
    const double bResistance = model.resistance(t);
    const double bDerivative = polynomialSlope(model.b, 1 / kelvin).second;
    p.processSteps.push_back(L"令 x = 1/T，B′(x) = Σ i·B[i]·x^(i−1)；T = " + p.processValue(kelvin) + L" K，x = " + p.processValue(1 / kelvin) + L" K⁻¹，B′(x) = " + p.processValue(bDerivative) + L"，B 曲线电阻 R_B = " + p.processValue(bResistance) + L" Ω。");
    p.processSteps.push_back(L"链式法则：dR/dt = −R_B × B′(1/T) / T² = −" + p.processValue(bResistance) + L" × " + p.processValue(bDerivative) + L" / " + p.processValue(kelvin) + L"² = " + p.processValue(slope) + L" Ω/°C；按显示温标换算后 = " + p.processValue(slope / unit.span()) + L" Ω/" + unit.symbol() + L"。");
    p.division(
        "ntc", {lo, hi}, unit, L"Ω", [model](double t) { return model.resistance(t); },
        [model](double t) -> std::optional<double> { return model.slope(t); });
}
} // 命名空间 toolbox
