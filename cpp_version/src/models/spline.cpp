// 自然端点三次均匀参数 B 样条；先反求 x(u)，再计算 y(u)。
#include "toolbox/models.hpp"
namespace toolbox {
static constexpr double eps = std::numeric_limits<double>::epsilon();
double Spline::Cubic::value(double u) const { return std::fma(std::fma(std::fma(d, u, c), u, b), u, a); }
double Spline::Cubic::derivative(double u) const { return std::fma(std::fma(3 * d, u, 2 * c), u, b); }
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
    // 横坐标误差小不代表参数收敛；水平切线附近继续缩小参数括区间。
    for (int i = 0; i < 1200; ++i) {
        double error = p.value(u) - target;
        if (!std::isfinite(error)) {
            break;
        }
        if (error == 0) {
            return u;
        }
        (error < 0 ? lo : hi) = u;
        double candidate = u - error / p.derivative(u);
        if (candidate == u)
            return u;
        double midpoint = lo + (hi - lo) / 2;
        if (midpoint == lo || midpoint == hi)
            return std::abs(p.value(lo) - target) <= std::abs(p.value(hi) - target) ? lo : hi;
        u = std::isfinite(candidate) && candidate > lo && candidate < hi ? candidate : midpoint;
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
std::vector<std::wstring> Spline::normalizationProcess(bool full) const {
    auto c = [full](double value) { return processNumber(value, full); };
    std::size_t farthest = 0;
    double difference = 0;
    for (std::size_t i = 0; i < points_.size(); ++i) {
        double current = std::abs(points_[i].y - origin_.y);
        if (current > difference) {
            difference = current;
            farthest = i;
        }
    }
    std::vector<std::wstring> lines{
        L"归一化基准为首个型值点 Q₀：x₀ = " + inputNumber(origin_.x) + L"，y₀ = " + inputNumber(origin_.y) + L"。",
        L"横坐标缩放因子 sₓ = x末 − x₀ = " + inputNumber(points_.back().x) + L" − " + inputNumber(origin_.x) + L" = " + c(scale_.x) + L"。"};
    lines.push_back(difference == 0
        ? L"所有纵坐标都等于 y₀，maxᵢ|yᵢ − y₀| = 0；程序将 sᵧ 设为 1，避免除以零。"
        : L"纵坐标缩放因子 sᵧ = maxᵢ|yᵢ − y₀|；最大差值来自 Q" + std::to_wstring(farthest) + L"：|" + inputNumber(points_[farthest].y) + L" − " + inputNumber(origin_.y) + L"| = " + c(difference) + L"，所以 sᵧ = " + c(scale_.y) + L"。");
    lines.push_back(L"各型值点归一化：Xᵢ = (xᵢ − x₀)/sₓ，Yᵢ = (yᵢ − y₀)/sᵧ；计算后按 y = y₀ + sᵧY 恢复纵坐标。");
    return lines;
}
std::vector<std::wstring> Spline::processAt(double x, bool full) const {
    auto c = [full](double value) { return processNumber(value, full); };
    auto found = std::lower_bound(points_.begin(), points_.end(), x,
                                  [](Point p, double value) { return p.x < value; });
    if (found != points_.end() && found->x == x)
        return {L"x = " + inputNumber(x) + L" 恰为 Q" + std::to_wstring(found - points_.begin()) + L" 的横坐标，直接返回原始 y = " + inputNumber(found->y) + L"。"};
    double target = (x - origin_.x) / scale_.x;
    Segment segment;
    double lo = 0, hi = 1;
    std::wstring branch;
    if (found == points_.begin() || found == points_.end()) {
        bool right = found == points_.end();
        segment = endpoint(right);
        auto bracket = externalBracket(segment.x, target, right);
        lo = bracket.first;
        hi = bracket.second;
        branch = right ? L"右端外推" : L"左端外推";
    } else {
        auto index = std::size_t(found - points_.begin() - 1);
        segment = segments_[index];
        branch = L"Q" + std::to_wstring(index) + L"～Q" + std::to_wstring(index + 1) + L" 插值";
    }
    double u = invert(segment.x, target, lo, hi), normalized = segment.y.value(u);
    double result = std::fma(normalized, scale_.y, origin_.y);
    return {
        L"x = " + inputNumber(x) + L"；" + branch + L"；目标归一化横坐标 X = (x − x₀)/sₓ = (" + inputNumber(x) + L" − " + inputNumber(origin_.x) + L")/" + c(scale_.x) + L" = " + c(target) + L"。",
        L"X(u) = a + bu + cu² + du³；本段系数 a、b、c、d = " + inputNumber(segment.x.a) + L"、" + inputNumber(segment.x.b) + L"、" + inputNumber(segment.x.c) + L"、" + inputNumber(segment.x.d) + L"；在 [" + inputNumber(lo) + L"，" + inputNumber(hi) + L"] 内用带括区间的 Newton 法反求，必要时二分，得到 u = " + c(u) + L"。",
        L"Y(u) = a + bu + cu² + du³；本段系数 a、b、c、d = " + inputNumber(segment.y.a) + L"、" + inputNumber(segment.y.b) + L"、" + inputNumber(segment.y.c) + L"、" + inputNumber(segment.y.d) + L"。",
        L"恢复坐标 y = y₀ + sᵧY(u) = " + inputNumber(origin_.y) + L" + " + c(scale_.y) + L" × " + c(normalized) + L" = " + c(result) + L"；多项式及恢复坐标使用融合乘加。"};
}
static std::size_t boundedSampleCount(double lo, double hi, double step, std::size_t limit,
                                      const char *limitError) {
    finite({lo, hi, step});
    require(hi > lo && step > 0, "曲线最小 x 必须小于最大 x，步进必须大于 0");
    double n = (hi - lo) / step;
    require(std::isfinite(n) && n <= double(limit), limitError);
    double nearest = std::round(n);
    if (std::abs(n - nearest) <= 8 * eps * std::max(n, 1.)) {
        n = nearest;
    }
    auto count = std::max<std::size_t>(2, std::size_t(std::ceil(n)) + 1);
    if (count > 2 && sampleX(lo, hi, step, count, count - 2) >= hi) {
        --count;
    }
    require(count <= limit, limitError);
    double resolution = std::max(std::nextafter(lo, hi) - lo, hi - std::nextafter(hi, lo));
    double remainder = std::abs(std::fmod(lo, resolution));
    bool ties = step == resolution && remainder > 0 && remainder == resolution * .5;
    if (count > 2 && (step < resolution || ties)) {
        const double infinity = std::numeric_limits<double>::infinity();
        bool reverse = lo >= 0;
        double previous = reverse ? hi : lo;
        for (std::size_t offset = 1; offset < count; ++offset) {
            auto index = reverse ? count - 1 - offset : offset;
            double x = sampleX(lo, hi, step, count, index);
            require(reverse ? x < previous : x > previous, "采样步进小于当前坐标的可分辨精度，请增大步进");
            // 向零检查到更细的浮点档位后，剩余采样点无需逐个核对。
            if ((reverse && step > std::nextafter(x, infinity) - x) ||
                (hi <= 0 && step > x - std::nextafter(x, -infinity))) {
                break;
            }
            previous = x;
        }
    }
    return count;
}
std::size_t sampleCount(double lo, double hi, double step) {
    return boundedSampleCount(lo, hi, step, 5000000, "采样点最多 5000000 个，请增大步进或缩小范围");
}

double sampleX(double lo, double hi, double step, std::size_t count, std::size_t index) {
    return index + 1 == count ? hi : std::fma(double(index), step, lo);
}
Values divisionPoints(double start, double end, double step) {
    auto count = boundedSampleCount(start, end, step, 50000, "分度表最多 50000 行，请增大步长");
    Values p(count);
    for (std::size_t i = 0; i < count; ++i) {
        p[i] = sampleX(start, end, step, count, i);
    }
    return p;
}
} // 命名空间 toolbox
