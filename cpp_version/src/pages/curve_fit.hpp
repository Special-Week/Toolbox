#pragma once
// 页面内部共享的图表入口，拟合模型与窗口控件保持分离。
#include "toolbox/curve_fit.hpp"
#include "toolbox/gui.hpp"

namespace toolbox {
void drawCurveFitChart(Page &page, const CurveFit &fit,
                       const std::vector<std::pair<double, double>> &points,
                       const std::wstring &xLabel, const std::wstring &yLabel, bool residual);
} // namespace toolbox
