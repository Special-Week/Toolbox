// 与 Rust 版使用相同的归一化坐标、缩放 QR 和统计量定义，不引入仪器专用修正。
#include "toolbox/curve_fit.hpp"

namespace toolbox {
namespace {
// Neumaier 补偿求和，保留均值及协方差中正负项相消时的低位。
double compensatedSum(const Values &values) {
    double total = 0, correction = 0;
    for (double value : values) {
        double next = total + value;
        correction += std::abs(total) >= std::abs(value) ? (total - next) + value
                                                       : (value - next) + total;
        total = next;
    }
    return total + correction;
}

double evaluate(const Values &coefficients, double input) {
    double value = 0;
    for (auto it = coefficients.rbegin(); it != coefficients.rend(); ++it) {
        value = std::fma(value, input, *it);
    }
    return value;
}
} // namespace

int CurveFit::degree(CurveFitKind model) {
    return model == CurveFitKind::Quadratic ? 2 : (model == CurveFitKind::Cubic ? 3 : 1);
}

std::size_t CurveFit::parameterCount(CurveFitKind model) {
    return model == CurveFitKind::ThroughOrigin ? 1 : std::size_t(degree(model) + 1);
}

double CurveFit::Normalization::normalize(double value) const {
    return (value - origin) / scale;
}

CurveFit::Normalization CurveFit::normalization(const Values &values, bool centered) {
    double minimum = *std::min_element(values.begin(), values.end());
    double maximum = *std::max_element(values.begin(), values.end());
    Normalization result;
    if (centered) {
        // 分别除以 2，避免极值相加溢出；常量保留其原值，避免次正规数减半下溢。
        result.origin = minimum == maximum ? minimum : minimum / 2 + maximum / 2;
    }
    result.scale = 0;
    for (double value : values) {
        result.scale = std::max(result.scale, std::abs(value - result.origin));
    }
    if (result.scale == 0) {
        result.scale = 1;
    }
    return result;
}

Values CurveFit::originalCoefficients(const Values &coefficients, Normalization x, Normalization y) {
    // 将 ys·P((x−x0)/xs)+y0 逐项展开；这里不使用界面显示的修约值。
    Values expanded{coefficients.back() * y.scale};
    require(coefficients.back() == 0 || expanded[0] != 0,
            "系数换算发生数值下溢，请调整输入量或响应量的单位");
    for (std::size_t j = coefficients.size() - 1; j-- > 0;) {
        Values next(expanded.size() + 1);
        for (std::size_t index = 0; index < expanded.size(); ++index) {
            double scaled = expanded[index] / x.scale;
            require(expanded[index] == 0 || scaled != 0,
                    "展开系数过小，无法用双精度数值表示，请调整输入量单位");
            next[index] = std::fma(-x.origin, scaled, next[index]);
            next[index + 1] += scaled;
        }
        next[0] += coefficients[j] * y.scale;
        require(std::all_of(next.begin(), next.end(), [](double value) { return std::isfinite(value); }),
                "展开系数超出数值范围，请调整输入量的单位或基准");
        expanded = std::move(next);
    }
    expanded[0] += y.origin;
    finite(expanded);
    return expanded;
}

CurveFit::CurveFit(const std::vector<std::pair<double, double>> &points, CurveFitKind model) : kind(model) {
    require(points.size() >= 2 && points.size() <= maximumPoints, "校准数据须为 2～200 组");
    require(int(kind) >= 0 && int(kind) <= 3, "未知拟合模型");
    require(points.size() >= parameterCount(kind), "所选模型的数据组数不足，请增加数据及不同的 x 值");
    Values inputs, responses;
    for (const auto &point : points) {
        inputs.push_back(point.first);
        responses.push_back(point.second);
    }
    finite(inputs);
    finite(responses);
    minimumX = *std::min_element(inputs.begin(), inputs.end());
    maximumX = *std::max_element(inputs.begin(), inputs.end());
    require(minimumX != maximumX, "所有输入量 x 相同，无法建立有效的校准曲线");

    bool centered = kind != CurveFitKind::ThroughOrigin;
    x_ = normalization(inputs, centered);
    y_ = normalization(responses, centered);
    Values normalizedX, normalizedY;
    std::vector<Values> rows;
    for (std::size_t index = 0; index < points.size(); ++index) {
        double input = x_.normalize(inputs[index]);
        normalizedX.push_back(input);
        normalizedY.push_back(y_.normalize(responses[index]));
        if (kind == CurveFitKind::ThroughOrigin) {
            rows.push_back({input});
        } else {
            Values row(std::size_t(degree(kind) + 1), 1);
            for (std::size_t power = 1; power < row.size(); ++power) {
                row[power] = row[power - 1] * input;
            }
            rows.push_back(std::move(row));
        }
    }
    normalizedCoefficients_ = leastSquares(rows, normalizedY);
    if (kind == CurveFitKind::ThroughOrigin) {
        normalizedCoefficients_.insert(normalizedCoefficients_.begin(), 0);
    }
    coefficients = originalCoefficients(normalizedCoefficients_, x_, y_);
    for (std::size_t index = 0; index < points.size(); ++index) {
        double predicted = evaluate(normalizedCoefficients_, normalizedX[index]);
        fitted.push_back(std::fma(predicted, y_.scale, y_.origin));
        // 在归一化响应坐标中求差，再恢复单位，保留大基准上的小残差。
        residuals.push_back((normalizedY[index] - predicted) * y_.scale);
    }
    finite(fitted);
    finite(residuals);

    // r 始终由原始 x/y 的中心化值计算，不能用拟合值替代原始响应量。
    auto centeredX = normalization(inputs, true), centeredY = normalization(responses, true);
    double count = double(points.size());
    Values sx, sy, dividedX, dividedY;
    for (std::size_t index = 0; index < points.size(); ++index) {
        sx.push_back(centeredX.normalize(inputs[index]));
        sy.push_back(centeredY.normalize(responses[index]));
        dividedX.push_back(sx.back() / count);
        dividedY.push_back(sy.back() / count);
    }
    double meanX = compensatedSum(dividedX), meanY = compensatedSum(dividedY);
    Values dx, dy;
    for (std::size_t index = 0; index < points.size(); ++index) {
        dx.push_back(sx[index] - meanX);
        dy.push_back(sy[index] - meanY);
    }
    double xNorm = rss(dx), yNorm = rss(dy);
    if (yNorm != 0) {
        Values covariance, scaledResiduals;
        for (std::size_t index = 0; index < points.size(); ++index) {
            covariance.push_back((dx[index] / xNorm) * (dy[index] / yNorm));
            scaledResiduals.push_back(residuals[index] / centeredY.scale);
        }
        double r = compensatedSum(covariance), ratio = rss(scaledResiduals) / yNorm;
        double determination = 1 - ratio * ratio;
        finite({r, determination});
        correlation = std::clamp(r, -1.0, 1.0);
        // 所有模型统一采用中心化 R²；过原点模型允许出现负值，不强行截断。
        rSquared = determination;
    }
    Values errors;
    for (double residual : residuals) {
        errors.push_back(residual / std::sqrt(count));
        maximumResidual = std::max(maximumResidual, std::abs(residual));
    }
    rmse = rss(errors);
    std::size_t freedom = points.size() - parameterCount(kind);
    if (freedom > 0) {
        errors.clear();
        for (double residual : residuals) {
            errors.push_back(residual / std::sqrt(double(freedom)));
        }
        residualSd = rss(errors);
    }
}

double CurveFit::predict(double input) const {
    finite({input});
    double value = std::fma(evaluate(normalizedCoefficients_, x_.normalize(input)), y_.scale, y_.origin);
    finite({value});
    return value;
}
} // namespace toolbox
