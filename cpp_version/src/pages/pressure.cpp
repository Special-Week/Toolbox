// 标准器核验、量程推荐及高差换算；切换单位时同步转换已填量程。
#include "toolbox/gui.hpp"
namespace toolbox {
static std::wstring judgement(bool good) {
    return good ? L"符合" : L"不符合";
}
static void legacy(Page &p) {
    p.heading(L"变送器通道参数");
    double dl = p.number("dut_low", L"被检压力下限", 0), dh = p.number("dut_high", L"被检压力上限", 1),
           da = p.number("dut_accuracy", L"被检压力准确度 (%)", .1),
           ra = p.number("ref_accuracy", L"标准压力准确度 (%)", .01),
           rl = p.number("ref_low", L"标准压力下限", 0), rh = p.number("ref_high", L"标准压力上限", 1.6);
    double sl = p.number("signal_low", L"被检输出下限 (mA)", 4),
           sh = p.number("signal_high", L"被检输出上限 (mA)", 20),
           sa = p.number("signal_accuracy", L"被检电信号准确度 (%)", .1),
           rsa = p.number("ref_signal_accuracy", L"标准电测准确度 (%)", .02),
           rsl = p.number("ref_signal_low", L"标准电测下限 (mA)", 0),
           rsh = p.number("ref_signal_high", L"标准电测上限 (mA)", 24),
           required = p.number("required", L"量传比要求", 3);
    p.check();
    auto pressure = selectStandard(Range(dl, dh), Range(rl, rh), da, ra, required),
         signal = selectStandard(Range(sl, sh), Range(rsl, rsh), sa, rsa, required);
    p.metrics(L"选型结果", {{L"被检压力允差", format(pressure.dutTolerance)},
                            {L"标准压力允差", format(pressure.referenceTolerance)},
                            {L"压力通道量传比", format(pressure.ratio)},
                            {L"被检输出允差", format(signal.dutTolerance)},
                            {L"标准电测允差", format(signal.referenceTolerance)},
                            {L"电信号通道量传比", format(signal.ratio)},
                            {L"压力量程覆盖", judgement(pressure.coverage)},
                            {L"电信号量程覆盖", judgement(signal.coverage)},
                            {L"规程要求", L"≥ " + format(required) + L" 倍"},
                            {L"综合判断", judgement(pressure.meets && signal.meets)}});
}
static void selection(Page &p, bool transmitter) {
    if (transmitter && p.select("model", L"评定模型", {L"压力与电测合成评定", L"两通道跨度比评定"}) == 1) {
        legacy(p);
        return;
    }
    int mode =
        p.select("selection.mode", L"选型方式", {L"核验指定标准器", L"按被检量程推荐", L"按检定范围推荐"});
    const auto &unitsList = units(Dimension::Pressure);
    std::vector<std::wstring> labels;
    int mpa = 0;
    for (std::size_t i = 0; i < unitsList.size(); ++i) {
        labels.push_back(unitsList[i].code);
        if (labels.back() == L"MPa") {
            mpa = int(i);
        }
    }
    int index = p.select("selection.unit", L"压力单位", labels, mpa),
        previous = p.state.choice(p.key("previous.unit"), index);
    if (previous != index) {
        for (const char *field : {"dut_low", "dut_high", "ref_low", "ref_high"}) {
            auto it = p.state.values.find(p.key(field));
            if (it != p.state.values.end()) {
                try {
                    it->second =
                        inputNumber(convert(Dimension::Pressure, parse(utf8(it->second)), previous, index));
                } catch (const std::exception &) {
                }
            }
        }
        p.state.choices[p.key("previous.unit")] = index;
    }
    bool piston =
        !transmitter && p.select("reference_type", L"标准器类型", {L"数字压力标准器", L"活塞压力计"}) == 1;
    double dl = p.number("dut_low", L"被检 / 检定范围下限", 0),
           dh = p.number("dut_high", L"被检 / 检定范围上限", 1),
           da = p.number("dut_accuracy", L"被检准确度 (%)", transmitter ? .1 : .4),
           required = p.number("required", L"量传比要求", 3);
    double sl = 4, sh = 20, reading = .02, fixed = .001;
    std::wstring signal = L"mA";
    if (transmitter) {
        int u = p.select("signal.unit", L"电信号单位", {L"mA", L"V", L"mV"});
        signal = std::vector<std::wstring>{L"mA", L"V", L"mV"}[u];
        sl = p.number("signal_low", L"输出信号下限", 4);
        sh = p.number("signal_high", L"输出信号上限", 20);
        reading = p.number("reading", L"电测读数准确度 (%)", .02);
        fixed = p.number("fixed", L"电测固定误差", .001);
    }
    if (mode == 0) {
        double rl = p.number("ref_low", L"标准器量程下限", 0),
               rh = p.number("ref_high", L"标准器量程上限", 1.6),
               ra = p.number("ref_accuracy", L"标准器准确度 (%)", transmitter ? .01 : .05);
        p.check();
        Range dut(dl, dh), reference(rl, rh);
        if (transmitter) {
            auto s = selectTransmitter(dut, reference, Range(sl, sh), da, ra, reading, fixed, required);
            p.metrics(L"选型结果", {{L"被检压力允差", format(s.pressure.dutTolerance)},
                                    {L"标准压力允差", format(s.pressure.referenceTolerance)},
                                    {L"合成扩展不确定度 (k=2)", format(s.expanded) + L" " + signal},
                                    {L"电信号允许限值", format(s.allowance) + L" " + signal},
                                    {L"综合判断", judgement(s.meets)},
                                    {L"压力量程覆盖", judgement(s.pressure.coverage)}});
        } else {
            auto s = selectStandard(dut, reference, da, ra, required, piston);
            p.metrics(L"选型结果", {{L"被检允差", format(s.dutTolerance)},
                                    {L"标准器允差", format(s.referenceTolerance)},
                                    {L"量传比", format(s.ratio)},
                                    {L"量程覆盖", judgement(s.coverage)},
                                    {L"综合判断", judgement(s.meets)}});
        }
        return;
    }
    int series = transmitter ? 0 : p.select("series", L"标准量程系列", {L"常用公制量程", L"扩展量程"});
    double fixedAccuracy = mode == 2 ? p.number("ref_accuracy", L"标准器准确度 (%)", .05) : 0;
    p.check();
    Range dut(dl, dh);
    selectStandard(dut, dut, da, .01, required, piston);
    std::vector<Range> catalogue;
    for (auto [lo, hi] : series == 0 ? commonRanges() : extendedRanges()) {
        catalogue.emplace_back(convert(Dimension::Pressure, lo, mpa, index),
                               convert(Dimension::Pressure, hi, mpa, index));
    }
    std::vector<Range> duties;
    if (mode == 1) {
        duties.push_back(dut);
    } else {
        for (auto &r : catalogue) {
            if (dut.covers(r)) {
                duties.push_back(r);
            }
        }
    }
    int count = 0;
    for (auto duty : duties) {
        Values accuracies = mode == 2 ? Values{fixedAccuracy}
                                      : (piston ? Values{.003, .005, .01, .05} : Values{.01, .02, .05});
        std::vector<std::pair<Range, double>> matches;
        for (auto r : catalogue) {
            for (double a : accuracies) {
                bool good =
                    transmitter
                        ? selectTransmitter(duty, r, Range(sl, sh), da, a, reading, fixed, required).meets
                        : selectStandard(duty, r, da, a, required, piston).meets;
                if (good) {
                    matches.push_back({r, a});
                }
            }
        }
        std::sort(matches.begin(), matches.end(), [](auto a, auto b) {
            return a.second != b.second ? a.second > b.second : a.first.span < b.first.span;
        });
        p.heading(L"被检量程：" + format(duty.low) + L"～" + format(duty.high) + L" " + labels[index]);
        if (matches.empty()) {
            p.note(L"没有满足当前条件的标准器组合");
        }
        for (std::size_t i = 0; i < matches.size(); ++i) {
            auto &m = matches[i];
            p.note((i == 0 ? L"推荐：" : L"可选：") + format(m.first.low) + L"～" + format(m.first.high) +
                       L" " + labels[index] + L"，" + format(m.second) + L"%",
                   i == 0 ? Blue : Muted);
            ++count;
        }
    }
    p.metrics(L"选型结果", {{L"满足条件的量程 / 准确度组合", std::to_wstring(count)}});
}
void renderPressure(Page &p) {
    if (p.tool.kind == Kind::PressureReference || p.tool.kind == Kind::TransmitterReference) {
        selection(p, p.tool.kind == Kind::TransmitterReference);
        return;
    }
    if (p.tool.kind == Kind::AltitudePressure) {
        int mode = p.select("mode", L"计算方式", {L"位势高度 → 标准气压", L"标准气压 → 位势高度"});
        double input = p.number(mode == 0 ? "altitude" : "pressure",
                                mode == 0 ? L"位势高度 (m)" : L"标准气压 (kPa)", mode == 0 ? 0 : 101.325);
        p.check();
        double h = mode == 0 ? input : atmosphereAltitude(input * 1000),
               pressure = mode == 0 ? atmospherePressure(h) : input * 1000;
        p.metrics(L"换算结果",
                  {{L"气压高度", format(h) + L" m"},
                   {L"标准气压", format(pressure / 1000) + L" kPa"},
                   {L"空气密度", format(pressure / (287.05287 * atmosphereTemperature(h))) + L" kg/m³"}});
        return;
    }
    p.heading(L"高差与传压介质");
    int medium = p.select("medium", L"介质类型", {L"液体", L"气体（输入密度）"});
    double h = p.number("height", L"高度差 ΔH (m)", .1);
    int gas = medium == 1
                  ? p.select("gas.state", L"气体密度条件", {L"输入当地状态密度", L"由标准状态密度换算"}, 1)
                  : 0;
    double rho = p.number("density." + std::to_string(medium) + "." + std::to_string(gas),
                          L"介质密度 (kg/m³)", medium == 0 ? 998.2 : (gas == 1 ? 1.293 : 1.204)),
           t = 20, pressure = 100, ambient = 101.325;
    bool gauge = false;
    if (gas == 1) {
        p.note(L"标准状态为 0 °C、101.325 kPa。");
        gauge = p.select("pressure.type", L"压力类型", {L"表压", L"绝压"}) == 0;
        t = p.number("gas.temperature", L"气体温度 (°C)", 20);
        pressure = p.number("gas.pressure", L"标准器压力 (kPa)", 100);
        ambient = p.number("gas.ambient", L"环境大气压 (kPa)", 101.325);
    }
    double g = p.number("gravity", L"重力加速度 g (m/s²)", 9.80665);
    p.check();
    double dp = gas == 1 ? gasDifference(h, rho, g, t, pressure * 1000, ambient * 1000, gauge)
                         : hydrostatic(h, rho, g);
    p.metrics(L"修正结果",
              {{L"标准器压力 − 被检表压力", format(dp) + L" Pa"},
               {L"标准值移至被检高度的修正量", format(-dp / 1000) + L" kPa"},
               {L"等效水柱（4 °C）", format(dp / 9.80638) + L" mmH₂O"},
               {L"修正方向", h > 0 ? L"被检表高于标准器" : (h < 0 ? L"被检表低于标准器" : L"无高差")}});
}
} // namespace toolbox
