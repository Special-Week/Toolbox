// CVD 校准拟合、标准大气分层与坐标刻度；各模型在输入边界统一单位。
#include "toolbox/models.hpp"
#include <iomanip>
#include <sstream>
namespace toolbox {
double cvd(double t, double r0, double a, double b, double c) {
    return r0 * (1 + a * t + b * t * t + (t < 0 ? c * (t - 100) * std::pow(t, 3) : 0));
}
std::array<double, 3> fitCvd(double r0, const std::array<std::pair<double, double>, 3> &points) {
    require(r0 > 0, "R₀ 必须大于 0");
    bool negative = false;
    std::vector<Values> rows;
    Values target;
    for (auto p : points) {
        finite({p.first, p.second});
        require(p.first >= -200 && p.first <= 850 && p.second > 0,
                "校准点温度须在 −200～850 °C 内，电阻必须大于 0");
        negative |= p.first < 0;
        rows.push_back(
            {p.first, p.first * p.first, p.first < 0 ? (p.first - 100) * std::pow(p.first, 3) : 0});
        target.push_back(p.second / r0 - 1);
    }
    require(negative, "三个温度点中至少要有一个低于 0 °C");
    // 三点拟合使用与 Rust 相同的列缩放、选主元消元。
    double scales[3] = {}, a[3][4] = {};
    for (int j = 0; j < 3; ++j) {
        for (int i = 0; i < 3; ++i) {
            scales[j] = std::max(scales[j], std::abs(rows[i][j]));
        }
        require(scales[j] != 0, "温度点组合无法确定三个系数");
    }
    for (int i = 0; i < 3; ++i) {
        for (int j = 0; j < 3; ++j) {
            a[i][j] = rows[i][j] / scales[j];
        }
        a[i][3] = target[i];
    }
    for (int col = 0; col < 3; ++col) {
        int pivot = col;
        for (int i = col + 1; i < 3; ++i) {
            if (std::abs(a[i][col]) >= std::abs(a[pivot][col])) {
                pivot = i;
            }
        }
        require(std::abs(a[pivot][col]) >= 1e-12, "温度点组合无法确定三个系数");
        for (int j = 0; j < 4; ++j) {
            std::swap(a[col][j], a[pivot][j]);
        }
        double divisor = a[col][col];
        for (int j = col; j < 4; ++j) {
            a[col][j] /= divisor;
        }
        for (int i = 0; i < 3; ++i) {
            if (i != col) {
                double factor = a[i][col];
                for (int j = col; j < 4; ++j) {
                    a[i][j] -= factor * a[col][j];
                }
            }
        }
    }
    std::array<double, 3> result{{a[0][3] / scales[0], a[1][3] / scales[1], a[2][3] / scales[2]}};
    finite({result[0], result[1], result[2]});
    return result;
}
double atmosphereTemperature(double h) {
    static const double layers[][3] = {{0, 288.15, -.0065},    {11000, 216.65, 0}, {20000, 216.65, .001},
                                       {32000, 228.65, .0028}, {47000, 270.65, 0}, {51000, 270.65, -.0028},
                                       {71000, 214.65, -.002}};
    int index = 0;
    for (int i = 0; i < 7; ++i) {
        if (h >= layers[i][0]) {
            index = i;
        }
    }
    return layers[index][1] + layers[index][2] * (h - layers[index][0]);
}
double atmospherePressure(double h) {
    require(h >= -1000 && h <= 86000, "位势高度应在 −1000 至 86 000 m 之间");
    double p = 101325, base = 0;
    static const double layers[][2] = {{11000, -.0065}, {20000, 0},      {32000, .001}, {47000, .0028},
                                       {51000, 0},      {71000, -.0028}, {86000, -.002}};
    for (auto &layer : layers) {
        double end = std::min(h, layer[0]), t = atmosphereTemperature(base), lapse = layer[1];
        p *= lapse == 0 ? std::exp(-9.80665 * (end - base) / (287.05287 * t))
                        : std::pow((t + lapse * (end - base)) / t, -9.80665 / (287.05287 * lapse));
        if (h <= layer[0]) {
            return p;
        }
        base = layer[0];
    }
    return p;
}
double atmosphereAltitude(double p) {
    require(std::isfinite(p) && p >= atmospherePressure(86000) && p <= atmospherePressure(-1000),
            "气压应为正值，并对应 −1000 至 86 000 m");
    double low = -1000, high = 86000;
    for (int i = 0; i < 90; ++i) {
        double mid = (low + high) / 2;
        if (atmospherePressure(mid) > p) {
            low = mid;
        } else {
            high = mid;
        }
    }
    return (low + high) / 2;
}
Axis::Axis(const std::wstring &name, double low, double high, int intervals, bool roundBounds) {
    double magnitude = std::max(std::abs(low), std::abs(high)), span = high - low;
    origin = std::isfinite(span) && span > 0 && magnitude / span > 100000 ? low : 0;
    magnitude = std::max(std::abs(low - origin), std::abs(high - origin));
    int exponent = magnitude > 0 && (magnitude < .001 || magnitude >= 1e6)
                       ? std::clamp(static_cast<int>(std::floor(std::log10(magnitude) / 3)) * 3, -300, 300)
                       : 0;
    factor = std::pow(10., exponent);
    low = (low - origin) / factor;
    high = (high - origin) / factor;
    if (low == high) {
        double padding = std::max(std::abs(low), 1.) * .05;
        low -= padding;
        high += padding;
    } else if (roundBounds) {
        double padding = (high - low) * .06;
        low -= padding;
        high += padding;
    }
    double raw = (high - low) / std::max(intervals, 2), power = std::pow(10., std::floor(std::log10(raw))),
           fraction = raw / power, multiple = 10;
    for (double m : {1., 2., 2.5, 5., 10.}) {
        if (fraction <= m * (1 + 1e-12)) {
            multiple = m;
            break;
        }
    }
    double step = multiple * power;
    if (roundBounds) {
        low = std::floor(low / step) * step;
        high = std::ceil(high / step) * step;
    }
    minimum = low;
    maximum = high;
    double first = std::ceil(low / step - 1e-9), last = std::floor(high / step + 1e-9);
    int count = std::min(32, static_cast<int>(std::max(0., last - first)) + 1);
    int precision = std::max(0, static_cast<int>(-std::floor(std::log10(step))));
    double scaled = step * std::pow(10., std::min(15, precision));
    if (std::abs(scaled - std::round(scaled)) > 1e-8) {
        ++precision;
    }
    precision = std::min(15, precision);
    for (int i = 0; i < count; ++i) {
        double v = (first + i) * step;
        if (std::abs(v) < step * 1e-8) {
            v = 0;
        }
        std::wostringstream out;
        out.imbue(std::locale::classic());
        out << std::fixed << std::setprecision(precision) << v;
        ticks.push_back({v, out.str()});
    }
    caption = name;
    if (origin != 0) {
        caption += L" - (" + format(origin) + L")";
    }
    if (exponent) {
        caption += L" (×10^" + std::to_wstring(exponent) + L")";
    }
}
double Axis::fraction(double value) const {
    return tickFraction((value - origin) / factor);
}
double Axis::tickFraction(double value) const {
    return (value - minimum) / (maximum - minimum);
}
} // namespace toolbox
