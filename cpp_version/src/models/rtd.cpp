// 各分度号采用独立特性，CVD 自定义系数在反算前检查整段单调性。
#include "toolbox/models.hpp"
namespace toolbox {
static constexpr double ni672Upper[] = {-60, -30, 0, 30, 60, 90, 120, 150, 180, 210, 240, 260};
RtdModel::RtdModel(std::size_t index, double zero, const Values &custom)
    : definition(rtdDefinitions().at(index)), r0(zero),
      coefficients(custom.empty() ? definition.coefficients : custom) {
    require(std::isfinite(r0) && r0 > 0, "R₀ 必须大于 0");
    if (definition.curve == RtdCurve::Cvd) {
        require(coefficients.size() == 3, "CVD 需要 A、B、C 三个系数");
        finite(coefficients);
        double b = coefficients[1], c = coefficients[2];
        Values candidates{definition.minimum, 0, definition.maximum};
        if (c != 0) {
            double d = 360000 * c * c - 96 * b * c;
            finite({d});
            if (d >= 0) {
                for (double s : {-1., 1.}) {
                    double t = (600 * c + s * std::sqrt(d)) / (24 * c);
                    if (t > definition.minimum && t < 0) {
                        candidates.push_back(t);
                    }
                }
            }
        }
        for (double t : candidates) {
            auto v = ratio(t);
            require(std::isfinite(v.first) && v.first > 0 && std::isfinite(v.second) && v.second > 0,
                    "CVD 系数不能形成正值、单调递增曲线");
        }
    }
    resistance(definition.minimum);
    resistance(definition.maximum);
}
RtdReference RtdModel::reference(double t) const {
    switch (definition.curve) {
    case RtdCurve::Cvd: {
        double a = coefficients[0], b = coefficients[1], c = coefficients[2];
        return {t < 0 ? Values{1, a, b, -100 * c, c} : Values{1, a, b}, 0, 1};
    }
    case RtdCurve::Pt3916:
        return {t < 0 ? Values{1, 3.971686e-3, -1.157433e-6, -2.051844e-8, -3.629438e-10, -3.157615e-12, -1.369914e-14, -2.303654e-17} : Values{1, 3.974973e-3, -5.8973e-7}, 0, 1};
    case RtdCurve::Cu428:
        return {{1, .00428 + 100 * 9.31e-8, -9.31e-8 - 100 * 1.23e-9, 1.23e-9}, 0, 1};
    case RtdCurve::Cu427:
        if (t < -50) {
            return {{1, .0392313, -7.45044e-6}, 200, .117058};
        }
        if (t <= 150) {
            return {{1, .0042743}, 0, 1};
        }
        return {{1, .00262628, 2.43732e-8}, -150, 1.641145};
    case RtdCurve::Ni617:
        return {t < 100 ? Values{1, .0054963, 6.7556e-6} : Values{1, .0054963, 6.7556e-6 - 100 * 9.2004e-9, 9.2004e-9}, 0, 1};
    case RtdCurve::Ni618:
        return {{1, .005485, 6.65e-6, 0, 2.805e-11, 0, -2e-17}, 0, 1};
    case RtdCurve::Ni1000:
        return {{1, .004427, 5.172e-6, 5.585e-9}, 0, 1};
    case RtdCurve::Ni672: {
        static const Values c[] = {{.9980384367, .005779005438, 4.519218356e-6, 1.883007648e-8},
                                   {.9995545058, .005854808892, 5.782609262e-6, 2.584891485e-8},
                                   {1, .005899358312, 7.267589932e-6, 4.234870007e-8},
                                   {1, .005899358312, 7.267589932e-6, 1.154640832e-8},
                                   {1.000118847, .005887473643, 7.663745572e-6, 7.144678985e-9},
                                   {1.002329124, .005776959768, 9.50564349e-6, -3.088087226e-9},
                                   {.9940315172, .006053466667, 6.432455728e-6, 8.294089672e-9},
                                   {1.007022904, .005728761999, 9.138994624e-6, 7.7592607e-10},
                                   {.891859209, .008032035898, -6.216164699e-6, 3.489850234e-8},
                                   {.9060247382, .007795943744, -4.904541625e-6, 3.246957072e-8},
                                   {1.103473241, .004975250849, 8.527329303e-6, 1.114941068e-8},
                                   {1.437355995, .0008017164189, 2.59170561e-5, -1.300325764e-8}};
        for (int i = 0; i < 12; ++i) {
            if (t <= ni672Upper[i] || i == 11) {
                return {c[i], 0, 1};
            }
        }
    }
    }
    throw std::runtime_error("未知电阻曲线");
}
std::pair<double, double> RtdModel::ratio(double t) const {
    auto ref = reference(t);
    auto value = polynomialSlope(ref.coefficients, t + ref.shift);
    return {ref.scale * value.first, ref.scale * value.second};
}
double RtdModel::resistance(double t) const {
    require(std::isfinite(t) && t >= definition.minimum && t <= definition.maximum, "温度超出分度范围");
    double r = r0 * ratio(t).first;
    finite({r});
    require(r > 0, "电阻计算结果超出有效范围，请检查 R₀ 与系数");
    return r;
}
double RtdModel::temperature(double r) const {
    require(std::isfinite(r) && r > 0, "电阻必须大于 0 Ω");
    double lo = definition.minimum, hi = definition.maximum;
    double lowR = resistance(lo), highR = resistance(hi);
    require(r >= lowR && r <= highR, "电阻超出当前 R₀ 与分度号的有效范围");
    std::vector<std::pair<double, double>> intervals;
    if (definition.curve == RtdCurve::Cu427) {
        intervals = {{lo, std::nextafter(-50., lo)}, {-50., 150.}, {std::nextafter(150., hi), hi}};
    } else if (definition.curve == RtdCurve::Ni672) {
        double start = lo;
        for (double end : ni672Upper) {
            intervals.push_back({start, end});
            start = std::nextafter(end, std::numeric_limits<double>::infinity());
        }
    } else {
        intervals.push_back({lo, hi});
    }
    std::optional<double> solution;
    for (auto [start, end] : intervals) {
        double a = resistance(start), b = resistance(end);
        if (r < a || r > b) {
            continue;
        }
        double t = r == a ? start : end;
        if (r != a && r != b) {
            for (int i = 0; i < 80; ++i) {
                double mid = start + (end - start) * .5;
                (resistance(mid) < r ? start : end) = mid;
            }
            t = start + (end - start) * .5;
        }
        if (std::abs(resistance(t) - r) > 1e-9 * std::max(r, 1.)) {
            continue;
        }
        require(!solution || std::abs(*solution - t) <= 1e-9,
                "该电阻在分段曲线中对应多个温度，无法唯一反算");
        solution = t;
    }
    require(solution.has_value(), "该电阻位于分段曲线间隙，无法可靠反算温度");
    return *solution;
}
double RtdModel::slope(double t) const {
    resistance(t);
    double value = r0 * ratio(t).second;
    require(std::isfinite(value) && value > 0, "当前温度处的电阻变化率无效");
    return value;
}
std::vector<Tolerance> rtdTolerances(RtdTolerance kind, double t, bool film) {
    std::vector<Tolerance> out;
    if (kind == RtdTolerance::None) {
        return out;
    }
    if (kind == RtdTolerance::Copper) {
        out.push_back({L"铜热电阻允差", L"±(0.30 + 0.006|t|) °C", -50, 150,
                       t >= -50 && t <= 150 ? std::optional<double>(.3 + .006 * std::abs(t)) : std::nullopt});
        return out;
    }
    const wchar_t *names[] = {L"AA 级", L"A 级", L"B 级", L"C 级"};
    const wchar_t *formulas[] = {L"±(0.10 + 0.0017|t|) °C", L"±(0.15 + 0.002|t|) °C",
                                 L"±(0.30 + 0.005|t|) °C", L"±(0.60 + 0.01|t|) °C"};
    double a[] = {.1, .15, .3, .6}, b[] = {.0017, .002, .005, .01};
    double loWire[] = {-50, -100, -196, -196}, hiWire[] = {250, 450, 600, 600};
    double loFilm[] = {0, -30, -50, -50}, hiFilm[] = {150, 300, 500, 600};
    for (int i = 0; i < 4; ++i) {
        double lo = film ? loFilm[i] : loWire[i], hi = film ? hiFilm[i] : hiWire[i];
        out.push_back({names[i], formulas[i], lo, hi,
                       t >= lo && t <= hi ? std::optional<double>(a[i] + b[i] * std::abs(t)) : std::nullopt});
    }
    return out;
}
} // 命名空间 toolbox
