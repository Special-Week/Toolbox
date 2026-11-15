// 误差统计和不确定度预算。折叠只控制展示，已隐藏的分量仍参与合成。
#include "toolbox/gui.hpp"
#include "toolbox/card_columns.hpp"
namespace toolbox {
static const Values sampleDefaults{10.02, 10.01, 10.03, 10.02, 10.01};
static const std::vector<std::wstring> distributions{L"均匀分布", L"三角分布", L"正态 k=2",
                                                     L"正态 k=3", L"两点分布", L"反正弦分布"};
static std::wstring roundedProcess(Page &p, double value, std::optional<int> digits) {
    return digits && *digits > 13 ? inputNumber(value) : p.processValue(value);
}
static std::wstring roundingDescription(const UncertaintyRounding &r) {
    return r.digits ? L"保留 " + std::to_wstring(*r.digits) + L" 位有效数字，" + (r.carry ? L"进位" : L"四舍六入、五成双") : L"未设置有效数字，保留原始精度";
}
static void sampleProcess(Page &p, const Values &values, const Statistics &s, bool separate) {
    p.processSteps.push_back(L"测量值 n = " + std::to_wstring(values.size()) + L"；平均值 x̄ = " + p.processValue(s.mean) + L"。");
    if (s.centered)
        p.processSteps.push_back(L"以首个测量值 x₀ = " + inputNumber(s.origin) + L" 为基准；中心化均值 d̄ = Σ(xᵢ − x₀)/n = " + p.processValue(s.deltaMean) + L"。");
    else
        p.processSteps.push_back(L"中心化差值溢出，使用缩放均值；残差按 xᵢ − x̄ 计算。");
    for (std::size_t i = 0; i < values.size(); ++i) {
        if (separate)
            p.processSteps.push_back(L"");
        if (s.centered) {
            double difference = values[i] - s.origin;
            p.processSteps.push_back(L"测量点 " + std::to_wstring(i + 1) + L"：dᵢ = xᵢ − x₀ = " + inputNumber(values[i]) + L" − " + inputNumber(s.origin) + L" = " + p.processValue(difference) + L"；残差 vᵢ = dᵢ − d̄ = " + p.processValue(difference) + L" − " + p.processValue(s.deltaMean) + L" = " + p.processValue(s.residuals[i]) + L"。");
        } else {
            p.processSteps.push_back(L"测量点 " + std::to_wstring(i + 1) + L"：残差 vᵢ = xᵢ − x̄ = " + inputNumber(values[i]) + L" − " + inputNumber(s.mean) + L" = " + p.processValue(s.residuals[i]) + L"。");
        }
    }
    p.processSteps.push_back(L"样本标准差 s = √[Σvᵢ²/(n − 1)] = " + p.processValue(s.sd) + L"；程序使用补偿求和和 hypot 累积。");
}
static void uncertaintyComponentProcess(Page &p, const Component &component, const UncertaintyDetail &detail, const std::wstring &parent) {
    const auto path = parent.empty() ? component.name : parent + L" / " + component.name;
    p.processSteps.push_back(L"");
    p.processSteps.push_back(L"分量：" + path + L"。");
    if (component.kind == 0) {
        auto stats = statistics(component.values);
        sampleProcess(p, component.values, stats, false);
        if (component.useRange) {
            double range = stats.maximum - stats.minimum, coefficient = rangeCoefficient(component.values.size());
            p.processSteps.push_back(L"极差法：R = " + inputNumber(stats.maximum) + L" − " + inputNumber(stats.minimum) + L" = " + p.processValue(range) + L"；u = R/(d₂√m) = " + p.processValue(range) + L" / (" + inputNumber(coefficient) + L" × √" + std::to_wstring(component.values.size()) + L") = " + p.processValue(detail.rawStandard) + L"。");
        } else {
            auto count = component.averageCount.value_or(1);
            p.processSteps.push_back(L"贝塞尔法：取测量次数 n = " + std::to_wstring(count) + (component.averageCount ? L"" : L"（留空默认 1 次）") + L"（此 n 可不同于上方估计 s 的测量值个数）；u = s/√n = " + p.processValue(stats.sd) + L" / √" + std::to_wstring(count) + L" = " + p.processValue(detail.rawStandard) + L"。");
        }
    } else if (component.kind == 1) {
        p.processSteps.push_back(L"B 类" + distributions[std::size_t(component.distribution)] + L"：" + (component.distribution == 2 || component.distribution == 3 ? L"证书扩展不确定度" : L"区间半宽") + L" = " + inputNumber(component.halfWidth) + L"，除数 d = " + inputNumber(distributionDivisor(component.distribution)) + L"；u = 输入/d = " + inputNumber(component.halfWidth) + L" / " + inputNumber(distributionDivisor(component.distribution)) + L" = " + p.processValue(detail.rawStandard) + L"。");
    } else {
        p.processSteps.push_back(component.maximum ? L"子分量合成方法：最大绝对贡献。" : L"子分量合成方法：贡献平方和根。");
        for (std::size_t i = 0; i < component.children.size(); ++i)
            uncertaintyComponentProcess(p, component.children[i], detail.children[i], path);
        p.processSteps.push_back(L"子分量合成后的标准不确定度 = " + p.processValue(detail.rawStandard) + L"。");
    }
    p.processSteps.push_back(roundingDescription(component.rounding) + L"；修约前 u = " + p.processValue(detail.rawStandard) + L"；修约后 u = " + roundedProcess(p, detail.standard, component.rounding.digits) + L"；贡献 q = 灵敏系数 × 修约后 u = " + inputNumber(detail.sensitivity) + L" × " + roundedProcess(p, detail.standard, component.rounding.digits) + L" = " + p.processValue(detail.contribution) + L"。");
}
static std::optional<std::size_t> averageCountInput(Page &p, const std::string &key) {
    const auto text = p.textInput(key, L"取测量次数 n", L"");
    p.note(L"留空默认 1 次。");
    try {
        return parseAverageCount(text);
    } catch (const std::exception &e) {
        if (p.inputError.empty())
            p.inputError = wide(e.what());
        return std::nullopt;
    }
}
static UncertaintyRounding roundingInput(Page &p, const std::string &key) {
    const auto text = p.textInput(key + ".digits", L"有效数字", L"");
    p.note(L"留空保留原始计算精度（1～17 位）。");
    const bool carry = p.select(key + ".rounding", L"修约方式", {L"四舍六入", L"进位"}, 1) == 1;
    try {
        return {parseSignificantDigits(text), carry};
    } catch (const std::exception &e) {
        if (p.inputError.empty())
            p.inputError = wide(e.what());
        return {std::nullopt, carry};
    }
}
static void componentCards(Page &p, int count, std::vector<int> &assignments,
                           const std::function<void(Page &, int)> &render) {
    const float left = p.x, top = p.y, width = p.w;
    assignments.resize(std::min(assignments.size(), std::size_t(count)));
    const bool columns = count > 1 && width >= 760;
    const float cardWidth = columns ? (width - 12) / 2 : width;
    float heights[2] = {top, top};
    for (int i = 0; i < count; ++i) {
        int column = 0;
        if (columns) {
            column = cardColumn(assignments, std::size_t(i), heights[0], heights[1]);
        }
        float cardX = left + column * (cardWidth + 12), cardY = heights[column];
        auto fill = p.ui.draws.size();
        p.ui.fill({cardX, cardY, cardWidth, 0}, White, 8);
        p.ui.border({cardX, cardY, cardWidth, 0}, Border, 8);
        p.x = cardX + 16;
        p.y = cardY + 16;
        p.w = cardWidth - 32;
        render(p, i);
        const float bottom = p.y + 16;
        p.ui.draws[fill].rect.h = bottom - cardY;
        p.ui.draws[fill + 1].rect.h = bottom - cardY;
        heights[column] = bottom + 8;
    }
    p.x = left;
    p.w = width;
    p.y = std::max(heights[0], heights[1]);
}
static Component component(Page &p, const std::string &key, int depth) {
    Component c;
    c.name = p.textInput(key + ".name", L"分量名称", L"分量");
    c.kind = p.select(key + ".type", L"分量类型",
                      depth < 4 ? std::vector<std::wstring>{L"A 类评定", L"B 类评定", L"合成分量"}
                                : std::vector<std::wstring>{L"A 类评定", L"B 类评定"},
                      1);
    c.sensitivity = p.number(key + ".sensitivity", L"灵敏系数", 1);
    c.rounding = roundingInput(p, key);
    if (c.kind == 0) {
        c.useRange = p.select(key + ".a_method", L"A 类计算方法", {L"贝塞尔法", L"极差法"}) == 1;
        int count = p.count(key + ".count", L"测量数据", 5, 2, c.useRange ? 9 : 50);
        c.values = p.values(key + ".sample", L"测量值", count, sampleDefaults);
        if (!c.useRange)
            c.averageCount = averageCountInput(p, key + ".average_count");
    } else if (c.kind == 1) {
        c.distribution = p.select(key + ".distribution", L"分布类型", distributions);
        c.halfWidth =
            p.number(key + ".width",
                     c.distribution == 2 || c.distribution == 3 ? L"证书扩展不确定度 U" : L"区间半宽", .02);
    } else {
        c.maximum = p.select(key + ".method", L"合成方法", {L"平方和根", L"最大绝对贡献"}) == 1;
        int n = p.count(key + ".children", L"子分量", 2, 1, 12);
        for (int i = 0; i < n; ++i) {
            std::string k = key + ".child." + std::to_string(i);
            bool open = p.collapse(k + ".open", L"子分量 " + std::to_wstring(i + 1), true);
            float oldY = p.y;
            Rect clip = p.ui.clip;
            if (!open) {
                p.ui.clip = {0, 0, 0, 0};
            }
            c.children.push_back(component(p, k, depth + 1));
            if (!open) {
                p.y = oldY;
            }
            p.ui.clip = clip;
        }
    }
    return c;
}
static void showDetails(Page &p, const std::vector<UncertaintyDetail> &items, int depth = 0) {
    for (auto &d : items) {
        p.note(std::wstring(std::size_t(depth) * 2, L' ') + d.name + L"：u=" + format(d.rawStandard) +
               L"，修约后 u=" + formatRoundedUncertainty(d.standard, d.digits) + L"，c=" +
               format(d.sensitivity) + L"，贡献=" + format(d.contribution));
        showDetails(p, d.children, depth + 1);
    }
}
static void uncertainty(Page &p) {
    if (p.select("budget.mode", L"评定方式", {L"分量预算表", L"A/B 类快速评定"}) == 1) {
        int bn = p.count("quick.b.count", L"B 类分量", 2, 0, 50);
        int distribution = p.select("quick.distribution", L"B 类共用分布", distributions);
        double bc = p.number("quick.b.c", L"B 类共用灵敏系数", 1);
        int an = 0;
        Values av, bv;
        std::optional<std::size_t> averageCount;
        double ac = 1;
        UncertaintyRounding aRounding;
        std::vector<UncertaintyRounding> bRounding;
        componentCards(p, 1 + bn, p.app.uncertaintyColumns[1], [&](Page &card, int index) {
            if (index == 0) {
                card.heading(L"A 类评定数据");
                an = card.count("quick.a.count", L"重复测量值", 5, 0, 50);
                av = card.values("quick.a.value", L"测量值", an, sampleDefaults);
                if (an)
                    averageCount = averageCountInput(card, "quick.a.average_count");
                ac = card.number("quick.a.c", L"A 类灵敏系数", 1);
                if (an)
                    aRounding = roundingInput(card, "quick.a");
            } else {
                int i = index - 1;
                card.heading(L"B 类分量 " + std::to_wstring(i + 1));
                auto label = distribution == 2 || distribution == 3 ? L"扩展不确定度 U" : L"半宽";
                bv.push_back(card.number("quick.b.value." + std::to_string(i), label,
                                         i == 0 ? .02 : (i == 1 ? .03 : 0)));
                bRounding.push_back(roundingInput(card, "quick.b." + std::to_string(i)));
            }
        });
        p.heading(L"合成设置");
        double k = p.number("quick.k", L"包含因子 k", 2);
        int digits =
            p.select("quick.result.digits", L"合成有效数字", {L"1 位", L"2 位", L"3 位", L"4 位"}, 1) + 1;
        bool carry = p.select("quick.result.rounding", L"合成修约方式", {L"四舍六入", L"进位"}, 1) == 1;
        p.check();
        require(an + bn > 0, "至少需要一组 A 类数据或一个 B 类分量");
        require(k > 0, "包含因子须大于 0");
        double ua = 0;
        if (!av.empty()) {
            auto s = statistics(av);
            ua = averageCount ? s.sd / std::sqrt(double(*averageCount)) : s.sd;
        }
        double rawUa = ua;
        ua = roundSignificant(ua, aRounding);
        Values bRaw;
        Values bInputs = bv;
        for (std::size_t i = 0; i < bv.size(); ++i) {
            auto &b = bv[i];
            require(b >= 0, "B 类半宽不能为负");
            b /= distributionDivisor(distribution);
            bRaw.push_back(b);
            b = roundSignificant(b, bRounding[i]);
        }
        double ub = rss(bv), uc = std::hypot(ac * ua, bc * ub);
        finite({uc, k * uc});
        auto [rounded, roundedExpanded] = roundedCombinedUncertainty(uc, k, digits, carry);
        if (!bv.empty() && p.collapse("quick.details", L"B 类分量修约明细")) {
            for (std::size_t i = 0; i < bv.size(); ++i)
                p.note(L"B 类分量 " + std::to_wstring(i + 1) + L"：u=" + format(bRaw[i]) + L"，修约后 u=" +
                       formatRoundedUncertainty(bv[i], bRounding[i].digits));
        }
        p.metrics(L"不确定度结果",
                  {{L"A 类标准不确定度", format(rawUa)},
                   {L"A 类修约后标准不确定度", formatRoundedUncertainty(ua, aRounding.digits)},
                   {L"B 类合成标准不确定度", format(ub)},
                   {L"合成标准不确定度 u_c", format(uc)},
                   {L"扩展不确定度 U", format(k * uc)},
                   {L"修约后标准不确定度", format(rounded)},
                   {L"修约后扩展不确定度", format(roundedExpanded)},
                   {L"包含因子 k", format(k)}});
        if (!av.empty()) {
            p.processSteps.push_back(L"A 类分量：");
            auto stats = statistics(av);
            sampleProcess(p, av, stats, false);
            p.processSteps.push_back(L"取测量次数 n = " + std::to_wstring(averageCount.value_or(1)) + (averageCount ? L"" : L"（留空默认 1 次）") + L"（此 n 可不同于上方估计 s 的测量值个数）；uA = s/√n = " + p.processValue(stats.sd) + L" / √" + std::to_wstring(averageCount.value_or(1)) + L" = " + p.processValue(rawUa) + L"；" + roundingDescription(aRounding) + L"；修约后 uA = " + roundedProcess(p, ua, aRounding.digits) + L"。");
        }
        for (std::size_t i = 0; i < bv.size(); ++i) {
            p.processSteps.push_back(L"");
            p.processSteps.push_back(L"B 类分量 " + std::to_wstring(i + 1) + L"：" + distributions[std::size_t(distribution)] + L"；" + (distribution == 2 || distribution == 3 ? L"证书扩展不确定度" : L"区间半宽") + L" = " + inputNumber(bInputs[i]) + L"；u = 输入 / 除数 = " + inputNumber(bInputs[i]) + L" / " + inputNumber(distributionDivisor(distribution)) + L" = " + p.processValue(bRaw[i]) + L"；" + roundingDescription(bRounding[i]) + L"，修约后 = " + roundedProcess(p, bv[i], bRounding[i].digits) + L"。");
        }
        p.processSteps.push_back(L"");
        p.processSteps.push_back(L"B 类合成 uB = √Σuᵢ² = " + p.processValue(ub) + L"；u_c = √[(cA·uA)² + (cB·uB)²] = √[(" + inputNumber(ac) + L" × " + p.processValue(ua) + L")² + (" + inputNumber(bc) + L" × " + p.processValue(ub) + L")²] = " + p.processValue(uc) + L"。");
        p.processSteps.push_back(L"扩展不确定度 U = k × u_c = " + inputNumber(k) + L" × " + p.processValue(uc) + L" = " + p.processValue(k * uc) + L"；按 u_c 和 U 的较大值确定修约位数，保留 " + std::to_wstring(digits) + L" 位有效数字，" + (carry ? L"进位" : L"四舍六入、五成双") + L"；修约后 u_c = " + p.processValue(rounded) + L"，修约后 U = k × 修约后 u_c = " + p.processValue(roundedExpanded) + L"。");
        return;
    }
    int n = p.count("budget.count", L"不确定度分量", 2, 1, 20);
    std::vector<Component> data;
    componentCards(p, n, p.app.uncertaintyColumns[0], [&](Page &card, int i) {
        card.heading(L"分量 " + std::to_wstring(i + 1));
        data.push_back(component(card, "budget." + std::to_string(i), 0));
    });
    bool maximum = p.select("budget.method", L"总合成方法", {L"平方和根", L"最大绝对贡献"}) == 1;
    double k = p.number("budget.k", L"包含因子 k", 2);
    int digits = p.select("budget.digits", L"合成结果有效数字", {L"1 位", L"2 位", L"3 位", L"4 位"}, 1) + 1;
    bool carry = p.select("budget.round_v2", L"合成结果修约方式", {L"四舍六入", L"进位"}, 1) == 1;
    p.check();
    require(k > 0, "包含因子须大于 0");
    Values contributions;
    std::vector<UncertaintyDetail> details;
    for (auto &c : data) {
        details.push_back(c.calculate());
        contributions.push_back(details.back().contribution);
    }
    double uc = combineUncertainty(contributions, maximum), expanded = k * uc;
    finite({expanded});
    auto [rounded, roundedExpanded] = roundedCombinedUncertainty(uc, k, digits, carry);
    if (p.collapse("budget.details", L"分量计算明细")) {
        showDetails(p, details);
    }
    p.metrics(L"不确定度结果", {{L"合成标准不确定度 u_c", format(uc)},
                                {L"扩展不确定度 U", format(expanded)},
                                {L"修约后标准不确定度", format(rounded)},
                                {L"修约后扩展不确定度", format(roundedExpanded)},
                                {L"包含因子 k", format(k)}});
    for (std::size_t i = 0; i < data.size(); ++i)
        uncertaintyComponentProcess(p, data[i], details[i], L"");
    p.processSteps.push_back(L"");
    p.processSteps.push_back((maximum ? L"总合成方法：取各分量绝对贡献的最大值；u_c = " : L"总合成方法：各分量贡献平方和根；u_c = ") + p.processValue(uc) + L"。");
    p.processSteps.push_back(L"扩展不确定度 U = k × u_c = " + inputNumber(k) + L" × " + p.processValue(uc) + L" = " + p.processValue(expanded) + L"；按 u_c 和 U 的较大值确定修约位数，保留 " + std::to_wstring(digits) + L" 位有效数字，" + (carry ? L"进位" : L"四舍六入、五成双") + L"；修约后 u_c = " + p.processValue(rounded) + L"，修约后 U = k × 修约后 u_c = " + p.processValue(roundedExpanded) + L"。");
}
void renderErrors(Page &p) {
    if (p.tool.kind == Kind::Uncertainty) {
        uncertainty(p);
        return;
    }
    if (p.tool.kind == Kind::Repeatability) {
        int n = p.count("sample.count", L"测量值", 5, 2, 50);
        auto v = p.values("sample", L"测量值", n, sampleDefaults);
        p.check();
        auto s = statistics(v);
        finite({s.maximum - s.minimum});
        if (p.collapse("residual", L"各项残差")) {
            for (int i = 0; i < n; ++i) {
                p.note(L"测量 " + std::to_wstring(i + 1) + L"：" + format(s.residuals[i]));
            }
        }
        double rsd = s.mean == 0 ? 0 : s.sd / std::abs(s.mean) * 100;
        finite({rsd});
        p.metrics(L"统计结果", {{L"测量次数", std::to_wstring(n)},
                                {L"平均值", format(s.mean)},
                                {L"样本标准差 s", format(s.sd)},
                                {L"极差", format(s.maximum - s.minimum)},
                                {L"平均值标准差 s/√n", format(s.standardError)},
                                {L"相对标准偏差", s.mean == 0 ? L"平均值为 0" : format(rsd) + L" %"}});
        sampleProcess(p, v, s, true);
        p.processSteps.push_back(L"");
        p.processSteps.push_back(L"极差 R = 最大值 − 最小值 = " + inputNumber(s.maximum) + L" − " + inputNumber(s.minimum) + L" = " + p.processValue(s.maximum - s.minimum) + L"；平均值标准差 = s/√n = " + p.processValue(s.sd) + L" / √" + std::to_wstring(n) + L" = " + p.processValue(s.standardError) + L"。");
        p.processSteps.push_back(s.mean == 0 ? L"平均值为 0，相对标准偏差无定义。" : L"相对标准偏差 = s/|x̄| × 100 = " + p.processValue(s.sd) + L" / |" + p.processValue(s.mean) + L"| × 100 = " + p.processValue(rsd) + L"%。");
        return;
    }
    if (p.tool.kind == Kind::RangeMethod) {
        int mode = p.select("method", L"输入方式", {L"逐项测量值", L"最大值、最小值与测量次数"}),
            n = p.count("count", L"测量次数", 5, 2, 9);
        double lo, hi;
        if (mode == 0) {
            auto values = p.values("sample", L"测量值", n, sampleDefaults);
            p.check();
            auto bounds = std::minmax_element(values.begin(), values.end());
            lo = *bounds.first;
            hi = *bounds.second;
        } else {
            hi = p.number("max", L"最大值", 10.03);
            lo = p.number("min", L"最小值", 10.01);
            p.check();
            require(hi >= lo, "最大值不能小于最小值");
        }
        double range = hi - lo, coefficient = rangeCoefficient(n), sd = range / coefficient;
        finite({range});
        p.metrics(L"极差法结果", {{L"测量次数", std::to_wstring(n)},
                                  {L"极差 R", format(range)},
                                  {L"极差系数 d₂", format(coefficient)},
                                  {L"重复性标准差 s ≈ R/d₂", format(sd)},
                                  {L"平均值标准差 s/√n", format(sd / std::sqrt(double(n)))}});
        p.processSteps.push_back(L"极差 R = 最大值 − 最小值 = " + inputNumber(hi) + L" − " + inputNumber(lo) + L" = " + p.processValue(range) + L"。");
        p.processSteps.push_back(L"测量次数 n = " + std::to_wstring(n) + L"，查极差系数 d₂ = " + inputNumber(coefficient) + L"；s = R/d₂ = " + p.processValue(range) + L" / " + inputNumber(coefficient) + L" = " + p.processValue(sd) + L"。");
        p.processSteps.push_back(L"平均值标准差 = s/√n = " + p.processValue(sd) + L" / √" + std::to_wstring(n) + L" = " + p.processValue(sd / std::sqrt(double(n))) + L"。");
        return;
    }
    if (p.tool.kind == Kind::ErrorAnalysis) {
        double measured = p.number("measured", L"测量值 x", 100.02),
               truth = p.number("truth", L"真值 x₀", 100);
        auto reference = p.textInput("reference", L"引用值（选填）", L"");
        p.check();
        double ref = reference.empty() ? truth : parse(utf8(reference)), delta = measured - truth;
        finite({delta});
        auto percent = [&](double base) {
            if (base == 0) {
                return std::wstring(L"分母为 0，未定义");
            }
            double value = delta / base * 100;
            finite({value});
            return format(value) + L" %";
        };
        p.metrics(L"误差结果", {{L"绝对误差 Δ", format(delta)},
                                {L"相对误差 δ", percent(truth)},
                                {L"引用误差 γ", percent(ref)},
                                {L"绝对值", format(std::abs(delta))}});
        p.processSteps.push_back(L"绝对误差 Δ = 测量值 − 真值 = " + inputNumber(measured) + L" − " + inputNumber(truth) + L" = " + p.processValue(delta) + L"；误差绝对值 |Δ| = " + p.processValue(std::abs(delta)) + L"。");
        p.processSteps.push_back(L"");
        p.processSteps.push_back(truth == 0 ? L"真值为 0，相对误差无定义。" : L"相对误差 = Δ/真值 × 100 = " + p.processValue(delta) + L" / " + inputNumber(truth) + L" × 100 = " + p.processValue(delta / truth * 100) + L"%。");
        const std::wstring baseDescription = reference.empty() ? L"引用值留空，使用真值；" : L"";
        p.processSteps.push_back(baseDescription + (ref == 0 ? L"引用值为 0，引用误差无定义。" : std::wstring(L"引用误差 = Δ/引用值 × 100 = ") + p.processValue(delta) + L" / " + inputNumber(ref) + L" × 100 = " + p.processValue(delta / ref * 100) + L"%。"));
        return;
    }
    int mode = p.select("mode", L"分量输入", {L"同单位误差分量", L"压力分量（相对 / 绝对误差）"});
    Values contributions;
    std::wstring symbol;
    if (mode == 0) {
        int n = p.count("count", L"误差分量", 3, 1, 50);
        Values errors, sensitivity;
        const Values defaults{.02, .03, .01};
        for (int i = 0; i < n; ++i) {
            const float oldX = p.x, oldY = p.y, oldW = p.w;
            const bool columns = oldW >= 360;
            if (columns)
                p.w = (oldW - 16) / 2;
            errors.push_back(p.number("error." + std::to_string(i), L"误差分量 " + std::to_wstring(i + 1),
                                      i < int(defaults.size()) ? defaults[i] : 0));
            const float nextY = p.y;
            if (columns) {
                p.x = oldX + p.w + 16;
                p.y = oldY;
            }
            sensitivity.push_back(p.number("sensitivity." + std::to_string(i),
                                           L"灵敏系数 " + std::to_wstring(i + 1), i < 3 ? 1 : 0));
            p.x = oldX;
            p.w = oldW;
            p.y = std::max(p.y, nextY);
        }
        p.check();
        for (int i = 0; i < n; ++i) {
            contributions.push_back(errors[i] * sensitivity[i]);
            if (i) p.processSteps.push_back(L"");
            p.processSteps.push_back(L"分量 " + std::to_wstring(i + 1) + L"：贡献 = 误差 × 灵敏系数 = " + inputNumber(errors[i]) + L" × " + inputNumber(sensitivity[i]) + L" = " + p.processValue(contributions.back()) + L"。");
        }
    } else {
        auto list = units(Dimension::Pressure);
        std::vector<std::wstring> names;
        for (auto u : list) {
            names.push_back(u.code);
        }
        int target = p.select("pressure.target", L"结果单位", names),
            n = p.count("pressure.count", L"压力误差分量", 3, 1, 50);
        symbol = L" " + names[target];
        for (int i = 0; i < n; ++i) {
            p.heading(L"分量 " + std::to_wstring(i + 1));
            std::string k = "pressure." + std::to_string(i);
            int source = p.select(k + ".unit", L"分量单位", names);
            bool relative = p.select(k + ".type", L"误差类型", {L"相对误差（量程百分比）", L"绝对误差"}) == 0;
            double span = 1;
            if (relative) {
                double lo = p.number(k + ".low", L"量程下限", 0),
                       hi = p.number(k + ".high", L"量程上限", 100);
                p.check();
                span = Range(lo, hi).span;
            }
            double error = p.number(k + ".error", relative ? L"误差 (%)" : L"绝对误差", .02);
            p.check();
            contributions.push_back(
                convert(Dimension::Pressure, error * (relative ? span / 100 : 1), source, target));
            if (i) p.processSteps.push_back(L"");
            if (relative)
                p.processSteps.push_back(L"分量 " + std::to_wstring(i + 1) + L"：量程跨度 = " + p.processValue(span) + L" " + names[source] + L"；绝对误差 = 相对误差 × 跨度 / 100 = " + inputNumber(error) + L" × " + p.processValue(span) + L" / 100 = " + p.processValue(error * span / 100) + L" " + names[source] + L"。");
            else
                p.processSteps.push_back(L"分量 " + std::to_wstring(i + 1) + L"：输入绝对误差 = " + inputNumber(error) + L" " + names[source] + L"。");
            p.processSteps.push_back(L"统一单位：" + p.processValue(error * (relative ? span / 100 : 1)) + L" " + names[source] + L" × (" + inputNumber(list[source].factor) + L" / " + inputNumber(list[target].factor) + L") = " + p.processValue(contributions.back()) + L" " + names[target] + L"。");
        }
    }
    double combined = rss(contributions), linear = 0, largest = 0;
    for (double c : contributions) {
        linear += std::abs(c);
        largest = std::max(largest, std::abs(c));
    }
    finite({linear});
    Metrics metrics{{L"分量个数", std::to_wstring(contributions.size())},
                    {L"合成误差（平方和根）", format(combined) + symbol}};
    if (mode == 0) {
        metrics.insert(metrics.end(),
                       {{L"最坏情况界限（绝对值求和）", format(linear)}, {L"最大单项贡献", format(largest)}});
    }
    p.metrics(L"合成结果", metrics);
    p.processSteps.push_back(L"");
    p.processSteps.push_back(L"合成误差 = √Σqᵢ² = " + p.processValue(combined) + symbol + L"；程序使用 hypot 累加。");
    if (mode == 0)
        p.processSteps.push_back(L"最坏情况界限 = Σ|qᵢ| = " + p.processValue(linear) + L"；最大单项贡献 = " + p.processValue(largest) + L"。");
}
} // 命名空间 toolbox
