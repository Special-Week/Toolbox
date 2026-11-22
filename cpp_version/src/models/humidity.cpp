// 湿空气模型：压力以 Pa 计算，四种已知参数共用饱和压和湿球方程。
#include "toolbox/models.hpp"
namespace toolbox {
static double liquidSaturation(double t) {
    double kelvin = t + 273.15;
    return std::pow(10., (10.286 * kelvin - 2148.4909) / (kelvin - 35.85));
}
double humidityLowWaterCoefficient() {
    // 两段水面公式在 1 °C 连续，0 °C 处仍为 611.2 Pa。
    return 244.12 * std::log(liquidSaturation(1) / 611.2);
}
double saturation(double t) {
    require(std::isfinite(t) && t >= -100 && t <= 100, "环境及湿球温度须在 −100～100 °C 内");
    if (t <= 0) {
        return 611.2 * std::exp(22.46 * t / (272.62 + t));
    }
    if (t < 1) {
        return 611.2 * std::exp(humidityLowWaterCoefficient() * t / (243.12 + t));
    }
    return liquidSaturation(t);
}
Humidity humidity(double t, double p, int known, double value) {
    finite({t, p, value});
    require(p > 0, "大气压必须大于 0");
    if (known != 1 && known != 2 && known != 3)
        require(value >= 0 && value <= 100, "相对湿度必须在 0%～100% 内");
    Humidity h{};
    h.saturated = saturation(t);
    switch (known) {
    case 1:
        h.vapor = saturation(value);
        break;
    case 2:
        h.vapor = saturation(value) - .000662 * p * (t - value);
        break;
    case 3: {
        require(value >= 0, "混合比不能为负");
        double q = value / 1000 * 28.959 / 18.0152;
        h.vapor = p * q / (1 + q);
        break;
    }
    default:
        h.vapor = value == 100 ? h.saturated : h.saturated * value / 100;
        break;
    }
    h.rawVapor = h.vapor;
    double rawRh = h.rawVapor * 100 / h.saturated;
    double wetRoundoff = known == 2 ? .000662 * p * (std::max({std::abs(t), std::abs(value), 1.}) * (2 * std::numeric_limits<double>::epsilon())) : 0;
    if (h.rawVapor < 0 && (rawRh >= -1e-10 || h.rawVapor >= -wetRoundoff)) {
        h.vapor = 0;
    } else if (rawRh > 100 && rawRh <= 100 + 1e-8) {
        h.vapor = h.saturated;
    }
    h.rh = h.vapor == 0 ? 0 : h.vapor == h.saturated ? 100 : h.vapor * 100 / h.saturated;
    require(h.rh >= 0 && h.rh <= 100 && h.vapor < p,
            "相对湿度应在 0～100%，水汽分压应小于大气压；露霜点及湿球不高于环境温度");
    if (h.vapor > 0) {
        double z = std::log(h.vapor) - std::log(611.2), l = std::log10(h.vapor);
        if (h.vapor < liquidSaturation(1)) {
            double coefficient = h.vapor <= 611.2 ? 17.62 : humidityLowWaterCoefficient();
            h.dew = 243.12 * z / (coefficient - z);
        } else {
            h.dew = (-2148.4909 + l * 35.85) / (l - 10.286) - 273.15;
        }
        h.frost = 272.62 * z / (22.46 - z);
    }
    auto wetEquation = [&](double wet) { return saturation(wet) - .000662 * p * (t - wet); };
    if (h.vapor == h.saturated) {
        h.wet = t;
    } else if (h.vapor >= wetEquation(-100)) {
        h.wet = inverse(wetEquation, h.vapor, -100, t);
    }
    h.volumeFraction = h.vapor / p;
    h.ppm = h.vapor / (p - h.vapor) * 1e6;
    h.mass = h.ppm * 18.0152 / 28.959;
    h.absolute = h.vapor * 18.015 / ((t + 273.15) * 8.31451);
    h.normal = h.absolute * (t + 273.15) * 101325 / (273.15 * p);
    h.specific = h.mass / (1e6 + h.mass) * 1000;
    h.mixing = h.mass / 1000;
    finite({h.wet.value_or(0), h.dew.value_or(0), h.frost.value_or(0), h.ppm, h.mass, h.absolute, h.normal});
    return h;
}
} // 命名空间 toolbox
