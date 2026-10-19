#pragma once
// 通用校准曲线接口；系数按原单位升幂排列，预测使用归一化模型保持精度。
#include "core.hpp"
#include <optional>

namespace toolbox {
enum class CurveFitKind { Linear, ThroughOrigin, Quadratic, Cubic };

struct CurveFit {
    static constexpr std::size_t maximumPoints = 200;
    CurveFitKind kind;
    Values coefficients, fitted, residuals;
    std::optional<double> correlation, rSquared, residualSd;
    double rmse = 0, maximumResidual = 0, minimumX = 0, maximumX = 0;

    CurveFit(const std::vector<std::pair<double, double>> &points, CurveFitKind model);
    static int degree(CurveFitKind model);
    static std::size_t parameterCount(CurveFitKind model);
    double predict(double input) const;

  private:
    struct Normalization {
        double origin = 0, scale = 1;
        double normalize(double value) const;
    };
    Normalization x_, y_;
    Values normalizedCoefficients_;
    static Normalization normalization(const Values &values, bool centered);
    static Values originalCoefficients(const Values &coefficients, Normalization x, Normalization y);
};
} // namespace toolbox
