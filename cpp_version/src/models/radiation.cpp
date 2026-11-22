// 有效波长法与 Planck 积分分别实现，温差修约由界面按用途处理。
#include "toolbox/models.hpp"
namespace toolbox {
static constexpr double c2 = 6.62607015e-34 * 299792458 / 1.380649e-23 * 1e6;

static double logExpm1(double x) { return x > 50 ? x : std::log(std::expm1(x)); }

static double logSum(const Values &logs) {
    const double maximum = *std::max_element(logs.begin(), logs.end());
    if (!std::isfinite(maximum)) return maximum;
    double sum = 0;
    for (double value : logs) sum += std::exp(value - maximum);
    return maximum + std::log(sum);
}

static double kelvin(double t) {
    require(std::isfinite(t) && t > -273.15, "温度必须高于绝对零度");
    return t + 273.15;
}
static void emissivity(double e) { require(std::isfinite(e) && e > 0 && e <= 1, "发射率须满足 0 < ε ≤ 1"); }
double effectiveWavelength(int response, double wavelength, double t) {
    return response == 1 ? 9.6 + 221 / (t + 273.15) : (response == 2 ? 8.7 + 525 / (t + 273.15) : wavelength);
}
double radiationError(double e, double wavelength, double t, double bg) {
    finite({e, wavelength, t, bg});
    emissivity(e);
    double tk = kelvin(t), bk = kelvin(bg);
    require(wavelength > 0, "波长必须大于 0");
    require(bg <= 200, "有效波长法要求背景温度不高于 200 °C，请改用 Planck 模型");
    if (e == 1) {
        return 0;
    }
    double c = 14388 / wavelength;
    double occupation = 1 / std::expm1(c / tk) - (1 - e) / std::expm1(c / bk);
    require(occupation > 0, "当前参数无实数解，请检查背景温度和发射率");
    double delta = tk - c / std::log1p(e / occupation);
    finite({delta});
    return delta;
}
double radiationCorrection(double instrument, double source, double wavelength, double t, double bg) {
    return radiationError(instrument, wavelength, t, bg) - radiationError(source, wavelength, t, bg);
}
double earBlackbody(double t, double e, double probe, double pe, bool single) {
    emissivity(e);
    emissivity(pe);
    double b = kelvin(t), p = kelvin(probe), scale = std::max(b, p);
    double b4 = std::pow(b / scale, 4), p4 = std::pow(p / scale, 4), signal;
    if (single) {
        signal = e * b4 + (1 - e) * (pe * p4 + (1 - pe) * e * b4);
    } else {
        double denominator = e + (1 - e) * pe;
        signal = e / denominator * b4 + (1 - e) * pe / denominator * p4;
    }
    double light = scale * std::sqrt(std::sqrt(signal)) - 273.15;
    finite({light});
    return light;
}
Spectrum::Spectrum(double wavelength, double upper) {
    auto valid = [](double v) { return std::isfinite(v) && v >= .1 && v <= 100; };
    require(valid(wavelength) && (upper == 0 || (valid(upper) && upper > wavelength)),
            "波长须在 0.1～100 µm 内，波段上限须大于下限");
    if (upper == 0) {
        nodes_.push_back({wavelength, -5 * std::log(wavelength)});
    } else {
        double h = (upper - wavelength) / 256;
        for (int i = 0; i <= 256; ++i) {
            double w = wavelength + i * h, weight = (i == 0 || i == 256) ? 1 : (i % 2 == 0 ? 2 : 4);
            nodes_.push_back({w, std::log(weight * h / 3) - 5 * std::log(w)});
        }
    }
}
double Spectrum::logRadiance(double k) const {
    if (k <= 0) {
        return -std::numeric_limits<double>::infinity();
    }
    Values logs;
    for (auto [w, weight] : nodes_) {
        logs.push_back(weight - logExpm1((c2 / w) / k));
    }
    return logSum(logs);
}
double Spectrum::invert(double logarithm) const {
    finite({logarithm});
    double lo = 0, hi = 512;
    while (logRadiance(hi) < logarithm) {
        const double limit = std::numeric_limits<double>::max();
        require(hi < limit, "辐射温度超出双精度范围");
        hi = std::min(2 * hi, limit);
    }
    for (int i = 0; i < 85; ++i) {
        double mid = lo + (hi - lo) / 2;
        (logRadiance(mid) < logarithm ? lo : hi) = mid;
    }
    return lo + (hi - lo) / 2;
}
double Spectrum::logBalance(double t, double bg, double numerator, double denominator) const {
    const double tk = kelvin(t), bk = kelvin(bg);
    const double lt = logRadiance(tk), lb = logRadiance(bk);
    const double logWeight = std::log(numerator) - std::log(denominator);
    if (numerator < denominator) {
        const double complement = std::log(denominator - numerator) - std::log(denominator);
        return logSum({lt + logWeight, lb + complement});
    }
    if (numerator == denominator || t == bg) return lt;

    // (B热 − B冷) / B热 = [1 − exp(−Δx)] / [1 − exp(−x冷)]。
    const double hotter = std::max(tk, bk), colder = std::min(tk, bk);
    const double deltaT = std::abs(t - bg), logHot = std::max(lt, lb);
    Values logs;
    for (auto [w, weight] : nodes_) {
        const double c = c2 / w, xCold = c / colder, xHot = c / hotter;
        const double dx = xCold * (deltaT / hotter);
        const double logStep = std::isnormal(dx) ? std::log(-std::expm1(-dx))
            : std::log(xCold) + std::log(deltaT) - std::log(hotter);
        logs.push_back(weight - logExpm1(xHot) - logHot + logStep - std::log(-std::expm1(-xCold)));
    }
    const double logChange = logWeight + logSum(logs);
    if (t > bg) return logHot + logSum({lb - logHot, logChange});
    require(logChange < 0, "当前发射率、示值与背景不能得到有效辐射信号");
    const double remaining = logChange < -std::log(2.) ? std::log1p(-std::exp(logChange))
        : std::log(-std::expm1(logChange));
    return lb + remaining;
}
double Spectrum::transform(double t, double bg, double numerator, double denominator) const {
    if (t == bg || numerator == denominator) {
        kelvin(t);
        kelvin(bg);
        return t;
    }
    return invert(logBalance(t, bg, numerator, denominator)) - 273.15;
}
double Spectrum::apparent(double t, double bg, double e) const {
    emissivity(e);
    return transform(t, bg, e, 1.);
}
double Spectrum::corrected(double t, double bg, double instrument, double source) const {
    emissivity(instrument);
    emissivity(source);
    return transform(t, bg, instrument, source);
}
} // 命名空间 toolbox
