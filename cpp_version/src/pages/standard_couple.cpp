// 标准热电偶固定点实测电势和温区系数两套入口，统一使用同一模型作正反算。
#include "sensors.hpp"
namespace toolbox {
void standardCouplePage(Page &p) {
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
    TemperatureUnit unit{p.select("unit", L"温度单位", {L"°C", L"K", L"°F"})};
    int direction = p.select("direction", L"计算方式", {L"温度 → 热电势", L"热电势 → 温度"});
    p.check();
    double dt = model.zone ? (model.zone->minimum + model.zone->maximum) / 2 : (type == 0 ? 1000 : 1200);
    std::string k = model.kind + "." + std::to_string(mode) + "." + std::to_string(zoneId);
    double input = p.number(direction == 0 ? k + ".t." + std::to_string(unit.index) : k + ".e",
                            direction == 0 ? L"热端温度 (" + unit.symbol() + L")" : L"测得热电势 (mV)",
                            direction == 0 ? unit.display(dt) : model.emf(dt));
    double cold = unit.celsius(p.number("cold." + std::to_string(unit.index),
                                        L"参考端温度 (" + unit.symbol() + L")", unit.display(0)));
    p.check();
    double ce = model.emf(cold), t = direction == 0 ? unit.celsius(input) : model.temperature(input + ce),
           e = direction == 0 ? model.emf(t) - ce : input;
    double lo = model.zone ? model.zone->minimum : (type == 0 ? -50 : 0),
           hi = model.zone ? model.zone->maximum : (type == 0 ? 1768.1 : 1820), a = std::max(t - .01, lo),
           b = std::min(t + .01, hi), slope = (model.emf(b) - model.emf(a)) / (b - a);
    p.metrics(L"换算结果", {{L"热端温度", format(unit.display(t)) + L" " + unit.symbol()},
                            {L"测得热电势", format(e) + L" mV"},
                            {L"微分热电势 dE/dt", format(slope / unit.span()) + L" mV/" + unit.symbol()},
                            {L"参考端电势", format(ce) + L" mV"}});
    p.division(k, {lo, hi}, unit, L"mV", [model, ce](double t) { return model.emf(t) - ce; });
}
} // namespace toolbox
