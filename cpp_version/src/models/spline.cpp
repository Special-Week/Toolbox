// 自然端点三次均匀参数 B 样条；先反求 x(u)，再计算 y(u)。
#include "toolbox/models.hpp"
namespace toolbox {
static constexpr double eps = std::numeric_limits<double>::epsilon();
double Spline::Cubic::value(double u) const {
    return std::fma(std::fma(std::fma(d, u, c), u, b), u, a);
}
double Spline::Cubic::derivative(double u) const {
    return std::fma(std::fma(3 * d, u, 2 * c), u, b);
}
double Spline::Cubic::minimumDerivative() const {
    double m = std::min(derivative(0), derivative(1));
    if (d > 0) {
        double v = -c / (3 * d);
        if (v > 0 && v < 1) {
            m = std::min(m, derivative(v));
        }
    }
    return m;
}
Spline::Cubic Spline::coefficients(const std::array<double, 4> &p, double value) {
    double d0 = p[1] - p[0], d1 = p[2] - p[1], d2 = p[3] - p[2];
    double c = (d1 - d0) / 2, d = (d2 - 2 * d1 + d0) / 6, max = 0;
    for (double v : p) {
        max = std::max(max, std::abs(v));
    }
    if (std::abs(c) <= 32 * eps * max) {
        c = 0;
    }
    if (std::abs(d) <= 32 * eps * max) {
        d = 0;
    }
    return {value, (p[2] - p[0]) / 2, c, d};
}
Spline::Spline(const std::vector<Point> &points) : points_(points) {
    require(points.size() >= 2 && points.size() <= 500, "Q 型值点数量必须为 2～500");
    for (std::size_t i = 0; i < points.size(); ++i) {
        finite({points[i].x, points[i].y});
        require(i == 0 || points[i].x > points[i - 1].x, "Q 点的 x 必须严格递增");
    }
    origin_ = points.front();
    scale_ = {points.back().x - origin_.x, 0};
    for (auto p : points) {
        scale_.y = std::max(scale_.y, std::abs(p.y - origin_.y));
    }
    if (scale_.y == 0) {
        scale_.y = 1;
    }
    finite({scale_.x, scale_.y});
    require(scale_.x > 0, "坐标跨度无效");
    std::vector<Point> q;
    for (auto p : points) {
        q.push_back({(p.x - origin_.x) / scale_.x, (p.y - origin_.y) / scale_.y});
    }
    const std::size_t n = q.size(), unknown = n - 2;
    std::vector<Point> control(n + 2), rhs(unknown);
    Values diagonal(unknown, 4);
    control[1] = q.front();
    control[n] = q.back();
    for (std::size_t i = 0; i < unknown; ++i) {
        rhs[i] = {6 * q[i + 1].x, 6 * q[i + 1].y};
        if (i == 0) {
            rhs[i].x -= q.front().x;
            rhs[i].y -= q.front().y;
        }
        if (i + 1 == unknown) {
            rhs[i].x -= q.back().x;
            rhs[i].y -= q.back().y;
        }
        if (i > 0) {
            double f = 1 / diagonal[i - 1];
            diagonal[i] -= f;
            rhs[i].x -= f * rhs[i - 1].x;
            rhs[i].y -= f * rhs[i - 1].y;
        }
    }
    for (std::size_t k = unknown; k > 0; --k) {
        std::size_t i = k - 1;
        Point next = i + 1 == unknown ? Point{} : control[i + 3];
        control[i + 2] = {(rhs[i].x - next.x) / diagonal[i], (rhs[i].y - next.y) / diagonal[i]};
    }
    control[0] = {2 * control[1].x - control[2].x, 2 * control[1].y - control[2].y};
    control[n + 1] = {2 * control[n].x - control[n - 1].x, 2 * control[n].y - control[n - 1].y};
    for (std::size_t i = 0; i + 3 < control.size(); ++i) {
        Segment s{coefficients({control[i].x, control[i + 1].x, control[i + 2].x, control[i + 3].x}, q[i].x),
                  coefficients({control[i].y, control[i + 1].y, control[i + 2].y, control[i + 3].y}, q[i].y)};
        require(s.x.minimumDerivative() >= -32 * eps, "Q 点之间的 x 出现回折，无法唯一计算 y(x)");
        segments_.push_back(s);
    }
}
Spline::Segment Spline::endpoint(bool right) const {
    auto s = right ? segments_.back() : segments_.front();
    auto p = right ? points_.back() : origin_;
    double u = right ? 1 : 0;
    return {{(p.x - origin_.x) / scale_.x, s.x.derivative(u), 0, s.x.d},
            {(p.y - origin_.y) / scale_.y, s.y.derivative(u), 0, s.y.d}};
}
std::pair<double, double> Spline::externalBracket(Cubic p, double target, bool right) {
    require(p.b >= 0 && (p.b != 0 || p.d > 0), "端点外推到达 x 转折点，无法唯一反求参数");
    double limit = p.d < 0 ? std::sqrt(-p.b / (3 * p.d)) : std::numeric_limits<double>::infinity();
    double distance = std::min(1., limit), sign = right ? 1 : -1;
    for (int i = 0; i < 1024; ++i) {
        double bound = sign * distance, value = p.value(bound);
        finite({value});
        if ((right && value >= target) || (!right && value <= target)) {
            return right ? std::make_pair(0., bound) : std::make_pair(bound, 0.);
        }
        require(distance != limit, "端点外推到达 x 转折点");
        distance = std::min(distance * 2, limit);
    }
    throw std::runtime_error("外推范围过大");
}
double Spline::invert(Cubic p, double target, double lo, double hi) {
    double low = p.value(lo), span = p.value(hi) - low;
    require(std::isfinite(span) && span > 0, "样条参数区间无效");
    double u = lo + (hi - lo) * ((target - low) / span);
    if (!std::isfinite(u) || u <= lo || u >= hi) {
        u = lo + (hi - lo) / 2;
    }
    for (int i = 0; i < 80; ++i) {
        double error = p.value(u) - target;
        if (!std::isfinite(error)) {
            break;
        }
        if (std::abs(error) <= 8 * eps * std::max(std::abs(target), 1.)) {
            return u;
        }
        (error < 0 ? lo : hi) = u;
        double candidate = u - error / p.derivative(u);
        u = std::isfinite(candidate) && candidate > lo && candidate < hi ? candidate : lo + (hi - lo) / 2;
    }
    throw std::runtime_error("反求样条参数未收敛，请检查 x 间距或外推范围");
}
double Spline::y(double x) const {
    finite({x});
    auto found = std::lower_bound(points_.begin(), points_.end(), x,
                                  [](Point p, double value) { return p.x < value; });
    if (found != points_.end() && found->x == x) {
        return found->y;
    }
    double target = (x - origin_.x) / scale_.x;
    finite({target});
    Segment s;
    double lo = 0, hi = 1;
    if (found == points_.begin() || found == points_.end()) {
        bool right = found == points_.end();
        s = endpoint(right);
        auto b = externalBracket(s.x, target, right);
        lo = b.first;
        hi = b.second;
    } else {
        s = segments_[std::size_t(found - points_.begin() - 1)];
    }
    double result = std::fma(s.y.value(invert(s.x, target, lo, hi)), scale_.y, origin_.y);
    finite({result});
    return result;
}
std::size_t sampleCount(double lo, double hi, double step) {
    finite({lo, hi, step});
    require(hi > lo && step > 0, "曲线最小 x 必须小于最大 x，步进必须大于 0");
    double n = (hi - lo) / step;
    require(std::isfinite(n), "采样点最多 5000000 个");
    double nearest = std::round(n);
    if (std::abs(n - nearest) <= 8 * eps * std::max(n, 1.)) {
        n = nearest;
    }
    require(n <= 4999999, "采样点最多 5000000 个，请增大步进或缩小范围");
    double resolution = std::max(std::nextafter(lo, hi) - lo, hi - std::nextafter(hi, lo));
    require(n <= 1 || step >= resolution, "采样步进小于当前坐标的可分辨精度，请增大步进");
    return std::max<std::size_t>(2, std::size_t(std::ceil(n)) + 1);
}
double sampleX(double lo, double hi, double step, std::size_t count, std::size_t index) {
    return index + 1 == count ? hi : std::fma(double(index), step, lo);
}
Values divisionPoints(double start, double end, double step) {
    finite({start, end, step});
    require(start < end && step > 0, "起点须小于终点，步长须大于 0");
    double count = std::floor((end - start) / step);
    require(std::isfinite(count) && count <= 49999, "分度表最多 50000 行，请增大步长");
    Values p;
    for (std::size_t i = 0; i <= std::size_t(count); ++i) {
        double x = start + double(i) * step;
        require(p.empty() || p.back() < x, "步长小于当前数值的可分辨精度");
        if (x <= end) {
            p.push_back(x);
        }
    }
    if (p.empty() || std::abs(p.back() - end) > 1e-10 * std::max(std::abs(end), 1.)) {
        p.push_back(end);
    } else {
        p.back() = end;
    }
    require(p.size() <= 50000, "分度表最多 50000 行");
    return p;
}
} // namespace toolbox
