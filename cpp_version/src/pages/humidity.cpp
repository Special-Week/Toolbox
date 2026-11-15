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
        auto c = [&](double v) { return p.processValue(v); };
        auto &steps = p.processSteps;
        steps.push_back(L"环境温度 t = " + c(unit.display(t)) + L" " + unit.symbol() + L" = " + c(t) + L" °C；大气压 P = " + c(pressure) + L" kPa × 1000 = " + c(pressure * 1000) + L" Pa。");
        auto saturationProcess = [&](double temperature, const std::wstring &label) {
            const auto x = c(temperature);
            const auto result = c(saturation(temperature));
            if (temperature <= 0)
                steps.push_back(label + L"（t ≤ 0 °C，冰面）：pₛ(t) = 611.2 × exp[22.46t/(272.62+t)] = 611.2 × exp[22.46 × " + x + L"/(272.62 + " + x + L")] = " + result + L" Pa。");
            else if (temperature < 1)
                steps.push_back(label + L"（0 < t < 1 °C，水面）：pₛ(t) = 611.2 × exp[17.62t/(243.12+t)] = 611.2 × exp[17.62 × " + x + L"/(243.12 + " + x + L")] = " + result + L" Pa。");
            else {
                const auto k = c(temperature + 273.15);
                steps.push_back(label + L"（t ≥ 1 °C，水面）：T = t + 273.15 = " + k + L" K；pₛ(t) = 10^[(10.286T−2148.4909)/(T−35.85)] = 10^[(10.286 × " + k + L"−2148.4909)/(" + k + L"−35.85)] = " + result + L" Pa。");
            }
        };
        saturationProcess(t, L"环境饱和水汽压");
        steps.push_back(L"");
        if (known == 0)
            steps.push_back(L"已知相对湿度 φ = " + c(value) + L" %；原始水汽压 pᵥ = pₛ × φ / 100 = " + c(h.saturated) + L" × " + c(value) + L" / 100 = " + c(h.rawVapor) + L" Pa。");
        else if (known == 1) {
            steps.push_back(L"输入露点/霜点 = " + c(unit.display(value)) + L" " + unit.symbol() + L" = " + c(value) + L" °C。");
            saturationProcess(value, L"输入露点/霜点的饱和水汽压");
            steps.push_back(L"原始水汽压 pᵥ = pₛ(输入露点/霜点) = " + c(h.rawVapor) + L" Pa。");
        } else if (known == 2) {
            steps.push_back(L"输入湿球温度 = " + c(unit.display(value)) + L" " + unit.symbol() + L" = " + c(value) + L" °C。");
            saturationProcess(value, L"输入湿球的饱和水汽压");
            steps.push_back(L"已知湿球温度 t湿 = " + c(value) + L" °C；原始水汽压 pᵥ = pₛ(t湿) − 0.000662P(t − t湿) = " + c(saturation(value)) + L" − 0.000662 × " + c(pressure * 1000) + L" × (" + c(t) + L" − " + c(value) + L") = " + c(h.rawVapor) + L" Pa。");
        } else {
            double q = value / 1000 * 28.959 / 18.0152;
            steps.push_back(L"已知混合比 = " + c(value) + L" g/kg；q = 混合比 / 1000 × 28.959 / 18.0152 = " + c(value) + L" / 1000 × 28.959 / 18.0152 = " + c(q) + L"；原始水汽压 pᵥ = Pq/(1+q) = " + c(pressure * 1000) + L" × " + c(q) + L"/(1 + " + c(q) + L") = " + c(h.rawVapor) + L" Pa。");
        }
        if (h.rawVapor != h.vapor)
            steps.push_back(L"原始相对湿度仅因浮点误差略微越界，程序将水汽压调整到物理边界：" + c(h.rawVapor) + L" → " + c(h.vapor) + L" Pa。");
        steps.push_back(L"相对湿度 φ = 100pᵥ/pₛ = 100 × " + c(h.vapor) + L" / " + c(h.saturated) + L" = " + c(h.rh) + L" %。");
        steps.push_back(L"");
        steps.push_back(L"水汽体积分数 = pᵥ/P = " + c(h.vapor) + L" / " + c(pressure * 1000) + L" = " + c(h.volumeFraction) + L" L/L。");
        steps.push_back(L"");
        steps.push_back(L"干空气基准体积比 = pᵥ/(P−pᵥ) × 10⁶ = " + c(h.vapor) + L" / (" + c(pressure * 1000) + L" − " + c(h.vapor) + L") × 10⁶ = " + c(h.ppm) + L" μL/L。");
        steps.push_back(L"");
        steps.push_back(L"干空气基准质量比 = 体积比 × 18.0152/28.959 = " + c(h.ppm) + L" × 18.0152/28.959 = " + c(h.mass) + L" μg/g。");
        steps.push_back(L"");
        steps.push_back(L"绝对湿度 = pᵥ × 18.015 / [(t+273.15) × 8.31451] = " + c(h.vapor) + L" × 18.015 / (" + c(t + 273.15) + L" × 8.31451) = " + c(h.absolute) + L" g/m³。");
        steps.push_back(L"");
        steps.push_back(L"标准状态含水量 = 绝对湿度 × (t+273.15) × 101325 / (273.15P) = " + c(h.absolute) + L" × " + c(t + 273.15) + L" × 101325 / (273.15 × " + c(pressure * 1000) + L") = " + c(h.normal) + L" g/Nm³。");
        steps.push_back(L"");
        steps.push_back(L"比湿 = 质量比/(10⁶+质量比) × 1000 = " + c(h.mass) + L" / (10⁶ + " + c(h.mass) + L") × 1000 = " + c(h.specific) + L" g/kg。");
        steps.push_back(L"");
        steps.push_back(L"混合比 = 质量比/1000 = " + c(h.mass) + L" / 1000 = " + c(h.mixing) + L" g/kg。");
        steps.push_back(L"");
        if (h.dew) {
            double z = std::log(h.vapor) - std::log(611.2), l = std::log10(h.vapor);
            steps.push_back(L"令 z = ln(pᵥ) − ln(611.2) = " + c(z) + L"；冰面公式反算温度 = 272.62z/(22.46−z) = " + c(*h.frost) + L" °C。" + (*h.frost <= 0 ? std::wstring(L"该温度不高于 0 °C，显示为霜点。") : std::wstring(L"该温度高于 0 °C，不显示为霜点。")));
            steps.push_back(h.vapor < 657 ? L"露点采用低压水面逆公式 243.12z/(17.62−z) = " + c(*h.dew) + L" °C。" : L"令 l = log₁₀(pᵥ) = " + c(l) + L"；露点 = (−2148.4909+35.85l)/(l−10.286)−273.15 = " + c(*h.dew) + L" °C。");
        } else steps.push_back(L"水汽压为零，露点和霜点没有有限值。");
        if (h.wet)
            steps.push_back(L"湿球温度通过在 −100 °C 至环境温度之间反解 pₛ(t湿) − 0.000662P(t−t湿) = pᵥ 得到：t湿 = " + c(*h.wet) + L" °C。");
        else steps.push_back(L"湿球方程在 −100 °C 至环境温度之间没有解，湿球温度超出模型范围。");
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
        auto c = [&](double v) { return p.processValue(v); };
        double scale = std::max(t + 273.15, probe + 273.15), b = (t + 273.15) / scale, q = (probe + 273.15) / scale;
        double signal = mode == 1 ? e * std::pow(b, 4) + (1-e) * (pe * std::pow(q, 4) + (1-pe) * e * std::pow(b, 4)) : e / (e+(1-e)*pe) * std::pow(b,4) + (1-e)*pe / (e+(1-e)*pe) * std::pow(q,4);
        p.processSteps = {L"黑体温度 = " + c(t) + L" °C，发射率 = " + c(e) + L"；探头温度 = " + c(probe) + L" °C，发射率 = " + c(pe) + L"。",
            L"为避免四次方溢出，S = max(T黑, T探) = " + c(scale) + L" K；b = T黑/S = " + c(b) + L"，q = T探/S = " + c(q) + L"。", L"",
            mode == 1 ? L"一次反射：Q = ε黑b⁴ + (1−ε黑)[ε探q⁴ + (1−ε探)ε黑b⁴] = " + c(signal) + L"。" : L"多次反射平衡：D = ε黑 + (1−ε黑)ε探 = " + c(e+(1-e)*pe) + L"；Q = (ε黑/D)b⁴ + [(1−ε黑)ε探/D]q⁴ = " + c(signal) + L"。",
            L"亮温 = S√√Q − 273.15 = " + c(light) + L" °C；亮温修正值 = 亮温 − 黑体温度 = " + c(light-t) + L" °C。"};
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
        auto c = [&](double v) { return p.processValue(v); };
        p.processSteps.push_back(response == 0 ? L"单一有效波长 λ = " + c(wavelength) + L" μm。" : response == 1 ? L"8～14 μm 波段：λ = 9.6 + 221/(t+273.15) = " + c(wavelength) + L" μm。" : L"7～18 μm 波段：λ = 8.7 + 525/(t+273.15) = " + c(wavelength) + L" μm。");
        auto errorProcess = [&](const wchar_t *label, double e) {
            double err = radiationError(e, wavelength, t, bg);
            if (e == 1) { p.processSteps.push_back(std::wstring(label) + L"发射率为 1，模型直接返回误差 0 °C。"); return err; }
            double cc = 14388 / wavelength, tk = t + 273.15, bk = bg + 273.15;
            double occupation = 1 / std::expm1(cc/tk) - (1-e) / std::expm1(cc/bk), bright = cc / std::log1p(e/occupation);
            p.processSteps.push_back(std::wstring(label) + L"：ε = " + c(e) + L"；T = " + c(tk) + L" K，T背景 = " + c(bk) + L" K，c = 14388/λ = " + c(cc) + L" K。");
            p.processSteps.push_back(L"C = 1/expm1(c/T) − (1−ε)/expm1(c/T背景) = " + c(occupation) + L"；M = c/ln(1+ε/C) = " + c(bright) + L" K。");
            p.processSteps.push_back(L"发射率误差 = T − M = " + c(err) + L" °C。程序使用 expm1 和 log1p 降低数值误差。");
            return err;
        };
        double raw;
        if (correction) {
            double instrumentError = errorProcess(L"仪表", instrument);
            double sourceError = errorProcess(L"辐射源", source);
            raw = instrumentError - sourceError;
        } else {
            raw = errorProcess(L"辐射源", source);
        }
        delta = correction ? radiationCorrection(instrument, source, wavelength, t, bg)
                           : radiationError(source, wavelength, t, bg);
        delta = std::round(delta * 100) / 100;
        result = t + delta;
        p.processSteps.push_back(L"" );
        p.processSteps.push_back(std::wstring(correction ? L"修正值 = 仪表误差 − 辐射源误差" : L"发射率误差") + L" = " + c(raw) + L" °C；按 0.01 °C 修约后 = " + c(delta) + L" °C；示值加修正量 = " + c(result) + L" °C。");
    } else {
        Spectrum spectrum(wavelength, upper);
        result =
            correction ? spectrum.corrected(t, bg, instrument, source) : spectrum.apparent(t, bg, source);
        delta = result - t;
        auto c = [&](double v) { return p.processValue(v); };
        double lt = spectrum.logRadiance(t+273.15), lb = spectrum.logRadiance(bg+273.15), scale = std::max(lt,lb);
        double weight = correction ? instrument/source : source;
        double signal = std::fma(weight, std::exp(lt-scale), (1-weight)*std::exp(lb-scale));
        p.processSteps = {L"Planck 辐射平衡：Bλ(T) = λ⁻⁵/[exp(c₂/(λT))−1]，c₂ = hc/k；温度使用 K。", upper == 0 ? L"单一波长 λ = " + c(wavelength) + L" μm。" : L"平坦波段 " + c(wavelength) + L"～" + c(upper) + L" μm；使用 256 等分复合辛普森积分。", L"程序在对数域求辐射量和加权和，避免指数溢出。", L"", correction ? L"ε源B(T真)+(1−ε源)B(T背景) = ε表B(T示)+(1−ε表)B(T背景)；权重 w = ε表/ε源 = " + c(weight) + L"。" : L"B(T亮) = ε源B(T真)+(1−ε源)B(T背景)；权重 w = ε源 = " + c(weight) + L"。", L"ln(B输入) = " + c(lt) + L"，ln(B背景) = " + c(lb) + L"，S = max(两者) = " + c(scale) + L"。", L"Q = w·exp(ln(B输入)−S)+(1−w)·exp(ln(B背景)−S) = " + c(signal) + L"；目标 ln(B) = S+ln(Q) = " + c(scale+std::log(signal)) + L"。", L"从 0～512 K 开始，必要时将上限加倍，再二分 85 次反算温度。", L"反算温度 = " + c(result) + L" °C；与输入温度之差 = " + c(delta) + L" °C。"};
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
} // 命名空间 toolbox
