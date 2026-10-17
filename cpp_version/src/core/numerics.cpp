#define NOMINMAX
#include "toolbox/core.hpp"
#include <windows.h>

namespace toolbox {
void require(bool ok, const char *message) {
    if (!ok) {
        throw std::runtime_error(message);
    }
}
void finite(std::initializer_list<double> values) {
    for (double v : values) {
        require(std::isfinite(v), "计算超出双精度数值范围，请检查输入数量级");
    }
}
void finite(const Values &values) {
    for (double v : values) {
        finite({v});
    }
}
std::wstring wide(const std::string &text) {
    if (text.empty()) {
        return {};
    }
    int n = MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), nullptr, 0);
    std::wstring out(n, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), &out[0], n);
    return out;
}
std::string utf8(const std::wstring &text) {
    if (text.empty()) {
        return {};
    }
    int n = WideCharToMultiByte(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), nullptr, 0, nullptr,
                                nullptr);
    std::string out(n, '\0');
    WideCharToMultiByte(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), &out[0], n, nullptr, nullptr);
    return out;
}
double polynomial(const Values &c, double x) {
    double y = 0;
    for (auto it = c.rbegin(); it != c.rend(); ++it) {
        y = y * x + *it;
    }
    return y;
}
std::pair<double, double> polynomialSlope(const Values &c, double x) {
    double y = 0, d = 0;
    for (auto it = c.rbegin(); it != c.rend(); ++it) {
        d = d * x + y;
        y = y * x + *it;
    }
    return {y, d};
}
double inverse(const Curve &f, double target, double low, double high) {
    finite({target, low, high});
    double a = f(low), b = f(high);
    finite({a, b});
    require(target >= std::min(a, b) - 1e-10 && target <= std::max(a, b) + 1e-10,
            "输入超出所选曲线的有效范围");
    for (int i = 0; i < 100; ++i) {
        double mid = (low + high) / 2, value = f(mid);
        finite({value});
        if ((value < target) == (a < b)) {
            low = mid;
        } else {
            high = mid;
        }
    }
    double root = (low + high) / 2;
    require(std::abs(f(root) - target) <= 1e-9 * std::max(std::abs(target), 1.0),
            "所选系数曲线在此输入处没有连续的反算解");
    return root;
}
Values leastSquares(const std::vector<Values> &rows, const Values &y) {
    const auto m = rows.size(), n = m ? rows[0].size() : 0;
    require(n > 0 && m >= n && y.size() == m, "校准点数量不足以确定所选系数");
    finite(y);
    for (const auto &row : rows) {
        require(row.size() == n, "校准矩阵维度错误");
        finite(row);
    }
    std::vector<Values> q, r(n, Values(n));
    Values scales(n);
    // 逐列缩放和两次重正交化，保持与 Rust 的 QR 求解顺序一致。
    for (std::size_t j = 0; j < n; ++j) {
        for (const auto &row : rows) {
            scales[j] = std::max(scales[j], std::abs(row[j]));
        }
        require(scales[j] != 0, "校准点不能独立确定系数");
        Values v(m);
        for (std::size_t k = 0; k < m; ++k) {
            v[k] = rows[k][j] / scales[j];
        }
        for (int pass = 0; pass < 2; ++pass) {
            for (std::size_t i = 0; i < j; ++i) {
                double dot = 0;
                for (std::size_t k = 0; k < m; ++k) {
                    dot += q[i][k] * v[k];
                }
                r[i][j] += dot;
                for (std::size_t k = 0; k < m; ++k) {
                    v[k] -= dot * q[i][k];
                }
            }
        }
        double norm = 0;
        for (double value : v) {
            norm = std::hypot(norm, value);
        }
        require(norm >= 1e-12, "校准点重合或分布过于集中，无法可靠拟合");
        r[j][j] = norm;
        for (double &value : v) {
            value /= norm;
        }
        q.push_back(v);
    }
    Values x(n);
    for (std::size_t j = 0; j < n; ++j) {
        for (std::size_t k = 0; k < m; ++k) {
            x[j] += q[j][k] * y[k];
        }
    }
    for (std::size_t j = n; j-- > 0;) {
        for (std::size_t k = j + 1; k < n; ++k) {
            x[j] -= r[j][k] * x[k];
        }
        x[j] /= r[j][j];
    }
    for (std::size_t j = 0; j < n; ++j) {
        x[j] /= scales[j];
    }
    finite(x);
    return x;
}
double rss(const Values &values) {
    double total = 0;
    for (double v : values) {
        total = std::hypot(total, v);
    }
    finite({total});
    return total;
}
static double compensated(const Values &values) {
    double sum = 0, correction = 0;
    for (double value : values) {
        double next = sum + value;
        correction += std::abs(sum) >= std::abs(value) ? (sum - next) + value : (value - next) + sum;
        sum = next;
    }
    return sum + correction;
}
Statistics statistics(const Values &values) {
    require(values.size() >= 2, "至少需要 2 个测量值");
    finite(values);
    double count = static_cast<double>(values.size());
    double minimum = *std::min_element(values.begin(), values.end()),
           maximum = *std::max_element(values.begin(), values.end());
    Values differences, residuals;
    bool valid = true;
    for (double v : values) {
        differences.push_back(v - values[0]);
        valid = valid && std::isfinite(differences.back());
    }
    double mean;
    if (valid) {
        Values divided;
        for (double d : differences) {
            divided.push_back(d / count);
        }
        double delta = compensated(divided);
        mean = values[0] + delta;
        for (double d : differences) {
            residuals.push_back((d - delta) / std::sqrt(count - 1));
        }
    } else {
        double scale = std::max(std::abs(minimum), std::abs(maximum));
        Values normalized;
        for (double v : values) {
            normalized.push_back((v / scale) / count);
        }
        mean = compensated(normalized) * scale;
        for (double v : values) {
            residuals.push_back((v - mean) / std::sqrt(count - 1));
        }
    }
    double sd = rss(residuals);
    finite({mean, sd, minimum, maximum});
    return {mean, sd, minimum, maximum, sd / std::sqrt(count)};
}
double rangeCoefficient(std::size_t n) {
    static const double c[] = {1.13, 1.69, 2.06, 2.33, 2.53, 2.70, 2.85, 2.97};
    require(n >= 2 && n <= 9, "极差法支持 2～9 次测量");
    return c[n - 2];
}
std::wstring TemperatureUnit::symbol() const {
    return index == 1 ? L"K" : index == 2 ? L"°F" : L"°C";
}
double TemperatureUnit::celsius(double t) const {
    return index == 1 ? t - 273.15 : index == 2 ? (t - 32) / 1.8 : t;
}
double TemperatureUnit::display(double t) const {
    return index == 1 ? t + 273.15 : index == 2 ? t * 1.8 + 32 : t;
}
double TemperatureUnit::span() const {
    return index == 2 ? 1.8 : 1.0;
}
double convert(Dimension dimension, double value, std::size_t from, std::size_t to) {
    const auto &list = units(dimension);
    require(from < list.size() && to < list.size(), "单位不存在");
    finite({value});
    const auto &a = list[from];
    const auto &b = list[to];
    bool temperature = dimension == Dimension::Temperature;
    if (temperature) {
        double minimum = a.id == 1001 ? -273.15 : a.id == 1002 ? -459.67 : a.id == 999 ? -218.52 : 0;
        require(value >= minimum, "绝对温度不能低于 0 K");
    }
    double result = from == to    ? value
                    : temperature ? (std::max(0.0, std::fma(value, a.factor, a.offset)) - b.offset) / b.factor
                                  : value * (a.factor / b.factor);
    finite({result});
    require(temperature || value == 0 || result != 0, "单位换算结果过小，超出双精度可表示范围");
    return result;
}
} // namespace toolbox
