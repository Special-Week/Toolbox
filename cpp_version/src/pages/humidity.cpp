// 湿度与辐射工具页面；温标切换保存独立字段，摄氏温差与显示温差明确区分。
#include "toolbox/gui.hpp"
namespace toolbox {
void renderHumidity(Page &p) {
    TemperatureUnit unit{
        p.tool.kind == Kind::HumidityConversion ? p.select("unit", L"温度单位", {L"°C", L"K", L"°F"}) : 0};
    std::string uk = std::to_string(unit.index);
    auto temp = [&](const std::string &key, const std::wstring &label, double value) {
        return unit.celsius(
            p.number(key + "." + uk, label + L" (" + unit.symbol() + L")", unit.display(value)));
    };
    auto out = [&](double t) { return format(unit.display(t)) + L" " + unit.symbol(); };
    if (p.tool.kind == Kind::HumidityConversion) {
        p.heading(L"湿空气参数");
        double t = temp("temperature", L"环境温度", 20),
               pressure = p.number("pressure", L"大气压 (kPa)", 101.325);
        int known = p.select("known", L"已知参数", {L"相对湿度", L"露点 / 霜点", L"湿球温度值", L"混合比"});
        const wchar_t *names[] = {L"相对湿度 (%RH)", L"露点 / 霜点", L"湿球温度", L"混合比 (g/kg)"};
        double value = (known == 1 || known == 2)
                           ? temp("known." + std::to_string(known), names[known], known == 1 ? 10 : 15)
                           : p.number("known." + std::to_string(known), names[known], known == 0 ? 50 : 7.5);
        if (known == 1) {
            p.note(L"≤0 °C 按霜点输入，>0 °C 按露点输入。");
        }
        p.check();
        auto h = humidity(t, pressure * 1000, known, value);
        Metrics m{{L"相对湿度", format(h.rh) + L" %RH"},
                  {L"露点（水面）", h.dew ? out(*h.dew) : L"无有限值（干燥空气）"}};
        if (h.frost && *h.frost <= 0) {
            m.push_back({L"霜点（冰面）", out(*h.frost)});
        }
        m.insert(m.end(), {{L"饱和水气压", format(h.saturated) + L" Pa"},
                           {L"水气压", format(h.vapor) + L" Pa"},
                           {L"含湿量（体积分数）", format(h.volumeFraction) + L" L/L"},
                           {L"体积比（以干空气计）", format(h.ppm) + L" μL/L"},
                           {L"绝对湿度", format(h.absolute) + L" g/m³"},
                           {L"质量比", format(h.mass) + L" μg/g"},
                           {L"比湿", format(h.specific) + L" g/kg"},
                           {L"混合比", format(h.mixing) + L" g/kg"},
                           {L"含水量", format(h.normal) + L" g/Nm³"},
                           {L"湿球温度", h.wet ? out(*h.wet) : L"低于 −100 °C，超出模型范围"}});
        p.metrics(L"换算结果", m);
        return;
    }
    if (p.tool.kind == Kind::EarBlackbody) {
        int mode = p.select("ear.model", L"反射模型", {L"多次反射平衡", L"一次反射截断近似"});
        double t = temp("blackbody", L"黑体温度", 35), e = p.number("emissivity", L"黑体发射率", .9995),
               probe = temp("probe", L"探头温度", 18), pe = p.number("probe_emissivity", L"探头发射率", .8);
        p.check();
        double light = earBlackbody(t, e, probe, pe, mode == 1);
        p.metrics(L"修正结果", {{L"黑体亮温", out(light)},
                                {L"亮温修正值（亮温 − 黑体温度）",
                                 format((light - t) * unit.span()) + L" " + unit.symbol()}});
        return;
    }
    int model = p.select("radiation.model", L"计算模型", {L"有效波长法", L"Planck 辐射平衡"});
    bool correction = p.tool.kind == Kind::RadiationCorrection;
    int response = p.select("response", L"光谱响应", {L"单一有效波长", L"8～14 µm 波段", L"7～18 µm 波段"});
    double wavelength = response == 1 ? 8 : 7, upper = response == 1 ? 14 : (response == 2 ? 18 : 0);
    if (response == 0) {
        wavelength = p.number("wavelength", L"有效波长 (µm)", .65);
    }
    double t = temp("temperature", correction ? L"温度计示值" : L"辐射源真实温度", 1000),
           bg = temp("background", L"反射背景温度", 20);
    double source = p.number("source_emissivity", L"辐射源发射率", .99), instrument = 1;
    if (correction) {
        instrument = p.number("instrument_emissivity", L"温度计设置发射率", .95);
    }
    p.check();
    double delta, result;
    if (model == 0) {
        wavelength = effectiveWavelength(response, wavelength, t);
        delta = correction ? radiationCorrection(instrument, source, wavelength, t, bg)
                           : radiationError(source, wavelength, t, bg);
        delta = std::round(delta * 100) / 100;
        result = t + delta;
    } else {
        Spectrum spectrum(wavelength, upper);
        result =
            correction ? spectrum.corrected(t, bg, instrument, source) : spectrum.apparent(t, bg, source);
        delta = result - t;
    }
    Metrics m;
    if (model == 0) {
        m.push_back(
            {correction ? L"示值修正值" : L"发射率误差", format(delta * unit.span()) + L" " + unit.symbol()});
        if (correction) {
            m.push_back({L"修正后温度", out(result)});
        }
        m.push_back({L"有效波长", format(wavelength) + L" µm"});
    } else if (correction) {
        m = {{L"修正后温度", out(result)},
             {L"示值修正值", format(delta * unit.span()) + L" " + unit.symbol()}};
    } else {
        m = {{L"表观亮温", out(result)},
             {L"示值误差（亮温 − 真温）", format(delta * unit.span()) + L" " + unit.symbol()},
             {L"示值修正值", format(-delta * unit.span()) + L" " + unit.symbol()}};
    }
    p.metrics(L"计算结果", m);
}
} // namespace toolbox
