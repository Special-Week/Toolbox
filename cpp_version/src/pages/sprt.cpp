// ITS-90 分区与偏差项数严格关联；银点附加项只在铝点以上生效。
#include "sensors.hpp"
namespace toolbox {
void sprtPage(Page &p) {
    if (p.select("model", L"计算模型", {L"标准铂电阻（SPRT 换算）", L"CVD 系数拟合"}) == 1) {
        cvdPage(p);
        return;
    }
    int hr = p.select("hr", L"高温分段",
                      {L"A6 / B6 / C6：至 961.78 °C", L"A7 / B7 / C7：至 660.323 °C",
                       L"A8 / B8：至 419.527 °C", L"A9 / B9：至 231.928 °C", L"A10：至 156.599 °C",
                       L"A11：至 29.765 °C", L"不使用高温分区"},
                      1),
        lr = p.select("lr", L"低温分区",
                      {L"a4, b4（−189.344～0 °C）", L"a5, b5（−38.834～0 °C）", L"不使用低温分区"});
    require(hr != 6 || lr != 2, "高温和低温分区不能同时不使用");
    bool full =
        p.select("deviation", L"偏差函数", {L"ITS-90 子区间偏差函数", L"三项高温 / 对数低温偏差"}) == 0;
    int source = p.select("source", L"系数设置", {L"输入证书系数", L"固定点电阻拟合"});
    Sprt m;
    m.lowQuadratic = full && lr == 1;
    m.rtp = p.number("rtp", L"水三相点电阻 Rtp (Ω)", m.rtp);
    if (source == 0) {
        if (hr != 6) {
            m.a = p.number("coefficient.a", L"a", m.a);
            // 子区间 8/9 只用 a、b，10/11 只用 a；不能沿用默认 c 或 b。
            m.b = !full || hr <= 3 ? p.number("coefficient.b", L"b", m.b) : 0;
            m.c = !full || hr <= 1 ? p.number("coefficient.c", L"c", m.c) : 0;
        }
        if (lr != 2) {
            std::string k = lr == 0 ? "4" : "5";
            m.a4 = p.number("coefficient.a" + k, lr == 0 ? L"a4" : L"a5", 0);
            m.b4 = p.number("coefficient.b" + k, lr == 0 ? L"b4" : L"b5", 0);
        }
        if (full && hr == 0) {
            p.check();
            double w = m.resistance(660.323) / m.rtp;
            m.d = p.number("coefficient.d", L"d6", 0);
            m.w660 = p.number("coefficient.w660", L"W660", w);
            require(m.w660 > 1, "W660 须大于 1");
        }
    } else {
        const std::vector<Values> highPoints{{231.928, 419.527, 660.323},
                                             {231.928, 419.527, 660.323},
                                             {231.928, 419.527},
                                             {156.599, 231.928},
                                             {156.599},
                                             {29.765},
                                             {}};
        Values lowPoints =
            lr == 0 ? Values{-189.344, -38.834} : (lr == 1 ? Values{-38.834, 29.765} : Values{});
        std::vector<std::pair<double, double>> hp, lp;
        // 镓点可能同时用于高温和低温拟合。只创建一个输入控件，并让两分区共用实测值。
        std::map<double, double> fixed;
        auto readFixed = [&](double t) {
            auto found = fixed.find(t);
            if (found != fixed.end()) {
                p.note(format(t) + L" °C 固定点与高温分区共用上方实测电阻。");
                return found->second;
            }
            double r = p.number("fixed." + rawNumber(t), format(t) + L" °C · 实测电阻 (Ω)", m.resistance(t));
            fixed.emplace(t, r);
            return r;
        };
        p.check();
        for (auto t : highPoints[hr]) {
            hp.push_back({t, readFixed(t)});
        }
        for (auto t : lowPoints) {
            lp.push_back({t, readFixed(t)});
        }
        p.check();
        m.fit(hp, lp, hr <= 1 ? 3 : (hr <= 3 ? 2 : 1));
        if (full && hr == 0) {
            for (auto [t, r] : hp) {
                if (t == 660.323) {
                    m.w660 = r / m.rtp;
                }
            }
            double r = p.number("fixed.961.78", L"961.78 °C · 银点实测电阻 (Ω)", m.resistance(961.78));
            p.check();
            m.fitSilver(961.78, r);
            hp.push_back({961.78, r});
        }
        Metrics coefficients;
        if (hr != 6) {
            coefficients = {{L"a", format(m.a)}, {L"b", format(m.b)}, {L"c", format(m.c)}};
        }
        if (lr != 2) {
            coefficients.insert(coefficients.end(), {{lr == 0 ? L"a4" : L"a5", format(m.a4)},
                                                     {lr == 0 ? L"b4" : L"b5", format(m.b4)}});
        }
        if (full && hr == 0) {
            coefficients.insert(coefficients.end(), {{L"d6", format(m.d)}, {L"W660", format(m.w660)}});
        }
        p.metrics(L"拟合系数", coefficients);
        if (p.collapse("residuals", L"固定点拟合残差")) {
            for (auto [t, r] : hp) {
                p.note(L"高温 " + format(t) + L" °C：" + format(t - m.temperatureBranch(r, true)) +
                       L" °C（证书温度 − 反算温度）");
            }
            for (auto [t, r] : lp) {
                // 镓点即使高于 0 °C，也必须用其参与拟合的低温偏差函数核验。
                p.note(L"低温 " + format(t) + L" °C：" + format(t - m.temperatureBranch(r, false)) +
                       L" °C（证书温度 − 反算温度）");
            }
        }
    }
    p.check();
    m.validate();
    TemperatureUnit unit{p.select("unit", L"温度单位", {L"°C", L"K", L"°F"})};
    int direction = p.select("direction", L"计算方式", {L"温度 → 电阻", L"电阻 → 温度"});
    double dt = hr == 6 ? -20 : 20;
    std::string k = "range." + std::to_string(hr) + "." + std::to_string(lr);
    double input = p.number(direction == 0 ? k + ".t." + std::to_string(unit.index) : k + ".r",
                            direction == 0 ? L"温度 (" + unit.symbol() + L")" : L"电阻 (Ω)",
                            direction == 0 ? unit.display(dt) : m.resistance(dt));
    p.check();
    double t = direction == 0 ? unit.celsius(input)
                              : (hr == 6 ? m.temperatureBranch(input, false) : m.temperature(input)),
           r = direction == 0 ? m.resistanceBranch(t, hr != 6 && t >= 0) : input;
    const double lows[] = {-189.344, -38.834, 0},
                 highs[] = {961.78, 660.323, 419.527, 231.928, 156.599, 29.765, 0};
    double lo = lows[lr], hi = highs[hr];
    require(t >= lo - 1e-8 && t <= hi + 1e-8, "温度超出所选高低温分区");
    auto curve = [m, hr](double t) { return m.resistanceBranch(t, hr != 6 && t >= 0); };
    double a = std::max(t - 1, lo), b = std::min(t + 1, hi), slope = (curve(b) - curve(a)) / (b - a);
    p.metrics(L"换算结果", {{L"温度", format(unit.display(t)) + L" " + unit.symbol()},
                            {L"电阻", format(r) + L" Ω"},
                            {L"电阻比 W = R / Rtp", format(r / m.rtp)},
                            {L"电阻变化率 dR/dt", format(slope / unit.span()) + L" Ω/" + unit.symbol()},
                            {L"电阻比变化率 dW/dt", format(slope / m.rtp / unit.span())}});
    p.division(k, {lo, hi}, unit, L"Ω", curve);
}
} // namespace toolbox
