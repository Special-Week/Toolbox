// 工业传感器页面，允差温区独立于分度曲线温区，不用边界温度替代超范围输入。
#include "sensors.hpp"
namespace toolbox {
void thermoReferenceProcess(Page &p, const std::string &kind, double t, const std::wstring &label) {
    const auto &segments = thermoSegments(kind);
    auto selected = std::find_if(segments.begin(), segments.end(), [t](const ThermoSegment &part) {
        return t >= part.minimum && t <= part.maximum;
    });
    require(selected != segments.end(), "温度超出所选分度号范围");
    const auto &segment = *selected;
    p.processSteps.push_back(label + L"：t = " + inputNumber(t) + L" °C，参考函数温区 " + inputNumber(segment.minimum) + L"～" + inputNumber(segment.maximum) + L" °C；E多项式(t) = Σcᵢtⁱ，系数按升幂：");
    for (std::size_t i = 0; i < segment.coefficients.size(); ++i)
        p.processSteps.push_back(L"c[" + std::to_wstring(i) + L"] = " + inputNumber(segment.coefficients[i]) + L"。");
    double poly = polynomial(segment.coefficients, t);
    p.processSteps.push_back(L"代入 t = " + inputNumber(t) + L" °C，多项式电势 = " + p.processValue(poly) + L" mV。");
    if (segment.gaussian) {
        auto g = *segment.gaussian;
        double term = g[0] * std::exp(g[1] * (t - g[2]) * (t - g[2]));
        p.processSteps.push_back(L"高斯修正 = a₀exp[a₁(t−a₂)²] = " + inputNumber(g[0]) + L" × exp[" + inputNumber(g[1]) + L" × (" + inputNumber(t) + L" − " + inputNumber(g[2]) + L")²] = " + p.processValue(term) + L" mV。");
    }
    p.processSteps.push_back(L"参考电势 E(" + inputNumber(t) + L") = " + p.processValue(segment.evaluate(t).first) + L" mV。");
}
static void toleranceCards(Page &p, const std::vector<Tolerance> &list, double t, TemperatureUnit unit,
                           const Curve &curve, const std::wstring &signal) {
    if (list.empty()) {
        p.note(L"此分度号未内置等级允差数据。");
        return;
    }
    for (auto &tolerance : list) {
        p.heading(tolerance.name);
        if (tolerance.delta) {
            double d = *tolerance.delta;
            p.note(L"±" + format(d * unit.span()) + L" " + unit.symbol(), Blue);
            p.note(L"温度界限：" + format(unit.display(t - d)) + L" ～ " + format(unit.display(t + d)) +
                   L" " + unit.symbol());
            try {
                double a = curve(t - d), b = curve(t + d);
                p.note(L"信号界限：" + format(a) + L" ～ " + format(b) + L" " + signal);
            } catch (const std::exception &) {
                p.note(L"温度允差边界超出分度曲线范围，无法换算信号界限。");
            }
        } else {
            p.note(L"当前温度不在此等级适用范围内");
        }
        p.note(tolerance.formula);
        p.note(L"适用范围：" + format(tolerance.minimum) + L" ～ " + format(tolerance.maximum) + L" °C");
    }
}
void renderSensors(Page &p) {
    if (p.tool.kind == Kind::StandardThermocouple || p.tool.kind == Kind::StandardPlatinumResistor)
        p.app.calibrationControls(p);
    switch (p.tool.kind) {
    case Kind::StandardThermocouple:
        standardCouplePage(p);
        return;
    case Kind::StandardPlatinumResistor:
        sprtPage(p);
        return;
    case Kind::Thermistor:
        thermistorPage(p);
        return;
    default:
        break;
    }
    if (p.tool.kind == Kind::WorkingThermocouple) {
        p.beginSplit(0.42f);
        p.heading(L"热电偶参数");
        const std::vector<std::string> codes{"B", "E", "J", "K", "N",  "R", "S", "T",
                                             "C", "D", "G", "L", "LR", "U", "A"};
        std::vector<std::wstring> names;
        for (auto &k : codes) {
            names.push_back(wide(k) + L" 型");
        }
        int selected = p.select("type", L"分度号", names, 3);
        auto kind = codes[selected];
        TemperatureUnit unit{p.select("unit", L"温度单位", {L"°C", L"K", L"°F"})};
        int mode = p.select("direction", L"计算方式", {L"温度 → 热电势", L"热电势 → 温度"});
        std::string uk = std::to_string(unit.index);
        double input = p.number(mode == 0 ? "temperature." + uk : "emf." + kind,
                                mode == 0 ? L"热端温度 (" + unit.symbol() + L")" : L"测得热电势 (mV)",
                                mode == 0 ? unit.display(100) : thermoEmf(kind, 100));
        auto bounds = thermoBounds(kind);
        double cold = unit.celsiusInBounds(
            p.number("cold." + uk, L"参考端温度 (" + unit.symbol() + L")", unit.display(0)), bounds);
        p.rightColumn();
        p.check();
        double ce = thermoEmf(kind, cold),
               roundoff = std::max(std::abs(input), std::abs(ce)) * (4 * std::numeric_limits<double>::epsilon()),
               t = mode == 0 ? unit.celsiusInBounds(input, bounds) : thermoTemperature(kind, input + ce, roundoff),
               emf = mode == 0 ? thermoEmf(kind, t) - ce : input;
        p.metrics(L"计算结果", {{L"热端温度", format(unit.display(t)) + L" " + unit.symbol()},
                                {L"测得热电势", format(emf) + L" mV"},
                                {L"补偿后热电势（参考端 0 °C）", format(emf + ce) + L" mV"},
                                {L"参考端电势", format(ce) + L" mV"},
                                {L"微分热电势 dE/dt",
                                 format(thermoSlope(kind, t) / unit.span()) + L" mV/" + unit.symbol()}});
        p.endSplit();
        p.processSteps.push_back(wide(kind) + L" 型参考函数；热端温度 " + p.processValue(t) + L" °C，参考端温度 " + p.processValue(cold) + L" °C。");
        thermoReferenceProcess(p, kind, t, L"热端");
        p.processSteps.push_back(L"");
        thermoReferenceProcess(p, kind, cold, L"参考端");
        p.processSteps.push_back(L"");
        double reconstructed = thermoEmf(kind, t) - ce;
        p.processSteps.push_back(std::wstring(mode == 0 ? L"测得热电势" : L"反算温度代回得到的热电势") + L" = E(热端) − E(参考端) = " + p.processValue(thermoEmf(kind, t)) + L" − " + p.processValue(ce) + L" = " + p.processValue(reconstructed) + L" mV。" );
        if (mode != 0)
            p.processSteps.push_back(L"反算时输入电势 = " + inputNumber(input) + L" mV，先加回参考端电势，在参考函数有效单调区间内二分求热端温度；正算残差 = 代回电势 − 输入电势 = " + p.processValue(reconstructed - input) + L" mV。");
        p.heading(L"等级允差 · IEC 60584-1");
        toleranceCards(
            p, thermoTolerances(kind, t), t, unit, [kind](double t) { return thermoEmf(kind, t); },
            L"mV（参考端 0 °C）");
        p.division(kind, thermoBounds(kind), unit, L"mV",
                   [kind, ce](double t) { return thermoEmf(kind, t) - ce; });
        return;
    }
    p.beginSplit(0.42f);
    p.heading(L"热电阻参数");
    std::vector<std::wstring> names;
    for (auto &d : rtdDefinitions()) {
        names.push_back(d.name);
    }
    int selected = p.select("graduation", L"分度号", names),
        mode = p.select("direction", L"计算方式", {L"温度 → 电阻", L"电阻 → 温度"}),
        u = p.select("unit", L"温度单位", {L"摄氏度 °C", L"华氏度 °F", L"开尔文 K"});
    TemperatureUnit unit{u == 1 ? 2 : (u == 2 ? 1 : 0)};
    auto d = rtdDefinitions()[selected];
    p.note(d.name + L" · 换算范围 " + format(unit.display(d.minimum)) + L"～" +
           format(unit.display(d.maximum)) + L" " + unit.symbol() + L" · 标称 R₀ = " + format(d.r0) + L" Ω");
    if (d.key == "cu10_427") {
        p.note(L"Cu10 的 10 Ω 为约 25 °C 时的标称值，0 °C 的 R₀ 为 9.035 Ω。");
    }
    double r0 = p.number(d.key + ".r0", L"R₀ (Ω)", d.r0);
    Values coefficients;
    if (d.key == "cvd") {
        coefficients = {p.number("cvd.a", L"系数 A (1/°C)", .0039083),
                        p.number("cvd.b", L"系数 B (1/°C²)", -5.775e-7),
                        p.number("cvd.c", L"系数 C (1/°C⁴)", -4.183e-12)};
    }
    p.check();
    RtdModel model(selected, r0, coefficients);
    double def = model.resistance(100);
    double input = p.number(mode == 0 ? "temperature." + std::to_string(u) : d.key + ".resistance",
                            mode == 0 ? L"温度 (" + unit.symbol() + L")" : L"电阻 (Ω)",
                            mode == 0 ? unit.display(100) : def);
    p.rightColumn();
    p.check();
    double t = mode == 0 ? unit.celsius(input) : model.temperature(input);
    for (double endpoint : {d.minimum, d.maximum}) {
        if (std::abs(t - endpoint) < 1e-10) {
            t = endpoint;
        }
    }
    double resistance = mode == 0 ? model.resistance(t) : input;
    p.metrics(L"换算结果",
              {{L"温度", format(unit.display(t)) + L" " + unit.symbol()},
               {L"电阻", format(resistance) + L" Ω"},
               {L"电阻比 R / R₀", format(resistance / r0)},
               {L"电阻变化率 dR/dt", format(model.slope(t) / unit.span()) + L" Ω/" + unit.symbol()}});
    p.endSplit();
    const auto ref = model.reference(t);
    const double variable = t + ref.shift;
    p.processSteps.push_back(L"分度号 " + d.name + L"；R₀ = " + inputNumber(r0) + L" Ω；当前温度 t = " + p.processValue(t) + L" °C，电阻 R = " + p.processValue(resistance) + L" Ω。");
    if (d.curve == RtdCurve::Cvd) {
        p.processSteps.push_back(L"CVD 系数 A = " + inputNumber(model.coefficients[0]) + L"，B = " + inputNumber(model.coefficients[1]) + L"，C = " + inputNumber(model.coefficients[2]) + L"；" + (t < 0 ? std::wstring(L"t < 0：R/R₀ = 1 + At + Bt² + C(t−100)t³。") : std::wstring(L"t ≥ 0：R/R₀ = 1 + At + Bt²。")));
    }
    p.processSteps.push_back(L"本段参考函数 R/R₀ = 倍率 × Σcᵢzⁱ；z = t + 偏移 = " + p.processValue(t) + L" + " + inputNumber(ref.shift) + L" = " + p.processValue(variable) + L"；倍率 = " + inputNumber(ref.scale) + L"，系数按升幂为：");
    for (std::size_t i = 0; i < ref.coefficients.size(); ++i)
        p.processSteps.push_back(L"c[" + std::to_wstring(i) + L"] = " + inputNumber(ref.coefficients[i]) + L"。");
    p.processSteps.push_back(L"电阻比 = " + p.processValue(model.ratio(t).first) + L"；曲线正算电阻 = R₀ × 电阻比 = " + inputNumber(r0) + L" × " + p.processValue(model.ratio(t).first) + L" = " + p.processValue(model.resistance(t)) + L" Ω。");
    if (mode != 0)
        p.processSteps.push_back(L"电阻反算时在所选分度范围内二分，并核对曲线残差。");
    p.processSteps.push_back(L"曲线解析变化率 dR/dt = " + p.processValue(model.slope(t) / unit.span()) + L" Ω/" + unit.symbol() + L"。");
    p.heading(L"等级允差");
    bool film = false;
    if (d.tolerance == RtdTolerance::Platinum || d.tolerance == RtdTolerance::PlatinumReference) {
        film = p.select("element", L"元件结构", {L"绕线式", L"薄膜式"}) == 1;
    }
    if (d.tolerance == RtdTolerance::PlatinumReference) {
        p.note(L"此分度号按 AA / A / B / C 公式给出参考允差；IEC 60751 标准特性为 385 系列。");
    }
    toleranceCards(
        p, rtdTolerances(d.tolerance, t, film), t, unit, [model](double t) { return model.resistance(t); },
        L"Ω");
    p.division(d.key, {d.minimum, d.maximum}, unit, L"Ω", [model](double t) { return model.resistance(t); });
}
void cvdPage(Page &p) {
    p.beginSplit(0.45f);
    p.heading(L"标准铂电阻系数拟合");
    p.note(L"输入 R₀ 与三个校准温度点的实测电阻，拟合 CVD 系数。至少一个点需低于 0 °C。");
    double r0 = p.number("cvd.r0", L"R₀ (Ω)", 100);
    auto points = p.pairs("cvd.point", L"点", 3, {{-100, 60.25584}, {100, 138.5055}, {200, 175.856}});
    p.rightColumn();
    p.check();
    std::array<std::pair<double, double>, 3> a{{points[0], points[1], points[2]}};
    auto c = fitCvd(r0, a);
    double residual = 0;
    for (auto [t, r] : points) {
        residual = std::max(residual, std::abs(cvd(t, r0, c[0], c[1], c[2]) - r));
    }
    p.metrics(L"拟合结果", {{L"系数 A", format(c[0])},
                            {L"系数 B", format(c[1])},
                            {L"系数 C", format(c[2])},
                            {L"最大残差", format(residual) + L" Ω"}});
    p.endSplit();
    p.processSteps.push_back(L"按 R/R₀ − 1 = At + Bt² + C(t−100)t³ 建立三元方程；t ≥ 0 时 C 项为零。采用列缩放和选主元消元求解。");
    for (std::size_t i = 0; i < points.size(); ++i) {
        auto point = points[i];
        p.processSteps.push_back(L"");
        p.processSteps.push_back(L"校准点 " + std::to_wstring(i + 1) + L"：t = " + inputNumber(point.first) + L" °C，R = " + inputNumber(point.second) + L" Ω；R/R₀ − 1 = " + inputNumber(point.second) + L" / " + inputNumber(r0) + L" − 1 = " + p.processValue(point.second / r0 - 1) + L"；拟合电阻 = " + p.processValue(cvd(point.first, r0, c[0], c[1], c[2])) + L" Ω。");
    }
    p.processSteps.push_back(L"解得 A = " + p.processValue(c[0]) + L"，B = " + p.processValue(c[1]) + L"，C = " + p.processValue(c[2]) + L"；最大绝对残差 = " + p.processValue(residual) + L" Ω。");
}
} // 命名空间 toolbox
