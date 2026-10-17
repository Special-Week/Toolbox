// 分量树逐层合成；每一层灵敏系数只作用一次，A 类使用样本标准差。
#include "toolbox/models.hpp"
namespace toolbox {
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
    UncertaintyDetail detail{name, 0, sensitivity, 0, {}};
    if (kind == 0) {
        auto s = statistics(values);
        detail.standard = useRange ? (s.maximum - s.minimum) / rangeCoefficient(values.size()) /
                                         std::sqrt(double(values.size()))
                                   : s.standardError;
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
    detail.contribution = detail.standard * sensitivity;
    finite({detail.standard, detail.contribution});
    return detail;
}
double roundedUncertainty(double value, int places, bool carry) {
    double factor = std::pow(10., places), v = value * factor;
    if (!std::isfinite(factor) || !std::isfinite(v)) {
        return value;
    }
    return (carry ? std::ceil(v) : std::round(v)) / factor;
}
} // namespace toolbox
