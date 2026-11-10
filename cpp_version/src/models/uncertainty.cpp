// 分量树逐层合成；每一层灵敏系数只作用一次，A 类使用样本标准差。
#include "toolbox/models.hpp"
namespace toolbox {

std::optional<std::size_t> parseAverageCount(const std::wstring &text) {
    const auto first = text.find_first_not_of(L" \t\r\n\f\v");
    if (first == std::wstring::npos) return std::nullopt;
    const auto last = text.find_last_not_of(L" \t\r\n\f\v");
    const auto number = text.substr(first, last - first + 1);
    const char *error = "取测量次数 n 须为正整数";
    require(number.find_first_not_of(L"0123456789") == std::wstring::npos, error);
    try {
        const auto n = std::stoull(number);
        require(n > 0 && n <= std::numeric_limits<std::size_t>::max(), error);
        return std::size_t(n);
    } catch (const std::exception &) { throw std::runtime_error(error); }
}
std::optional<int> parseSignificantDigits(const std::wstring &text) {
    const auto first = text.find_first_not_of(L" \t\r\n\f\v");
    if (first == std::wstring::npos) return std::nullopt;
    auto number = text.substr(first, text.find_last_not_of(L" \t\r\n\f\v") - first + 1);
    const char *error = "有效数字须为 1～17 的整数，留空保留原始计算精度";
    require(number.find_first_not_of(L"0123456789") == std::wstring::npos, error);
    try {
        auto n = std::stoull(number);
        require(n >= 1 && n <= 17, error);
        return int(n);
    } catch (const std::exception &) { throw std::runtime_error(error); }
}

static std::pair<std::string, int> decimalDigits(double value) {
    finite({value});
    require(value > 0, "修约数值必须大于 0");
    auto text = rawNumber(value);
    auto decimal = text.find('.');
    int point = int(decimal == std::string::npos ? text.size() : decimal);
    if (decimal != std::string::npos) text.erase(decimal, 1);
    auto first = text.find_first_not_of('0');
    require(first != std::string::npos, "修约数值无效");
    return {text.substr(first), point - int(first) - 1};
}
int uncertaintyExponent(double value) { return decimalDigits(value).second; }

double roundedUncertainty(double value, int places, bool carry) {
    finite({value});
    require(value >= 0, "不确定度不能为负数");
    if (value == 0) return 0;
    auto [digits, exponent] = decimalDigits(value);
    long long keep = static_cast<long long>(exponent) + places + 1;
    if (keep >= static_cast<long long>(digits.size())) return value;
    std::size_t take = std::size_t(std::max(0LL, keep));
    auto rest = digits.substr(take);
    bool increment = carry ? rest.find_first_not_of('0') != std::string::npos
        : keep < 0 ? false
        : rest[0] > '5' || (rest[0] == '5' && (rest.find_first_not_of('0', 1) != std::string::npos ||
                                                  (take && (digits[take - 1] - '0') % 2)));
    auto retained = take ? digits.substr(0, take) : std::string("0");
    if (increment) {
        std::size_t i = retained.size();
        while (i && retained[i - 1] == '9') retained[--i] = '0';
        if (i) ++retained[i - 1]; else retained.insert(retained.begin(), '1');
    }
    return parse(retained + "e" + std::to_string(-places));
}
std::pair<double, double> roundedCombinedUncertainty(double uc, double k, int digits, bool carry) {
    finite({uc, k});
    require(uc >= 0 && k > 0, "合成不确定度须非负，包含因子须大于 0");
    require(digits >= 1 && digits <= 4, "合成有效数字须为 1～4 位");
    double expanded = uc * k;
    finite({expanded});
    double reference = std::max(uc, expanded);
    int places = reference == 0 ? digits - 1 : digits - uncertaintyExponent(reference) - 1;
    double rounded = roundedUncertainty(uc, places, carry);
    double roundedExpanded = rounded * k;
    finite({roundedExpanded});
    return {rounded, roundedExpanded};
}
double roundSignificant(double value, UncertaintyRounding rounding) {
    finite({value});
    require(value >= 0, "不确定度不能为负数");
    if (!rounding.digits || value == 0) return value;
    require(*rounding.digits >= 1 && *rounding.digits <= 17, "有效数字须为 1～17 位");
    return roundedUncertainty(value, *rounding.digits - 1 - uncertaintyExponent(value), rounding.carry);
}
std::wstring formatRoundedUncertainty(double value, std::optional<int> digits) {
    if (!digits || value == 0) return format(value);
    int places = std::max(0, *digits - 1 - uncertaintyExponent(value));
    auto text = rawNumber(value);
    auto dot = text.find('.');
    int present = dot == std::string::npos ? 0 : int(text.size() - dot - 1);
    if (places > present) {
        if (dot == std::string::npos) text += '.';
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
    UncertaintyDetail detail{name, 0, 0, sensitivity, 0, {}, rounding.digits};
    if (kind == 0) {
        auto s = statistics(values);
        if (useRange) detail.standard = (s.maximum - s.minimum) / rangeCoefficient(values.size()) /
                                        std::sqrt(double(values.size()));
        else if (averageCount) {
            require(*averageCount > 0, "取测量次数 n 须为正整数");
            detail.standard = s.sd / std::sqrt(double(*averageCount));
        } else detail.standard = s.sd;
    } else if (kind == 1) {
        require(std::isfinite(halfWidth) && halfWidth >= 0, "B 类半宽不能为负");
        detail.standard = halfWidth / distributionDivisor(distribution);
    } else {
        Values contributions;
        for (const auto &child : children) {
            auto d = child.calculate();
            contributions.push_back(d.contribution);
            detail.children.push_back(std::move(d));
        }
        detail.standard = combineUncertainty(contributions, maximum);
    }
    detail.rawStandard = detail.standard;
    detail.standard = roundSignificant(detail.standard, rounding);
    detail.contribution = detail.standard * sensitivity;
    finite({detail.standard, detail.contribution});
    return detail;
}
} // namespace toolbox
