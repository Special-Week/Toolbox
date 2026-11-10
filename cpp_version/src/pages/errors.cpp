// 误差统计和不确定度预算。折叠只控制展示，已隐藏的分量仍参与合成。
#include "toolbox/gui.hpp"
#include "toolbox/card_columns.hpp"
namespace toolbox {
static const Values sampleDefaults{10.02, 10.01, 10.03, 10.02, 10.01};
static const std::vector<std::wstring> distributions{L"均匀分布", L"三角分布", L"正态 k=2",
                                                     L"正态 k=3", L"两点分布", L"反正弦分布"};
static std::optional<std::size_t> averageCountInput(Page &p, const std::string &key) {
    const auto text = p.textInput(key, L"取测量次数 n", L"");
    p.note(L"留空默认 1 次。");
    try { return parseAverageCount(text); }
    catch (const std::exception &e) {
        if (p.inputError.empty()) p.inputError = wide(e.what());
        return std::nullopt;
    }
}
static UncertaintyRounding roundingInput(Page &p, const std::string &key) {
    const auto text = p.textInput(key + ".digits", L"有效数字", L"");
    p.note(L"留空保留原始计算精度（1～17 位）。");
    const bool carry = p.select(key + ".rounding", L"修约方式", {L"四舍六入", L"进位"}, 1) == 1;
    try { return {parseSignificantDigits(text), carry}; }
    catch (const std::exception &e) {
        if (p.inputError.empty()) p.inputError = wide(e.what());
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
        p.x = cardX + 16; p.y = cardY + 16; p.w = cardWidth - 32;
        render(p, i);
        const float bottom = p.y + 16;
        p.ui.draws[fill].rect.h = bottom - cardY;
        p.ui.draws[fill + 1].rect.h = bottom - cardY;
        heights[column] = bottom + 8;
    }
    p.x = left; p.w = width; p.y = std::max(heights[0], heights[1]);
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
        if (!c.useRange) c.averageCount = averageCountInput(p, key + ".average_count");
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
                if (an) averageCount = averageCountInput(card, "quick.a.average_count");
                ac = card.number("quick.a.c", L"A 类灵敏系数", 1);
                if (an) aRounding = roundingInput(card, "quick.a");
            } else {
                int i = index - 1;
                card.heading(L"B 类分量 " + std::to_wstring(i + 1));
                auto label = distribution == 2 || distribution == 3 ? L"扩展不确定度 U" : L"半宽";
                bv.push_back(card.number("quick.b.value." + std::to_string(i), label, i == 0 ? .02 : (i == 1 ? .03 : 0)));
                bRounding.push_back(roundingInput(card, "quick.b." + std::to_string(i)));
            }
        });
        p.heading(L"合成设置");
        double k = p.number("quick.k", L"包含因子 k", 2);
        int digits = p.select("quick.result.digits", L"合成有效数字", {L"1 位", L"2 位", L"3 位", L"4 位"}, 1) + 1;
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
                p.note(L"B 类分量 " + std::to_wstring(i + 1) + L"：u=" + format(bRaw[i]) +
                       L"，修约后 u=" + formatRoundedUncertainty(bv[i], bRounding[i].digits));
        }
        p.metrics(L"不确定度结果", {{L"A 类标准不确定度", format(rawUa)},
                                    {L"A 类修约后标准不确定度", formatRoundedUncertainty(ua, aRounding.digits)},
                                    {L"B 类合成标准不确定度", format(ub)},
                                    {L"合成标准不确定度 u_c", format(uc)},
                                    {L"扩展不确定度 U", format(k * uc)},
                                    {L"修约后标准不确定度", format(rounded)},
                                    {L"修约后扩展不确定度", format(roundedExpanded)},
                                    {L"包含因子 k", format(k)}});
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
                p.note(L"测量 " + std::to_wstring(i + 1) + L"：" + format(v[i] - s.mean));
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
            if (columns) p.w = (oldW - 16) / 2;
            errors.push_back(p.number("error." + std::to_string(i), L"误差分量 " + std::to_wstring(i + 1),
                                      i < int(defaults.size()) ? defaults[i] : 0));
            const float nextY = p.y;
            if (columns) {
                p.x = oldX + p.w + 16;
                p.y = oldY;
            }
            sensitivity.push_back(p.number("sensitivity." + std::to_string(i), L"灵敏系数 " + std::to_wstring(i + 1),
                                           i < 3 ? 1 : 0));
            p.x = oldX;
            p.w = oldW;
            p.y = std::max(p.y, nextY);
        }
        p.check();
        for (int i = 0; i < n; ++i) {
            contributions.push_back(errors[i] * sensitivity[i]);
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
}
} // namespace toolbox
