// 分量树逐层合成；每一层灵敏系数只作用一次，A 类使用样本标准差。
#include "toolbox/models.hpp"
namespace toolbox {

std::optional<std::size_t> parseAverageCount(const std::wstring &text) {
    const auto first = text.find_first_not_of(L" \t\r\n\f\v");
    if (first == std::wstring::npos)
        return std::nullopt;
    const auto last = text.find_last_not_of(L" \t\r\n\f\v");
    const auto number = text.substr(first, last - first + 1);
    const char *error = "取测量次数 n 须为正整数";
    require(number.find_first_not_of(L"0123456789") == std::wstring::npos, error);
    try {
        const auto n = std::stoull(number);
        require(n > 0 && n <= std::numeric_limits<std::size_t>::max(), error);
        return std::size_t(n);
    } catch (const std::exception &) {
        throw std::runtime_error(error);
    }
}
std::optional<int> parseSignificantDigits(const std::wstring &text) {
    const auto first = text.find_first_not_of(L" \t\r\n\f\v");
    if (first == std::wstring::npos)
        return std::nullopt;
    auto number = text.substr(first, text.find_last_not_of(L" \t\r\n\f\v") - first + 1);
    const char *error = "有效数字须为 1～17 的整数，留空保留原始计算精度";
    require(number.find_first_not_of(L"0123456789") == std::wstring::npos, error);
    try {
        auto n = std::stoull(number);
        require(n >= 1 && n <= 17, error);
        return int(n);
    } catch (const std::exception &) {
        throw std::runtime_error(error);
    }
}

std::optional<double> parseDegreesOfFreedom(const std::wstring &text) {
    const auto first = text.find_first_not_of(L" \t\r\n\f\v");
    if (first == std::wstring::npos)
        return std::nullopt;
    const auto trimmed = text.substr(first, text.find_last_not_of(L" \t\r\n\f\v") - first + 1);
    if (trimmed == L"∞")
        return std::nullopt;
    std::string lower = utf8(trimmed);
    for (char &ch : lower) {
        if (ch >= 'A' && ch <= 'Z')
            ch = char(ch - 'A' + 'a');
    }
    if (lower == "inf" || lower == "infinity")
        return std::nullopt;
    const char *error = "自由度须为大于 0 的数，留空表示 ∞";
    double value = 0;
    try {
        value = parse(utf8(trimmed));
    } catch (const std::exception &) {
        throw std::runtime_error(error);
    }
    require(std::isfinite(value) && value > 0, error);
    return value;
}
double rangeDegreesOfFreedom(std::size_t n) {
    static constexpr double kTable[8] = {0.9, 1.8, 2.7, 3.6, 4.5, 5.4, 6.3, 7.2};
    require(n >= 2 && n <= 9, "极差法支持 2～9 次测量");
    return kTable[n - 2];
}
std::wstring formatDegreesOfFreedom(DegreesOfFreedom dof) {
    if (dof.kind == DofKind::Infinite)
        return L"∞";
    if (dof.kind == DofKind::NotApplicable)
        return L"不适用";
    return format(dof.value);
}
DegreesOfFreedom combineDegreesOfFreedom(const std::vector<std::pair<double, DegreesOfFreedom>> &terms,
                                         bool maximum) {
    require(!terms.empty(), "至少需要一个不确定度分量");
    if (maximum)
        return DegreesOfFreedom::notApplicable();
    Values contributions;
    contributions.reserve(terms.size());
    for (const auto &item : terms)
        contributions.push_back(item.first);
    double uc = rss(contributions);
    if (uc == 0)
        return DegreesOfFreedom::notApplicable();
    double norm = 0;
    for (const auto &[q, dof] : terms) {
        if (q == 0)
            continue;
        if (dof.kind == DofKind::NotApplicable)
            return DegreesOfFreedom::notApplicable();
        if (dof.kind == DofKind::Infinite)
            continue;
        require(std::isfinite(dof.value) && dof.value > 0, "自由度须为大于 0 的数");
        const double weight = std::abs(q / uc) / std::sqrt(std::sqrt(dof.value));
        norm = std::hypot(norm, weight * weight);
    }
    double eff = (1 / norm) / norm;
    if (!std::isfinite(eff))
        return DegreesOfFreedom::infinite();
    require(eff > 0, "有效自由度超出双精度数值范围");
    return DegreesOfFreedom::finite(eff);
}
static double lnGamma(double z) {
    static constexpr double kCoeffs[9] = {
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    };
    double shift = 0.0;
    while (z < 1.5) {
        shift -= std::log(z);
        z += 1.0;
    }
    z -= 1.0;
    double x = kCoeffs[0];
    for (int i = 1; i < 9; ++i)
        x += kCoeffs[i] / (z + double(i));
    double t = z + 7.5;
    return shift + 0.5 * std::log(2.0 * 3.14159265358979323846) + (z + 0.5) * std::log(t) - t + std::log(x);
}
static double betaCf(double a, double b, double x) {
    constexpr double kFpMin = 1e-300;
    constexpr double kEps = 1e-15;
    double qab = a + b, qap = a + 1.0, qam = a - 1.0;
    double c = 1.0, d = 1.0 - qab * x / qap;
    if (std::abs(d) < kFpMin)
        d = kFpMin;
    d = 1.0 / d;
    double h = d;
    for (int m = 1; m <= 200; ++m) {
        double mf = double(m), m2 = 2.0 * mf;
        double aa = mf * (b - mf) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if (std::abs(d) < kFpMin)
            d = kFpMin;
        c = 1.0 + aa / c;
        if (std::abs(c) < kFpMin)
            c = kFpMin;
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + mf) * (qab + mf) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if (std::abs(d) < kFpMin)
            d = kFpMin;
        c = 1.0 + aa / c;
        if (std::abs(c) < kFpMin)
            c = kFpMin;
        d = 1.0 / d;
        double del = d * c;
        h *= del;
        if (std::abs(del - 1.0) < kEps)
            break;
    }
    return h;
}
static double studentTTail(double a, double lnV, double y) {
    const double s = 2.0 * y - lnV;
    double x = 0, lnX = 0, ln1mX = 0;
    if (s > 36.0) {
        x = std::exp(-s);
        lnX = -s;
        ln1mX = -x;
    } else if (s < -36.0) {
        double es = std::exp(s);
        x = 1.0 - es;
        lnX = -es;
        ln1mX = s;
    } else {
        double es = std::exp(s);
        double lnDenom = std::log1p(es);
        x = 1.0 / (1.0 + es);
        lnX = -lnDenom;
        ln1mX = s - lnDenom;
    }
    const double lnHalfPi = 0.5 * std::log(3.14159265358979323846);
    if (x < (a + 1.0) / (a + 2.5)) {
        double logPref = lnGamma(a + 0.5) - lnGamma(a + 1.0) - lnHalfPi + a * lnX + 0.5 * ln1mX;
        return std::exp(logPref) * betaCf(a, 0.5, x);
    }
    double logPref = lnGamma(a + 0.5) - lnGamma(a) - lnGamma(1.5) + 0.5 * ln1mX + a * lnX;
    return 1.0 - std::exp(logPref) * betaCf(0.5, a, 1.0 - x);
}
double studentTQuantile(DegreesOfFreedom dof, double confidence) {
    double z = 0;
    if (std::abs(confidence - 0.95) < 1e-12)
        z = 1.959963984540054;
    else if (std::abs(confidence - 0.99) < 1e-12)
        z = 2.5758293035489004;
    else
        throw std::runtime_error("仅支持 95% 或 99% 置信概率");
    if (dof.kind == DofKind::NotApplicable)
        throw std::runtime_error("当前合成模式或零不确定度不适用有效自由度，请改用手动指定包含因子 k");
    if (dof.kind == DofKind::Infinite)
        return z;
    double v = dof.value;
    require(std::isfinite(v) && v > 0, "有效自由度须为大于 0 的数");
    if (v >= 1e5) {
        double z2 = z * z, z3 = z2 * z, z5 = z3 * z2;
        return z + (z3 + z) / (4.0 * v) + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * v * v);
    }
    double alpha = 1.0 - confidence, a = 0.5 * v, lnV = std::log(v);
    require(a > 0 && std::isfinite(lnV), "有效自由度过小，t 分布包含因子超出双精度数值范围");
    double low = std::log(z), high = std::log(std::numeric_limits<double>::max());
    double tailHigh = studentTTail(a, lnV, high);
    require(std::isfinite(tailHigh) && tailHigh <= alpha, "有效自由度过小，t 分布包含因子超出双精度数值范围");
    for (int i = 0; i < 80; ++i) {
        double mid = 0.5 * (low + high);
        if (studentTTail(a, lnV, mid) > alpha)
            low = mid;
        else
            high = mid;
    }
    double y = 0.5 * (low + high);
    double residual = std::abs(studentTTail(a, lnV, y) - alpha);
    double t = std::exp(y);
    require(std::isfinite(t) && std::isfinite(residual) && residual <= 1e-9 * alpha,
            "有效自由度过小，t 分布包含因子超出双精度数值范围");
    return t;
}
double coverageFactor(int mode, double manualK, DegreesOfFreedom dof) {
    if (mode == 1)
        return studentTQuantile(dof, 0.95);
    if (mode == 2)
        return studentTQuantile(dof, 0.99);
    finite({manualK});
    require(manualK > 0, "包含因子须大于 0");
    return manualK;
}

static std::pair<std::string, int> decimalDigits(double value) {
    finite({value});
    require(value > 0, "修约数值必须大于 0");
    auto text = rawNumber(value);
    auto decimal = text.find('.');
    int point = int(decimal == std::string::npos ? text.size() : decimal);
    if (decimal != std::string::npos)
        text.erase(decimal, 1);
    auto first = text.find_first_not_of('0');
    require(first != std::string::npos, "修约数值无效");
    return {text.substr(first), point - int(first) - 1};
}
int uncertaintyExponent(double value) { return decimalDigits(value).second; }

double roundedUncertainty(double value, int places, bool carry) {
    finite({value});
    require(value >= 0, "不确定度不能为负数");
    if (value == 0)
        return 0;
    auto [digits, exponent] = decimalDigits(value);
    long long keep = static_cast<long long>(exponent) + places + 1;
    if (keep >= static_cast<long long>(digits.size()))
        return value;
    std::size_t take = std::size_t(std::max(0LL, keep));
    auto rest = digits.substr(take);
    bool increment = carry      ? rest.find_first_not_of('0') != std::string::npos
                     : keep < 0 ? false
                                : rest[0] > '5' || (rest[0] == '5' &&
                                                    (rest.find_first_not_of('0', 1) != std::string::npos ||
                                                     (take && (digits[take - 1] - '0') % 2)));
    auto retained = take ? digits.substr(0, take) : std::string("0");
    if (increment) {
        std::size_t i = retained.size();
        while (i && retained[i - 1] == '9')
            retained[--i] = '0';
        if (i)
            ++retained[i - 1];
        else
            retained.insert(retained.begin(), '1');
    }
    return parse(retained + "e" + std::to_string(-places));
}
std::pair<double, double> roundedCombinedUncertainty(double uc, double k, int digits, bool carry) {
    finite({uc, k});
    require(uc >= 0 && k > 0, "合成不确定度须非负，包含因子须大于 0");
    require(digits >= 1 && digits <= 4, "合成有效数字须为 1～4 位");
    double expanded = uc * k;
    finite({expanded});
    const UncertaintyRounding rounding{digits, carry};
    return {roundSignificant(uc, rounding), roundSignificant(expanded, rounding)};
}
double roundSignificant(double value, UncertaintyRounding rounding) {
    finite({value});
    require(value >= 0, "不确定度不能为负数");
    if (!rounding.digits || value == 0)
        return value;
    require(*rounding.digits >= 1 && *rounding.digits <= 17, "有效数字须为 1～17 位");
    return roundedUncertainty(value, *rounding.digits - 1 - uncertaintyExponent(value), rounding.carry);
}
std::wstring formatRoundedUncertainty(double value, std::optional<int> digits) {
    if (!digits || value == 0)
        return format(value);
    int places = std::max(0, *digits - 1 - uncertaintyExponent(value));
    auto text = rawNumber(value);
    auto dot = text.find('.');
    int present = dot == std::string::npos ? 0 : int(text.size() - dot - 1);
    if (places > present) {
        if (dot == std::string::npos)
            text += '.';
        text.append(std::size_t(places - present), '0');
    }
    return wide(text);
}
double distributionDivisor(int d) {
    switch (d) {
    case 1:
        return std::sqrt(6.);
    case 2:
        return 2;
    case 3:
        return 3;
    case 4:
        return 1;
    case 5:
        return std::sqrt(2.);
    default:
        return std::sqrt(3.);
    }
}
double combineUncertainty(const Values &values, bool maximum) {
    require(!values.empty(), "至少需要一个不确定度分量");
    finite(values);
    if (!maximum) {
        return rss(values);
    }
    double result = 0;
    for (double v : values) {
        result = std::max(result, std::abs(v));
    }
    return result;
}
UncertaintyDetail Component::calculate() const {
    finite({sensitivity});
    UncertaintyDetail detail{name, 0, 0, sensitivity, 0, DegreesOfFreedom::infinite(), {}, rounding.digits};
    if (kind == 0) {
        auto s = statistics(values);
        if (useRange) {
            detail.standard =
                (s.maximum - s.minimum) / rangeCoefficient(values.size()) / std::sqrt(double(values.size()));
            detail.dof = DegreesOfFreedom::finite(rangeDegreesOfFreedom(values.size()));
        } else {
            if (averageCount) {
                require(*averageCount > 0, "取测量次数 n 须为正整数");
                detail.standard = s.sd / std::sqrt(double(*averageCount));
            } else {
                detail.standard = s.sd;
            }
            detail.dof = DegreesOfFreedom::finite(double(values.size() - 1));
        }
    } else if (kind == 1) {
        require(std::isfinite(halfWidth) && halfWidth >= 0, "B 类半宽不能为负");
        if (dof) {
            require(std::isfinite(*dof) && *dof > 0, "自由度须为大于 0 的数，留空表示 ∞");
            detail.dof = DegreesOfFreedom::finite(*dof);
        } else {
            detail.dof = DegreesOfFreedom::infinite();
        }
        detail.standard = halfWidth / distributionDivisor(distribution);
    } else {
        Values contributions;
        std::vector<std::pair<double, DegreesOfFreedom>> dofTerms;
        for (const auto &child : children) {
            auto d = child.calculate();
            contributions.push_back(d.contribution);
            dofTerms.push_back({d.contribution, d.dof});
            detail.children.push_back(std::move(d));
        }
        detail.standard = combineUncertainty(contributions, maximum);
        detail.dof = combineDegreesOfFreedom(dofTerms, maximum);
    }
    detail.rawStandard = detail.standard;
    detail.standard = roundSignificant(detail.standard, rounding);
    detail.contribution = detail.standard * sensitivity;
    finite({detail.standard, detail.contribution});
    return detail;
}
} // 命名空间 toolbox
