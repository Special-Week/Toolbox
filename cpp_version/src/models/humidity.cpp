// 湿空气模型：压力以 Pa 计算，四种已知参数共用饱和压和湿球方程。
#include "toolbox/models.hpp"
namespace toolbox {
double saturation(double t) {
    require(std::isfinite(t) && t >= -100 && t <= 100, "环境及湿球温度须在 −100～100 °C 内");
    if (t <= 0) {
        return 611.2 * std::exp(22.46 * t / (272.62 + t));
    }
    if (t < 1) {
        return 611.2 * std::exp(17.62 * t / (243.12 + t));
    }
    return std::pow(10., (10.286 * (t + 273.15) - 2148.4909) / (t + 273.15 - 35.85));
}
Humidity humidity(double t, double p, int known, double value) {
    finite({t, p, value});
    require(p > 0, "大气压必须大于 0");
    Humidity h{};
    h.saturated = saturation(t);
    switch (known) {
    case 1:
        h.vapor = saturation(value);
        break;
    case 2:
        h.vapor = saturation(value) - .000662 * p * (t - value);
        // 极干、低温时两项相减会放大温度末位舍入。仅在湿球入口消除该数值噪声。
        if (std::abs(h.vapor) <=
            16 * std::numeric_limits<double>::epsilon() *
                (saturation(value) + .000662 * p * std::max({1., std::abs(t), std::abs(value)}))) {
            h.vapor = 0;
        }
        break;
    case 3: {
        require(value >= 0, "混合比不能为负");
        double q = value / 1000 * 28.959 / 18.0152;
        h.vapor = p * q / (1 + q);
        break;
    }
    default:
        h.vapor = h.saturated * value / 100;
        break;
    }
    h.rh = h.vapor / h.saturated * 100;
    if (std::abs(h.rh) < 1e-10) {
        h.rh = 0;
        h.vapor = 0;
    }
    if (std::abs(h.rh - 100) < 1e-8) {
        h.rh = 100;
    }
    require(h.rh >= 0 && h.rh <= 100 && h.vapor < p,
            "相对湿度应在 0～100%，水汽分压应小于大气压；露霜点及湿球不高于环境温度");
    if (h.vapor > 0) {
        double z = std::log(h.vapor / 611.2), l = std::log10(h.vapor);
        h.dew = h.vapor < 657 ? 243.12 * z / (17.62 - z) : (-2148.4909 + l * 35.85) / (l - 10.286) - 273.15;
        h.frost = 272.62 * z / (22.46 - z);
    }
    auto wetEquation = [&](double wet) { return saturation(wet) - .000662 * p * (t - wet); };
    if (h.vapor >= wetEquation(-100)) {
        h.wet = inverse(wetEquation, h.vapor, -100, t);
    }
    h.volumeFraction = h.vapor / p;
    h.ppm = h.vapor / (p - h.vapor) * 1e6;
    h.mass = h.ppm * 18.0152 / 28.959;
    h.absolute = h.vapor * 18.015 / ((t + 273.15) * 8.31451);
    h.normal = h.absolute * (t + 273.15) * 101325 / (273.15 * p);
    h.specific = h.mass / (1e6 + h.mass) * 1000;
    h.mixing = h.mass / 1000;
    finite({h.ppm, h.mass, h.absolute, h.normal});
    return h;
}
} // namespace toolbox
