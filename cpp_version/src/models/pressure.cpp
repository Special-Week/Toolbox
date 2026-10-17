// 压力及变送器量程模型；输入准确度使用百分数，内部保持 double 精度。
#include "toolbox/models.hpp"
namespace toolbox {
Range::Range(double lower, double upper) : low(lower), high(upper), span(upper - lower) {
    finite({low, high, span});
    require(span > 0, "量程上限必须大于下限");
}
bool Range::covers(const Range &other) const {
    return low <= other.low && high >= other.high;
}
Selection selectStandard(Range dut, Range reference, double da, double ra, double required, bool piston) {
    finite({da, ra, required});
    require(da > 0 && ra > 0 && required > 0, "准确度和量传比须大于 0");
    double dt = dut.span * (da / 100),
           rt = (piston ? std::max(dut.span, .1 * reference.span) : reference.span) * (ra / 100);
    require(dt > 0 && rt > 0, "允差低于可表示范围");
    double ratio = dt / rt;
    finite({dt, rt, ratio});
    bool coverage = reference.covers(dut);
    return {dt, rt, ratio, coverage, coverage && ratio >= required};
}
ElectricalSelection selectTransmitter(Range dut, Range reference, Range output, double da, double ra,
                                      double reading, double fixed, double required) {
    finite({reading, fixed});
    require(reading >= 0 && fixed >= 0, "电测读数误差与固定误差不得为负");
    auto p = selectStandard(dut, reference, da, ra, required);
    double pressure = p.referenceTolerance / dut.span * output.span;
    // 双极性信号的最坏读数项取两端绝对值的较大者，不能只看输出上限。
    double electrical = reading / 100 * std::max(std::abs(output.low), std::abs(output.high)) + fixed;
    double expanded = 2 * std::hypot(pressure, electrical) / std::sqrt(3.);
    double allowance = output.span * da / 100 / required;
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
std::pair<double, double> transmitterForward(Range process, Range output, bool root, double value) {
    finite({value});
    require(value >= process.low && value <= process.high, "输入值超出所选量程");
    double fraction = (value - process.low) / process.span;
    require(!root || fraction >= 0, "开方输入不能低于量程下限");
    double signal = std::fma(root ? std::sqrt(fraction) : fraction, output.span, output.low);
    finite({signal, fraction});
    return {signal, fraction};
}
std::pair<double, double> transmitterInverse(Range process, Range output, bool root, double value) {
    finite({value});
    require(value >= output.low && value <= output.high, "输入值超出所选量程");
    double fraction = (value - output.low) / output.span;
    require(!root || fraction >= 0, "开方输出不能低于信号下限");
    if (root) {
        fraction *= fraction;
    }
    double processValue = std::fma(fraction, process.span, process.low);
    finite({processValue, fraction});
    return {processValue, fraction};
}
} // namespace toolbox
