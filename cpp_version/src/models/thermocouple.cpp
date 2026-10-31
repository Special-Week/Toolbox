// 热电偶正向参考函数与有界反算。分段边界、修正项及允差范围分别管理。
#include "toolbox/models.hpp"
#include <map>
#include <mutex>
#include <sstream>

namespace toolbox {
std::pair<double, double> ThermoSegment::evaluate(double t) const {
    auto result = polynomialSlope(coefficients, t);
    if (gaussian) {
        const auto &g = *gaussian;
        double term = g[0] * std::exp(g[1] * (t - g[2]) * (t - g[2]));
        result.first += term;
        result.second += term * 2 * g[1] * (t - g[2]);
    }
    return result;
}
const std::vector<ThermoSegment> &thermoSegments(const std::string &kind) {
    static std::map<std::string, std::vector<ThermoSegment>> cache;
    static std::mutex mutex;
    std::lock_guard<std::mutex> guard(mutex);
    auto found = cache.find(kind);
    if (found != cache.end()) {
        return found->second;
    }
    std::vector<ThermoSegment> result;
    for (const auto &extra : extraReferences()) {
        if (extra.kind == kind) {
            for (std::size_t i = 0; i < extra.coefficients.size(); ++i) {
                Values c = extra.coefficients[i];
                for (auto &v : c) {
                    v *= extra.scale;
                }
                result.push_back({i == 0 ? extra.minimum : extra.join,
                                  i + 1 == extra.coefficients.size() ? extra.maximum : extra.join,
                                  c,
                                  {}});
            }
        }
    }
    if (result.empty()) {
        std::istringstream input(nistSource(kind));
        std::string line;
        while (std::getline(input, line)) {
            if (line.find("Inverse coefficients") != std::string::npos) {
                break;
            }
            if (line.rfind("range:", 0) == 0) {
                auto range = line.substr(6);
                std::replace(range.begin(), range.end(), ',', ' ');
                std::istringstream numbers(range);
                std::string lowText, highText;
                int degree;
                require(bool(numbers >> lowText >> highText >> degree) && degree >= 0 && degree <= 20,
                        "热电偶参考数据损坏");
                // 边界也使用统一十进制解析，避免旧 CRT 把 1768.1 等上限读低一位。
                double lo = parse(lowText), hi = parse(highText);
                Values c;
                for (int i = 0; i <= degree; ++i) {
                    require(bool(std::getline(input, line)), "热电偶系数不完整");
                    c.push_back(parse(line));
                }
                result.push_back({lo, hi, c, {}});
            } else if (line.find("exponential:") != std::string::npos) {
                std::array<double, 3> g{};
                for (auto &value : g) {
                    require(bool(std::getline(input, line)), "高斯项不完整");
                    value = parse(line.substr(line.find('=') + 1));
                }
                require(!result.empty(), "高斯项缺少温区");
                result.back().gaussian = g;
            }
        }
    }
    require(!result.empty(), "没有该分度号的数据");
    return cache.emplace(kind, std::move(result)).first->second;
}
std::pair<double, double> thermoBounds(const std::string &kind) {
    const auto &s = thermoSegments(kind);
    return {s.front().minimum, s.back().maximum};
}
static const ThermoSegment &segment(const std::string &kind, double t) {
    finite({t});
    for (const auto &s : thermoSegments(kind)) {
        if (t >= s.minimum && t <= s.maximum) {
            return s;
        }
    }
    throw std::runtime_error("温度超出所选分度号范围");
}
double thermoEmf(const std::string &kind, double t) {
    return segment(kind, t).evaluate(t).first;
}
double thermoSlope(const std::string &kind, double t) {
    return segment(kind, t).evaluate(t).second;
}
double thermoTemperature(const std::string &kind, double e) {
    finite({e});
    auto bounds = thermoBounds(kind);
    if (kind == "B") {
        require(e > 0, "B 型非正热电势不能唯一反算温度");
        double lo = 0, hi = 50;
        for (int i = 0; i < 64; ++i) {
            double mid = (lo + hi) / 2;
            (thermoSlope(kind, mid) < 0 ? lo : hi) = mid;
        }
        bounds.first = (lo + hi) / 2;
    }
    double lo = bounds.first, hi = bounds.second;
    double lowEmf = thermoEmf(kind, lo), highEmf = thermoEmf(kind, hi);
    require(e >= lowEmf && e <= highEmf, "热电势超出所选分度号的参考函数范围");
    if (e == lowEmf) {
        return lo;
    }
    if (e == highEmf) {
        return hi;
    }
    // 精确命中连接点时直接返回，避免微小分段差异让二分迭代误报无解。
    for (const auto &part : thermoSegments(kind)) {
        const double join = part.maximum;
        if (join > lo && join < hi && thermoEmf(kind, join) == e) {
            return join;
        }
    }
    for (int i = 0; i < 80; ++i) {
        double mid = lo + (hi - lo) * .5;
        (thermoEmf(kind, mid) < e ? lo : hi) = mid;
    }
    double t = lo + (hi - lo) * .5;
    require(std::abs(thermoEmf(kind, t) - e) <= 1e-9 * std::max(std::abs(e), 1.),
            "该电势位于分段曲线的不连续区间，无法可靠反算温度");
    return t;
}
std::vector<Tolerance> thermoTolerances(const std::string &kind, double t) {
    struct Def {
        const wchar_t *name;
        double lo, hi, a, b;
        const wchar_t *formula;
    };
    std::vector<Def> d;
    d.reserve(3);
    if (kind == "K" || kind == "N" || kind == "E") {
        d = {{L"I 级", -40, kind == "E" ? 800. : 1000., 1.5, .004, L"±max(1.5, 0.004|t|) °C"},
             {L"II 级", -40, kind == "E" ? 900. : 1200., 2.5, .0075, L"±max(2.5, 0.0075|t|) °C"},
             {L"III 级", -200, 40, 2.5, .015, L"±max(2.5, 0.015|t|) °C"}};
    } else if (kind == "J") {
        d = {{L"I 级", -40, 750, 1.5, .004, L"±max(1.5, 0.004|t|) °C"},
             {L"II 级", -40, 750, 2.5, .0075, L"±max(2.5, 0.0075|t|) °C"}};
    } else if (kind == "T") {
        d = {{L"I 级", -40, 350, .5, .004, L"±max(0.5, 0.004|t|) °C"},
             {L"II 级", -40, 350, 1, .0075, L"±max(1, 0.0075|t|) °C"},
             {L"III 级", -200, 40, 1, .015, L"±max(1, 0.015|t|) °C"}};
    } else if (kind == "B") {
        d = {{L"II 级", 600, 1700, 0, .0025, L"±0.0025|t| °C"},
             {L"III 级", 600, 1700, 0, .005, L"±0.005|t| °C"}};
    } else if (kind == "S" || kind == "R") {
        d = {{L"I 级", 0, 1600, 1, 0, L"±[1 + 0.003max(t−1100, 0)] °C"},
             {L"II 级", 0, 1600, 1.5, .0025, L"±max(1.5, 0.0025|t|) °C"}};
    }
    std::vector<Tolerance> out;
    for (std::size_t i = 0; i < d.size(); ++i) {
        auto &v = d[i];
        std::optional<double> delta;
        if (t >= v.lo && t <= v.hi) {
            delta = ((kind == "S" || kind == "R") && i == 0) ? 1 + .003 * std::max(t - 1100., 0.)
                                                             : std::max(v.a, v.b * std::abs(t));
        }
        out.push_back({v.name, v.formula, v.lo, v.hi, delta});
    }
    return out;
}
} // namespace toolbox
