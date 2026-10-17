// 证书校准模型：固定点偏差、标准铂电阻及热敏电阻双向拟合。
#include "toolbox/models.hpp"
namespace toolbox {
std::vector<ThermoZone> standardZones(const std::string &kind) {
    if (kind == "S") {
        return {{-50,
                 1064.18,
                 {0, .00540313308631, .000012593428974, -2.32477968689e-8, 3.22028823036e-11,
                  -3.31465196389e-14, 2.55744251786e-17, -1.25068871393e-20, 2.71443176145e-24}},
                {1064.18,
                 1664.5,
                 {1.32900444085, .00334509311344, .00000654805192818, -1.64856259209e-9, 1.29989605174e-14}},
                {1664.5,
                 1768.1,
                 {146.628232636, -.258430516752, .000163693574641, -3.30439046987e-8, -9.43223690612e-15}}};
    }
    return {{0,
             630.615,
             {0, -.00024650818346, .0000059040421171, -1.3257931636e-9, 1.5668291901e-12, -1.694452924e-15,
              6.2990347094e-19}},
            {630.615,
             1820,
             {-3.8938168621, .02857174747, -.000084885104785, 1.5785280164e-7, -1.6835344864e-10,
              1.1109794013e-13, -4.4515431033e-17, 9.8975640821e-21, -9.3791330289e-25}}};
}
void StandardCouple::fit(ThermoZone &z, const std::vector<std::pair<double, double>> &points) {
    std::vector<Values> rows;
    Values rhs;
    for (auto [t, e] : points) {
        finite({t, e});
        require(t >= z.minimum && t <= z.maximum, "校准点超出所选温区");
        rows.push_back({t, t * t});
        rhs.push_back(e - polynomial(z.coefficients, t));
    }
    auto c = leastSquares(rows, rhs);
    z.coefficients.at(1) += c[0];
    z.coefficients.at(2) += c[1];
}
double StandardCouple::emf(double t) const {
    finite({t});
    if (zone) {
        if (t == 0) {
            return 0;
        }
        require(t >= zone->minimum && t <= zone->maximum, "温度超出所选系数温区");
        double e = polynomial(zone->coefficients, t);
        finite({e});
        return e;
    }
    Values delta;
    for (auto [p, e] : points) {
        finite({p, e});
        delta.push_back(e - thermoEmf(kind, p));
    }
    double correction = 0;
    if (kind == "S") {
        require(points.size() == 3, "S 型需要锌、铝、铜三个校准点");
        if (t < 400) {
            correction = delta[0] * t / points[0].first;
        } else {
            for (int i = 0; i < 3; ++i) {
                double term = delta[i];
                for (int j = 0; j < 3; ++j) {
                    if (i != j) {
                        term *= (t - points[j].first) / (points[i].first - points[j].first);
                    }
                }
                correction += term;
            }
        }
    } else {
        require(points.size() == 5, "B 型需要 1100～1500 °C 五个校准点");
        correction = delta.back();
        for (int i = 0; i < 5; ++i) {
            double p = points[i].first;
            if (t <= p + 30) {
                correction =
                    (i == 0 && t < p - 300) ? (t >= 23 ? delta[0] * (t - 23) / (p - 23) : 0) : delta[i];
                break;
            }
            if (t < p + 70) {
                correction = i == 4 ? delta[i] : delta[i] + (delta[i + 1] - delta[i]) * (t - p - 30) / 40;
                break;
            }
        }
    }
    double e = thermoEmf(kind, t) + correction;
    finite({e});
    return e;
}
double StandardCouple::temperature(double e) const {
    double lo = zone ? zone->minimum : (kind == "S" ? -50. : 50.);
    double hi = zone ? zone->maximum : (kind == "S" ? 1768.1 : 1820.);
    if (kind == "B") {
        lo = std::max(lo, 50.);
    }
    return inverse([&](double t) { return emf(t); }, e, lo, hi);
}
double sprtReference(double t) {
    require(std::isfinite(t) && t >= -259.3467 && t <= 961.78, "温度超出参考电阻比范围");
    return t >= 0 ? polynomial(sprtC(), (t - 481) / 481)
                  : std::exp(polynomial(sprtA(), (std::log((t + 273.15) / 273.16) + 1.5) / 1.5));
}
static double referenceTemperature(double wr) {
    // 逆参考多项式只是初值；用正参考函数校正，消除银点附近约 0.1 mK 的近似残差。
    bool high = wr >= sprtReference(0);
    double t = high ? polynomial(sprtD(), (wr - 2.64) / 1.64)
                    : 273.16 * polynomial(sprtB(), (std::pow(wr, 1. / 6) - .65) / .35) - 273.15;
    for (int i = 0; i < 6; ++i) {
        double value, slope;
        if (high) {
            auto p = polynomialSlope(sprtC(), (t - 481) / 481);
            value = p.first;
            slope = p.second / 481;
        } else {
            require(t > -273.15, "参考电阻比超出低温参考函数的有效范围");
            auto p = polynomialSlope(sprtA(), (std::log((t + 273.15) / 273.16) + 1.5) / 1.5);
            value = std::exp(p.first);
            slope = value * p.second / (1.5 * (t + 273.15));
        }
        require(std::isfinite(slope) && slope > 0, "参考电阻比无法可靠反算温度");
        double next = t - (value - wr) / slope;
        finite({next});
        if (next == t) {
            break;
        }
        t = next;
    }
    return t;
}
void Sprt::validate() const {
    finite({rtp, a, b, c, a4, b4, d, w660});
    require(rtp > 0, "水三相点电阻必须大于 0");
}
double Sprt::temperatureBranch(double r, bool high) const {
    validate();
    require(std::isfinite(r) && r > 0, "电阻必须大于 0 Ω");
    double w = r / rtp, x = w - 1, z = std::max(w - w660, 0.);
    double slope = high ? 1 - a - 2 * b * x - 3 * c * x * x - 2 * d * z
                        : (lowQuadratic ? 1 - a4 - 2 * b4 * x : 1 - a4 - b4 * (std::log(w) + x / w));
    require(std::isfinite(slope) && slope > 0, "偏差系数使电阻比曲线不再单调");
    double wr = high ? w - a * x - b * x * x - c * x * x * x - d * z * z
                     : w - a4 * x - b4 * (lowQuadratic ? x * x : x * std::log(w));
    require(wr > 0, "偏差系数使参考电阻比非正");
    double t = referenceTemperature(wr);
    finite({t});
    return t;
}
double Sprt::temperature(double r) const {
    return temperatureBranch(r, r >= rtp);
}
double Sprt::resistanceBranch(double t, bool high) const {
    validate();
    require(std::isfinite(t) && t >= -200 && t <= 961.78, "SPRT 换算范围为 −200～961.78 °C");
    return rtp * inverse([&](double w) { return temperatureBranch(w * rtp, high); }, t, high ? .99 : .01,
                         (high || (lowQuadratic && t >= 0)) ? 4.5 : 1.01);
}
double Sprt::resistance(double t) const {
    return resistanceBranch(t, t >= 0);
}
void Sprt::fit(const std::vector<std::pair<double, double>> &high,
               const std::vector<std::pair<double, double>> &low, int terms) {
    validate();
    require(terms >= 1 && terms <= 3, "高温拟合项数应为 1～3");
    std::vector<Values> rows;
    Values rhs;
    for (auto [t, r] : high) {
        finite({t, r});
        require(r > 0, "校准点电阻须大于 0");
        double w = r / rtp;
        Values row;
        for (int n = 1; n <= terms; ++n) {
            row.push_back(std::pow(w - 1, n));
        }
        rows.push_back(row);
        rhs.push_back(w - sprtReference(t));
    }
    if (!high.empty()) {
        auto v = leastSquares(rows, rhs);
        a = v[0];
        b = v.size() > 1 ? v[1] : 0;
        c = v.size() > 2 ? v[2] : 0;
    }
    rows.clear();
    rhs.clear();
    for (auto [t, r] : low) {
        finite({t, r});
        require(r > 0, "校准点电阻须大于 0");
        double w = r / rtp;
        rows.push_back({w - 1, lowQuadratic ? (w - 1) * (w - 1) : (w - 1) * std::log(w)});
        rhs.push_back(w - sprtReference(t));
    }
    if (!low.empty()) {
        auto v = leastSquares(rows, rhs);
        a4 = v[0];
        b4 = v[1];
    }
}
void Sprt::fitSilver(double t, double r) {
    double w = r / rtp, x = w - 1;
    require(w > w660, "银点电阻比须大于铝点电阻比");
    d = (w - sprtReference(t) - a * x - b * x * x - c * x * x * x) / ((w - w660) * (w - w660));
    finite({d});
}
BetaThermistor::BetaThermistor(double zero, double referenceC, double coefficient)
    : r0(zero), t0(referenceC + 273.15), beta(coefficient) {
    finite({r0, t0, beta});
    require(r0 > 0 && t0 > 0 && beta > 0, "R₀、参考绝对温度、β 必须大于 0");
}
double BetaThermistor::temperature(double r) const {
    require(std::isfinite(r) && r > 0, "电阻必须大于 0");
    double inv = 1 / t0 + (std::log(r) - std::log(r0)) / beta;
    require(inv > 0, "绝对温度必须大于 0 K");
    double t = 1 / inv - 273.15;
    finite({t});
    return t;
}
double BetaThermistor::resistance(double t) const {
    require(std::isfinite(t) && t > -273.15, "温度须高于绝对零度");
    double r = std::exp(std::log(r0) + beta * (1 / (t + 273.15) - 1 / t0));
    require(std::isfinite(r) && r > 0, "电阻超出可表示范围");
    return r;
}
double PolynomialThermistor::temperature(double r) const {
    require(std::isfinite(r) && r > 0, "电阻必须大于 0 Ω");
    double inv = polynomial(a, std::log(r));
    require(inv > 0, "绝对温度必须大于 0 K");
    double t = 1 / inv - 273.15;
    finite({t});
    return t;
}
double PolynomialThermistor::resistance(double t) const {
    require(std::isfinite(t) && t > -273.15, "温度须高于绝对零度");
    double r = std::exp(polynomial(b, 1 / (t + 273.15)));
    require(std::isfinite(r) && r > 0, "电阻超出可表示范围");
    return r;
}
double PolynomialThermistor::slope(double t) const {
    double r = resistance(t), kelvin = t + 273.15;
    // ln R = B(1/T)，链式法则 dR/dt = −R·B'(1/T)/T²；无需越过校准温区取差分点。
    double derivative = -r * polynomialSlope(b, 1 / kelvin).second / (kelvin * kelvin);
    finite({derivative});
    return derivative;
}
PolynomialThermistor PolynomialThermistor::fit(const std::vector<std::pair<double, double>> &points,
                                               int order, bool sh) {
    require(order >= 3 && order <= 6, "多项式阶数须为 3～6");
    std::vector<int> powers;
    for (int i = 0; i <= order; ++i) {
        if (!sh || i != 2) {
            powers.push_back(i);
        }
    }
    std::vector<Values> ra, rb;
    Values ya, yb;
    for (auto [t, r] : points) {
        finite({t, r});
        require(t > -273.15 && r > 0, "校准温度须高于绝对零度，电阻须大于 0");
        double x = std::log(r), y = 1 / (t + 273.15);
        Values a, b;
        for (int n : powers) {
            a.push_back(std::pow(x, n));
            b.push_back(std::pow(y, n));
        }
        ra.push_back(a);
        rb.push_back(b);
        ya.push_back(y);
        yb.push_back(x);
    }
    auto a = leastSquares(ra, ya), b = leastSquares(rb, yb);
    PolynomialThermistor model{Values(order + 1), Values(order + 1)};
    for (std::size_t i = 0; i < powers.size(); ++i) {
        model.a[powers[i]] = a[i];
        model.b[powers[i]] = b[i];
    }
    return model;
}
} // namespace toolbox
