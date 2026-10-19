// 校准曲线及残差图：依据实际文字宽度分配刻度留白，图表不截获滚轮。
#include "curve_fit.hpp"

namespace toolbox {
void drawCurveFitChart(Page &p, const CurveFit &fit,
                       const std::vector<std::pair<double, double>> &points,
                       const std::wstring &xLabel, const std::wstring &yLabel, bool residual) {
    std::vector<Point> curve, plotted;
    if (!residual) {
        for (int index = 0; index <= 180; ++index) {
            double fraction = index / 180.;
            double x = std::fma(1 - fraction, fit.minimumX, fraction * fit.maximumX);
            curve.push_back({x, fit.predict(x)});
        }
    }
    double minimumY = std::numeric_limits<double>::infinity(), maximumY = -minimumY;
    for (std::size_t index = 0; index < points.size(); ++index) {
        plotted.push_back({points[index].first, residual ? fit.residuals[index] : points[index].second});
    }
    for (const auto &point : plotted) {
        minimumY = std::min(minimumY, point.y);
        maximumY = std::max(maximumY, point.y);
    }
    for (const auto &point : curve) {
        minimumY = std::min(minimumY, point.y);
        maximumY = std::max(maximumY, point.y);
    }
    if (residual) {
        minimumY = std::min(minimumY, 0.0);
        maximumY = std::max(maximumY, 0.0);
    }
    float height = residual ? 270.f : std::clamp(p.w * .39f, 310.f, 390.f);
    Axis ya(yLabel, minimumY, maximumY, 5, true);
    auto measure = [&](const std::wstring &text) { return p.app.textWidth(text, 12); };
    float left = 23;
    for (const auto &tick : ya.ticks) {
        left = std::max(left, measure(tick.label) + 23);
    }
    float available = std::max(1.f, p.w - left - 32);
    int intervals = std::clamp(int(std::floor(available / 88)), 2, 9);
    Axis xa(xLabel, fit.minimumX, fit.maximumX, intervals, false);
    auto labelWidth = [&] {
        float width = 0;
        for (const auto &tick : xa.ticks) {
            width = std::max(width, measure(tick.label));
        }
        return width;
    };
    float tickWidth = labelWidth();
    int adjusted = std::max(2, int(std::floor(available / (tickWidth + 24))));
    if (adjusted < intervals) {
        xa = Axis(xLabel, fit.minimumX, fit.maximumX, adjusted, false);
        tickWidth = labelWidth();
    }
    left = std::max(left, tickWidth / 2 + 12);
    float right = std::max(24.f, tickWidth / 2 + 12);
    Rect plot{p.x + left, p.y + 43, std::max(1.f, p.w - left - right), height - 97};
    p.ui.fill({p.x, p.y, p.w, height}, White, 7);
    p.ui.border({p.x, p.y, p.w, height}, Border, 7);
    for (const auto &tick : ya.ticks) {
        float y = plot.y + plot.h * (1 - float(ya.tickFraction(tick.value)));
        float width = measure(tick.label);
        p.ui.line(plot.x, y, plot.x + plot.w, y, Border);
        p.ui.text({plot.x - 9 - width, y - 12, width + 1, 24}, tick.label, 12, Muted);
    }
    for (const auto &tick : xa.ticks) {
        float x = plot.x + plot.w * float(xa.tickFraction(tick.value));
        p.ui.line(x, plot.y, x, plot.y + plot.h, Border);
        p.ui.text({x - tickWidth / 2 - 2, plot.y + plot.h + 10, tickWidth + 4, 24}, tick.label,
                  12, Muted, false, true);
    }
    p.ui.line(plot.x, plot.y, plot.x, plot.y + plot.h, Muted, 1.2f);
    p.ui.line(plot.x, plot.y + plot.h, plot.x + plot.w, plot.y + plot.h, Muted, 1.2f);
    p.ui.text({plot.x, p.y + 7, plot.w, 24}, ya.caption, 12, Ink);
    p.ui.text({plot.x, p.y + height - 28, plot.w, 24}, xa.caption, 12, Ink, false, true);

    auto at = [&](Point point) {
        return Point{plot.x + plot.w * xa.fraction(point.x),
                     plot.y + plot.h * (1 - ya.fraction(point.y))};
    };
    auto oldClip = p.ui.clip;
    p.ui.clip = {std::max(oldClip.x, plot.x), std::max(oldClip.y, plot.y),
                 std::max(0.f, std::min(oldClip.x + oldClip.w, plot.x + plot.w) - std::max(oldClip.x, plot.x)),
                 std::max(0.f, std::min(oldClip.y + oldClip.h, plot.y + plot.h) - std::max(oldClip.y, plot.y))};
    if (residual) {
        float zero = float(at({fit.minimumX, 0}).y);
        p.ui.line(plot.x, zero, plot.x + plot.w, zero, Blue, 1.4f);
    } else {
        for (std::size_t index = 1; index < curve.size(); ++index) {
            auto a = at(curve[index - 1]), b = at(curve[index]);
            p.ui.line(float(a.x), float(a.y), float(b.x), float(b.y), Blue, 2.2f);
        }
    }
    for (const auto &point : plotted) {
        auto position = at(point);
        p.ui.fill({float(position.x) - 5, float(position.y) - 5, 10, 10}, White, 5);
        p.ui.fill({float(position.x) - 3.6f, float(position.y) - 3.6f, 7.2f, 7.2f}, Red, 3.6f);
    }
    // 同页存在拟合图和残差图时，将两者都计入鼠标刷新范围。
    auto &hoverArea = p.app.chartArea;
    if (hoverArea.w <= 0 || hoverArea.h <= 0) {
        hoverArea = plot;
    } else {
        float x = std::min(hoverArea.x, plot.x), y = std::min(hoverArea.y, plot.y);
        hoverArea = {x, y, std::max(hoverArea.x + hoverArea.w, plot.x + plot.w) - x,
                           std::max(hoverArea.y + hoverArea.h, plot.y + plot.h) - y};
    }
    if (plot.contains(p.app.mouseX, p.app.mouseY) && oldClip.contains(p.app.mouseX, p.app.mouseY)) {
        std::size_t closest = 0;
        double distance = std::numeric_limits<double>::infinity();
        for (std::size_t index = 0; index < plotted.size(); ++index) {
            auto position = at(plotted[index]);
            double current = std::hypot(position.x - p.app.mouseX, position.y - p.app.mouseY);
            if (current < distance) {
                closest = index;
                distance = current;
            }
        }
        std::vector<std::wstring> labels;
        if (distance < 12) {
            labels = {L"第 " + std::to_wstring(closest + 1) + L" 组",
                      L"x = " + format(points[closest].first), L"y = " + format(points[closest].second),
                      L"拟合 y = " + format(fit.fitted[closest]), L"残差 = " + format(fit.residuals[closest])};
        } else if (!residual) {
            double fraction = (p.app.mouseX - plot.x) / plot.w;
            double x = std::fma(1 - fraction, fit.minimumX, fraction * fit.maximumX);
            double y = fit.predict(x);
            auto position = at({x, y});
            p.ui.line(float(position.x), plot.y, float(position.x), plot.y + plot.h, Muted);
            p.ui.fill({float(position.x) - 4, float(position.y) - 4, 8, 8}, Blue, 4);
            labels = {L"x = " + format(x), L"拟合 y = " + format(y)};
        }
        if (!labels.empty()) {
            float width = std::min(300.f, plot.w - 16);
            p.ui.fill({plot.x + 8, plot.y + 8, width, float(labels.size()) * 22 + 12}, Soft, 5);
            for (std::size_t index = 0; index < labels.size(); ++index) {
                p.ui.text({plot.x + 16, plot.y + 12 + float(index) * 22, width - 16, 22}, labels[index], 12, Blue);
            }
        }
    }
    p.ui.clip = oldClip;
    p.y += height + 8;
    p.ui.line(p.x, p.y + 12, p.x + 20, p.y + 12, Blue, 2.2f);
    p.ui.text({p.x + 27, p.y, 142, 24}, residual ? L"零残差参考线" : L"拟合曲线", 12, Ink);
    p.ui.fill({p.x + 183, p.y + 8, 7, 7}, Red, 3.5f);
    p.ui.text({p.x + 198, p.y, p.w - 198, 24}, L"校准数据点", 12, Ink);
    p.y += 34;
}
} // namespace toolbox
