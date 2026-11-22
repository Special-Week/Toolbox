// 标准热电偶固定点实测电势和温区系数两套入口，统一使用同一模型作正反算。
#include "sensors.hpp"
namespace toolbox {
static void standardReferenceProcess(Page &p, const StandardCouple &model, double t, const std::wstring &label) {
    if (model.zone) {
        if (t == 0) {
            p.processSteps.push_back(label + L"：0 °C 参考电势按模型固定为 0 mV。");
            return;
        }
        p.processSteps.push_back(label + L"：t = " + inputNumber(t) + L" °C，证书温区 " + inputNumber(model.zone->minimum) + L"～" + inputNumber(model.zone->maximum) + L" °C；E(t) = Σcᵢtⁱ，系数按升幂：");
        for (std::size_t i = 0; i < model.zone->coefficients.size(); ++i)
            p.processSteps.push_back(L"c[" + std::to_wstring(i) + L"] = " + inputNumber(model.zone->coefficients[i]) + L"。");
        p.processSteps.push_back(L"代入 t = " + inputNumber(t) + L" °C，E(t) = " + p.processValue(model.emf(t)) + L" mV。");
        return;
    }
    thermoReferenceProcess(p, model.kind, t, label + L"参考曲线");
    Values offsets;
    for (std::size_t i = 0; i < model.points.size(); ++i) {
        double point = model.points[i].first, measured = model.points[i].second, ref = thermoEmf(model.kind, point);
        offsets.push_back(measured - ref);
        p.processSteps.push_back(L"固定点 " + std::to_wstring(i + 1) + L"：偏差 δ = 实测 − 参考 = " + inputNumber(measured) + L" − " + p.processValue(ref) + L" = " + p.processValue(offsets.back()) + L" mV。");
    }
    double correction = 0;
    if (model.kind == "S") {
        if (t < 400) {
            correction = offsets[0] * t / model.points[0].first;
            p.processSteps.push_back(L"S 型 t < 400 °C：偏差线性延伸 δ(t) = δ₀ × t/t₀ = " + p.processValue(offsets[0]) + L" × " + inputNumber(t) + L" / " + inputNumber(model.points[0].first) + L" = " + p.processValue(correction) + L" mV。");
        } else {
            p.processSteps.push_back(L"S 型 t ≥ 400 °C：固定点偏差按拉格朗日二次插值，δ(t) = Σδᵢ ∏(t−tⱼ)/(tᵢ−tⱼ)。");
            for (int i = 0; i < 3; ++i) {
                double term = offsets[i];
                std::wstring substitution = p.processValue(offsets[i]);
                for (int j = 0; j < 3; ++j)
                    if (i != j) {
                        term *= (t - model.points[j].first) / (model.points[i].first - model.points[j].first);
                        substitution += L" × (" + inputNumber(t) + L" − " + inputNumber(model.points[j].first) + L") / (" + inputNumber(model.points[i].first) + L" − " + inputNumber(model.points[j].first) + L")";
                    }
                correction += term;
                p.processSteps.push_back(L"第 " + std::to_wstring(i + 1) + L" 项 = " + substitution + L" = " + p.processValue(term) + L" mV。");
            }
        }
    } else {
        correction = offsets.back();
        bool branchReported = false;
        for (std::size_t i = 0; i < model.points.size(); ++i) {
            double point = model.points[i].first;
            if (t <= point + 30) {
                if (i == 0 && t < point - 300) {
                    correction = t >= 23 ? offsets[0] * (t - 23) / (point - 23) : 0;
                    p.processSteps.push_back(t >= 23 ? L"B 型低温线性段：δ(t) = δ₀(t−23)/(t₀−23) = " + p.processValue(offsets[0]) + L" × (" + inputNumber(t) + L" − 23)/(" + inputNumber(point) + L" − 23) = " + p.processValue(correction) + L" mV。" : L"B 型 t < 23 °C：偏差为零。");
                } else {
                    correction = offsets[i];
                    p.processSteps.push_back(L"B 型固定点邻域：采用第 " + std::to_wstring(i + 1) + L" 点偏差。");
                }
                branchReported = true;
                break;
            }
            if (t < point + 70) {
                correction = i == 4 ? offsets[i] : offsets[i] + (offsets[i + 1] - offsets[i]) * (t - point - 30) / 40;
                p.processSteps.push_back(i == 4 ? L"B 型末点邻域：采用末点偏差。" : L"B 型固定点 " + std::to_wstring(i + 1) + L" 与 " + std::to_wstring(i + 2) + L" 之间的过渡段：δ(t) = δᵢ + (δᵢ₊₁−δᵢ)(t−tᵢ−30)/40 = " + p.processValue(offsets[i]) + L" + (" + p.processValue(offsets[i + 1]) + L" − " + p.processValue(offsets[i]) + L") × (" + inputNumber(t) + L" − " + inputNumber(point) + L" − 30)/40 = " + p.processValue(correction) + L" mV。");
                branchReported = true;
                break;
            }
        }
        if (!branchReported)
            p.processSteps.push_back(L"B 型高于末点邻域：采用末点偏差 " + p.processValue(correction) + L" mV。");
    }
    p.processSteps.push_back(L"校准偏差 δ(t) = " + p.processValue(correction) + L" mV；校准电势 = 参考电势 + 偏差 = " + p.processValue(thermoEmf(model.kind, t)) + L" + " + p.processValue(correction) + L" = " + p.processValue(model.emf(t)) + L" mV。");
}
void standardCouplePage(Page &p) {
    p.beginSplit(0.46f);
    p.heading(L"校准与换算参数");
    const auto calibrationStart = p.ui.controls.size();
    int type = p.select("type", L"分度号", {L"S 型", L"B 型"}),
        mode = p.select("calibration", L"校准方式", {L"固定点实测热电势", L"温区多项式系数"});
    StandardCouple model;
    model.kind = type == 0 ? "S" : "B";
    int zoneId = 0;
    if (mode == 0) {
        std::vector<std::pair<double, double>> defaults =
            type == 0 ? std::vector<std::pair<double, double>>{{419.527, 3.4472},
                                                               {660.323, 5.8605},
                                                               {1084.62, 10.5822}}
                      : std::vector<std::pair<double, double>>{
                            {1100, 5.8029}, {1200, 6.8050}, {1300, 7.8809}, {1400, 8.9926}, {1500, 10.1205}};
        p.note(L"证书中的实际电势，不是修正量。");
        for (std::size_t i = 0; i < defaults.size(); ++i) {
            auto [t, e] = defaults[i];
            model.points.push_back({t, p.number(model.kind + ".cal." + std::to_string(i),
                                                format(t) + L" °C · 实测电势 (mV)", e)});
        }
    } else {
        auto zones = standardZones(model.kind);
        std::vector<std::wstring> names;
        for (auto &z : zones) {
            names.push_back(format(z.minimum) + L" ～ " + format(z.maximum) + L" °C");
        }
        zoneId = p.select(model.kind + ".zone", L"系数温区", names);
        auto zone = zones[zoneId];
        std::string k = model.kind + "." + std::to_string(zoneId);
        bool fit = p.select("source", L"系数设置", {L"手动输入系数", L"校准点拟合"}) == 1;
        if (fit) {
            int n = p.count("fit.count", L"校准点", 3, 2, 30);
            std::vector<std::pair<double, double>> defaults;
            for (int i = 0; i < n; ++i) {
                double t = zone.minimum + (zone.maximum - zone.minimum) * (i + 1) / (n + 1);
                defaults.push_back({t, polynomial(zone.coefficients, t)});
            }
            p.note(L"左列为温度 (°C)，右列为实测电势 (mV)。");
            auto points = p.pairs("fit." + k, L"点", n, defaults);
            p.check();
            StandardCouple::fit(zone, points);
            if (p.collapse("fit.details", L"拟合系数与残差")) {
                for (std::size_t i = 0; i < zone.coefficients.size(); ++i) {
                    p.note(L"c[" + std::to_wstring(i) + L"] = " + format(zone.coefficients[i]));
                }
                for (auto [t, e] : points) {
                    p.note(format(t) + L" °C 电势残差：" + format(e - polynomial(zone.coefficients, t)) +
                           L" mV");
                }
            }
        } else {
            for (std::size_t i = 0; i < zone.coefficients.size(); ++i) {
                zone.coefficients[i] = p.number(k + ".c." + std::to_string(i),
                                                L"系数 c[" + std::to_wstring(i) + L"]", zone.coefficients[i]);
            }
        }
        model.zone = zone;
    }
    p.check();
    p.app.captureCalibration(p, calibrationStart);
    TemperatureUnit unit{p.select("unit", L"温度单位", {L"°C", L"K", L"°F"})};
    int direction = p.select("direction", L"计算方式", {L"温度 → 热电势", L"热电势 → 温度"});
    p.check();
    double dt = model.zone ? (model.zone->minimum + model.zone->maximum) / 2 : (type == 0 ? 1000 : 1200);
    std::string k = model.kind + "." + std::to_string(mode) + "." + std::to_string(zoneId);
    double input = p.number(direction == 0 ? k + ".t." + std::to_string(unit.index) : k + ".e",
                            direction == 0 ? L"热端温度 (" + unit.symbol() + L")" : L"测得热电势 (mV)",
                            direction == 0 ? unit.display(dt) : model.emf(dt));
    auto bounds = model.bounds();
    double cold = unit.celsiusInBounds(p.number("cold." + std::to_string(unit.index),
                                                L"参考端温度 (" + unit.symbol() + L")", unit.display(0)),
                                       bounds);
    p.rightColumn();
    p.check();
    double ce = model.referenceEndEmf(cold),
           t = direction == 0 ? unit.celsiusInBounds(input, bounds) : model.temperature(input + ce),
           e = direction == 0 ? model.emf(t) - ce : input;
    auto slope = model.slope(t);
    p.metrics(L"换算结果", {{L"热端温度", format(unit.display(t)) + L" " + unit.symbol()},
                            {L"测得热电势", format(e) + L" mV"},
                            {L"微分热电势 dE/dt", slope ? format((*slope)[2] / unit.span()) + L" mV/" + unit.symbol() : L"未定义"},
                            {L"参考端电势", format(ce) + L" mV"}});
    p.endSplit();
    standardReferenceProcess(p, model, t, L"热端");
    p.processSteps.push_back(L"");
    standardReferenceProcess(p, model, cold, L"参考端");
    p.processSteps.push_back(L"");
    double reconstructed = model.emf(t) - ce;
    p.processSteps.push_back(std::wstring(direction == 0 ? L"测得热电势" : L"反算温度代回得到的热电势") + L" = E(热端) − E(参考端) = " + p.processValue(model.emf(t)) + L" − " + p.processValue(ce) + L" = " + p.processValue(reconstructed) + L" mV。" );
    if (direction != 0)
        p.processSteps.push_back(L"反算时输入电势 = " + inputNumber(input) + L" mV，先加回参考端电势，再" + (model.zone ? L"在所选有效温区内二分" : L"对固定点曲线分段二分") + L"；正算残差 = 代回电势 − 输入电势 = " + p.processValue(reconstructed - input) + L" mV。");
    if (slope) {
        auto [lo, hi, value] = *slope;
        p.processSteps.push_back(L"在同一分段的 " + p.processValue(lo) + L"～" + p.processValue(hi) + L" °C 内作差分，得到微分热电势 " + p.processValue(value / unit.span()) + L" mV/" + unit.symbol() + L"。");
    } else {
        p.processSteps.push_back(L"分段交界处的微分热电势未定义。");
    }
    p.division(k, model.bounds(), unit, L"mV", [model, ce](double t) { return model.emf(t) - ce; },
               [model](double t) -> std::optional<double> {
                   auto slope = model.slope(t);
                   return slope ? std::optional<double>((*slope)[2]) : std::nullopt;
               });
}
} // 命名空间 toolbox
