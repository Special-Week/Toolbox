// 压力及变送器量程模型；输入准确度使用百分数，内部保持 double 精度。
#include "toolbox/models.hpp"
#include <cstdint>
#include <initializer_list>
namespace toolbox {
Range::Range(double lower, double upper) : low(lower), high(upper), span(upper - lower) {
    finite({low, high, span});
    require(span > 0, "量程上限必须大于下限");
}
bool Range::covers(const Range &other) const { return low <= other.low && high >= other.high; }
Selection selectStandard(Range dut, Range reference, double da, double ra, double required, bool piston) {
    finite({da, ra, required});
    require(da > 0 && ra > 0 && required > 0, "准确度和量传比须大于 0");
    double dt = dut.span * (da / 100),
           rt = (piston ? std::max(dut.span, .1 * reference.span) : reference.span) * (ra / 100);
    require(dt > 0 && rt > 0, "允差低于可表示范围");
    double ratio = dt / rt;
    finite({dt, rt, ratio});
    bool coverage = reference.covers(dut);
    bool meetsRatio = ratio >= required || required - ratio <= required * (4 * std::numeric_limits<double>::epsilon());
    return {dt, rt, ratio, coverage, coverage && meetsRatio};
}
static double productRatio(double a, double b, double c, double d) {
    int ea, eb, ec, ed;
    a = std::frexp(a, &ea);
    b = std::frexp(b, &eb);
    c = std::frexp(c, &ec);
    d = std::frexp(d, &ed);
    return std::scalbn((a * b) / (c * d), ea + eb - ec - ed);
}

ElectricalSelection selectTransmitter(Range dut, Range reference, Range output, double da, double ra,
                                      double reading, double fixed, double required) {
    finite({reading, fixed});
    require(reading >= 0 && fixed >= 0, "电测读数误差与固定误差不得为负");
    auto p = selectStandard(dut, reference, da, ra, required);
    const double pressurePerUnit = p.referenceTolerance / dut.span;
    const double pressure = std::isnormal(pressurePerUnit) ? pressurePerUnit * output.span
        : productRatio(p.referenceTolerance, output.span, dut.span, 1.);
    // 双极性信号的最坏读数项取两端绝对值的较大者，不能只看输出上限。
    const double peak = std::max(std::abs(output.low), std::abs(output.high));
    const double readingFraction = reading / 100;
    const double electrical = fixed + (reading == 0 || std::isnormal(readingFraction) ? readingFraction * peak
        : productRatio(reading, peak, 100., 1.));
    const double combined = std::hypot(pressure, electrical), doubled = 2 * combined;
    const double expanded = std::isfinite(doubled) ? doubled / std::sqrt(3.) : combined * (2. / std::sqrt(3.));
    const double unscaledAllowance = output.span * da;
    const double allowance = std::isnormal(unscaledAllowance) && std::isnormal(unscaledAllowance / 100.)
        ? unscaledAllowance / 100. / required : productRatio(output.span, da, 100., required);
    finite({expanded, allowance});
    return {p, expanded, allowance, p.meets && expanded < allowance};
}
double hydrostatic(double height, double density, double gravity) {
    finite({height, density, gravity});
    require(density > 0 && gravity > 0, "密度与重力加速度须大于 0");
    double dp = density * gravity * height;
    finite({dp});
    return dp;
}
double gasDifference(double height, double normalDensity, double gravity, double t, double p, double ambient,
                     bool gauge) {
    finite({height, normalDensity, gravity, t, p, ambient});
    double absolute = p + (gauge ? ambient : 0);
    require(absolute > 0 && ambient > 0 && t > -273.15, "绝对压力和环境压力须大于 0，温度须高于绝对零度");
    double density = normalDensity * (absolute / 101325) * 273.15 / (t + 273.15);
    double dp = hydrostatic(height, density, gravity);
    return gauge ? dp - (ambient - atmospherePressure(atmosphereAltitude(ambient) + height)) : dp;
}
enum class TransmitterPower { Linear, Root, Square };

static double scaledTransmitterStep(double delta, double sourceSpan, double targetSpan,
                                    TransmitterPower power, double factor = 1) {
    int deltaExponent, sourceExponent, targetExponent;
    const double numerator = std::frexp(delta, &deltaExponent);
    const double denominator = std::frexp(sourceSpan, &sourceExponent);
    const double target = std::frexp(targetSpan, &targetExponent);
    double ratio = numerator / denominator;
    int exponent = deltaExponent - sourceExponent;
    if (power == TransmitterPower::Root) {
        if (exponent % 2 != 0) {
            ratio *= 2;
            --exponent;
        }
        ratio = std::sqrt(ratio);
        exponent /= 2;
    } else if (power == TransmitterPower::Square) {
        ratio *= ratio;
        exponent *= 2;
    }
    return std::scalbn(ratio * target * factor, exponent + targetExponent);
}

static double transmitterFromUpper(double value, Range source, Range target, double factor) {
    const double distance = value - source.high;
    const double slope = (target.span / source.span) * factor;
    if (std::isnormal(slope))
        return std::fma(distance, slope, target.high);
    return distance == 0 ? target.high : target.high -
        scaledTransmitterStep(-distance, source.span, target.span, TransmitterPower::Linear, factor);
}

bool transmitterUsesMidpoint(double fraction) {
    return fraction >= 0.25 && fraction <= 0.75;
}

bool transmitterCrossesZero(Range range) {
    return range.low < 0 && range.high > 0;
}

static std::pair<double, double> sumWithError(double a, double b) {
    const double sum = a + b;
    const double part = sum - a;
    return {sum, (a - (sum - part)) + (b - part)};
}

struct ScaledProduct {
    double high, low;
    int exponent;

    static ScaledProduct product(double a, double b) {
        int ea, eb;
        a = std::frexp(a, &ea);
        b = std::frexp(b, &eb);
        const double high = a * b;
        return {high, std::fma(a, b, -high), ea + eb};
    }

    ScaledProduct add(ScaledProduct other) const {
        if (high == 0 && low == 0) return other;
        if (other.high == 0 && other.low == 0) return *this;
        const int common = std::max(exponent, other.exponent);
        const auto [main, mainError] = sumWithError(std::scalbn(high, exponent - common),
                                                   std::scalbn(other.high, other.exponent - common));
        const auto [tail, tailError] = sumWithError(std::scalbn(low, exponent - common),
                                                   std::scalbn(other.low, other.exponent - common));
        const auto [middle, middleError] = sumWithError(mainError, tail);
        const auto [sum, carry] = sumWithError(main, middle);
        const auto [leading, trailing] = sumWithError(sum, (carry + middleError) + tailError);
        return {leading, trailing, common};
    }

    ScaledProduct multiply(const ScaledProduct &other) const {
        const double product = high * other.high;
        const double error = std::fma(high, other.high, -product);
        const auto [leading, trailing] = sumWithError(product, error + high * other.low + low * other.high);
        return {leading, trailing, exponent + other.exponent};
    }

    ScaledProduct ratioRoot(const ScaledProduct &denominator) const {
        int ne, de;
        const double num = std::frexp(high + low, &ne);
        const double den = std::frexp(denominator.high + denominator.low, &de);
        const int difference = exponent + ne - denominator.exponent - de;
        const bool odd = difference % 2 != 0;
        return {std::sqrt((odd ? 2. : 1.) * (num / den)), 0., (difference - odd) / 2};
    }

    double divide(const ScaledProduct &denominator) const {
        int shift;
        const double leading = std::frexp(denominator.high, &shift);
        const double trailing = std::scalbn(denominator.low, -shift);
        const double quotient = high / leading;
        const double remainder = std::fma(-quotient, leading, high) + low - quotient * trailing;
        return std::scalbn(quotient + remainder / leading, exponent - denominator.exponent - shift);
    }
};

static double transmitterCrossingLinear(double value, Range source, Range target) {
    if (value == source.low) return target.low;
    if (value == source.high) return target.high;
    if (source.low == target.low && source.high == target.high) return value;

    // 分子为 x(d − c) + cb − da。先合并常数项，各乘积保留独立的二进制指数。
    const auto offset = ScaledProduct::product(target.low, source.high)
        .add(ScaledProduct::product(-target.high, source.low));
    const auto increment = ScaledProduct::product(value, target.high)
        .add(ScaledProduct::product(value, -target.low));
    const auto numerator = offset.add(increment);
    int exponent;
    const double denominator = std::frexp(source.span, &exponent);
    const double quotient = numerator.high / denominator;
    const double remainder = std::fma(-quotient, denominator, numerator.high) + numerator.low;
    return std::clamp(std::scalbn(quotient + remainder / denominator, numerator.exponent - exponent),
                      target.low, target.high);
}

struct TripleProduct {
    double a, b, c;
    int shift;
};

// 三个双精度尾数的乘积可用 12 位分段精确相加，跨零时不会留下舍入残差。
static ScaledProduct sumTripleProducts(std::initializer_list<TripleProduct> terms) {
    constexpr int bits = 12, minExponent = -3 * (1074 + 52);
    constexpr std::int64_t base = 1 << bits;
    std::array<std::int64_t, (3 * 1024 - minExponent) / bits + 3> digits{};
    const auto parts = [](double value) -> std::pair<std::uint64_t, int> {
        int exponent;
        const double mantissa = std::frexp(std::abs(value), &exponent);
        return {static_cast<std::uint64_t>(std::scalbn(mantissa, 53)), exponent - 53};
    };
    const auto chunk = [](std::uint64_t value, int index) {
        return static_cast<std::int64_t>((value >> (index * bits)) & (base - 1));
    };
    for (auto term : terms) {
        if (term.a == 0 || term.b == 0 || term.c == 0) continue;
        const int sign = (std::signbit(term.a) ^ std::signbit(term.b) ^ std::signbit(term.c)) ? -1 : 1;
        const auto [a, ae] = parts(term.a);
        const auto [b, be] = parts(term.b);
        const auto [c, ce] = parts(term.c);
        const int offset = ae + be + ce + term.shift - minExponent;
        for (int i = 0; i < 5; ++i) {
            for (int j = 0; j < 5; ++j) {
                for (int k = 0; k < 5; ++k) {
                    const auto product = chunk(a, i) * chunk(b, j) * chunk(c, k);
                    digits[offset / bits + i + j + k] += sign * (product << (offset % bits));
                }
            }
        }
    }
    const auto carry = [&] {
        for (std::size_t i = 0; i + 1 < digits.size(); ++i) {
            const auto remainder = static_cast<std::int64_t>(static_cast<std::uint64_t>(digits[i]) & (base - 1));
            digits[i + 1] += (digits[i] - remainder) / base;
            digits[i] = remainder;
        }
    };
    carry();
    const bool negative = digits.back() < 0;
    if (negative) {
        for (auto &digit : digits) digit = -digit;
        carry();
    }
    int end = static_cast<int>(digits.size()) - 1;
    while (end && digits[end] == 0) --end;
    double mantissa = 0;
    for (int i = std::max(0, end - 4); i <= end; ++i)
        mantissa = mantissa / base + static_cast<double>(digits[i]);
    return {negative ? -mantissa : mantissa, 0., minExponent + end * bits};
}

static double transmitterCrossingNonlinear(double value, Range source, Range target, bool forward) {
    if (value == source.low) return target.low;
    if (value == source.high) return target.high;
    const auto delta = ScaledProduct::product(value, 1.).add(ScaledProduct::product(-source.low, 1.));
    const auto inputSpan = ScaledProduct::product(source.high, 1.).add(ScaledProduct::product(-source.low, 1.));
    const auto outputSpan = ScaledProduct::product(target.high, 1.).add(ScaledProduct::product(-target.low, 1.));
    const double a = source.low, b = source.high, c = target.low, d = target.high, x = value;
    double result;
    if (forward) {
        // (c + R√f)(R√f − c) = R²f − c²，先计算分子里的微小差值。
        const auto numerator = sumTripleProducts({
            {d, d, x, 0}, {d, -c, x, 1}, {c, c, x, 0},
            {d, d, -a, 0}, {d, c, a, 1}, {c, c, -b, 0},
        });
        const auto conjugate = outputSpan.multiply(delta.ratioRoot(inputSpan))
            .add(ScaledProduct::product(-target.low, 1.));
        result = numerator.divide(inputSpan.multiply(conjugate));
    } else {
        const auto denominator = inputSpan.multiply(inputSpan);
        const auto numerator = sumTripleProducts({
            {d, x, x, 0}, {d, x, -a, 1}, {d, a, a, 0},
            {-c, x, x, 0}, {c, x, a, 1}, {c, b, b, 0}, {c, b, -a, 1},
        });
        result = numerator.divide(denominator);
    }
    return std::clamp(result, target.low, target.high);
}

static double transmitterFromMidpoint(double value, Range source, Range target) {
    const auto midpoint = [](Range range) {
        return range.low < 0 && range.high > 0 ? (range.low + range.high) * 0.5
                                               : range.low + range.span * 0.5;
    };
    const double targetMid = midpoint(target);
    const double distance = value - midpoint(source);
    const double slope = target.span / source.span;
    if (std::isnormal(slope))
        return std::fma(distance, slope, targetMid);
    return distance == 0 ? targetMid : targetMid + std::copysign(
        scaledTransmitterStep(std::abs(distance), source.span, target.span, TransmitterPower::Linear), distance);
}

std::pair<double, double> transmitterForward(Range process, Range output, bool root, double value) {
    finite({value});
    require(value >= process.low && value <= process.high, "输入值超出所选量程");
    double fraction = (value - process.low) / process.span;
    require(!root || fraction >= 0, "开方输入不能低于量程下限");
    double outputFraction = root ? std::sqrt(fraction) : fraction;
    double signal;
    if (transmitterCrossesZero(output)) {
        signal = root ? transmitterCrossingNonlinear(value, process, output, true)
                      : transmitterCrossingLinear(value, process, output);
    } else if (!root && transmitterUsesMidpoint(fraction)) {
        signal = transmitterFromMidpoint(value, process, output);
    } else if (fraction > 0.5) {
        signal = transmitterFromUpper(value, process, output, root ? 1 / (1 + outputFraction) : 1);
    } else if (value > process.low && fraction < std::numeric_limits<double>::min()) {
        signal = output.low + scaledTransmitterStep(value - process.low, process.span, output.span,
            root ? TransmitterPower::Root : TransmitterPower::Linear);
    } else {
        signal = std::fma(outputFraction, output.span, output.low);
    }
    finite({signal, fraction});
    return {signal, fraction};
}
std::pair<double, double> transmitterInverse(Range process, Range output, bool root, double value) {
    finite({value});
    require(value >= output.low && value <= output.high, "输入值超出所选量程");
    double fraction = (value - output.low) / output.span;
    require(!root || fraction >= 0, "开方输出不能低于信号下限");
    const double processFraction = root ? fraction * fraction : fraction;
    double processValue;
    if (transmitterCrossesZero(process)) {
        processValue = root ? transmitterCrossingNonlinear(value, output, process, false)
                            : transmitterCrossingLinear(value, output, process);
    } else if (!root && transmitterUsesMidpoint(fraction)) {
        processValue = transmitterFromMidpoint(value, output, process);
    } else if (fraction > 0.5) {
        const double deficit = (output.high - value) / output.span;
        processValue = transmitterFromUpper(value, output, process, root ? 2 - deficit : 1);
    } else if (value > output.low && processFraction < std::numeric_limits<double>::min()) {
        processValue = process.low + scaledTransmitterStep(value - output.low, output.span, process.span,
            root ? TransmitterPower::Square : TransmitterPower::Linear);
    } else {
        processValue = std::fma(processFraction, process.span, process.low);
    }
    finite({processValue, fraction});
    return {processValue, processFraction};
}
} // 命名空间 toolbox
