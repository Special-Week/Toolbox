// M、Tr 与钢筋量规。工件偏差、量规位置及制造公差分别计算，避免混用尺寸基准。
#include "toolbox/gauges.hpp"

namespace toolbox::gauges {
Gauge::Gauge(std::string c, std::string n, double nominal, double tolerance)
    : code(std::move(c)), name(std::move(n)), pd{nominal - tolerance / 2, nominal + tolerance / 2},
      center(nominal) {}
std::vector<Gauge> plugSet(double major, double low, double high, double minor, Factors f, bool trap,
                           double truncation) {
    Gauge go("T", "通端工作塞规", low + f.zpl, f.tpl);
    go.major = {major + f.zpl - f.tpl, major + f.zpl + f.tpl};
    go.minorMax = minor;
    go.wear = go.pd[0] - f.sgo;
    const double center = high + (trap ? f.tpl / 2 : f.tpl);
    Gauge ng("Z", "止端工作塞规", center, f.tpl);
    ng.pd = {high, high + f.tpl};
    double dc = trap ? center + truncation : high + f.tpl + truncation + f.tpl / 2;
    ng.major = trap ? std::array<double, 2>{dc - f.tpl, dc + f.tpl} : std::array<double, 2>{dc - f.tpl, dc};
    ng.minorMax = minor;
    ng.wear = ng.pd[0] - f.sng;
    return {go, ng};
}
std::pair<std::vector<Gauge>, std::vector<Gauge>> ringSet(double d, double d2, double d1, double es,
                                                          double td, Factors f, double truncation) {
    Gauge go("T环", "通端工作环规", d2 + es - f.zr, f.tr);
    go.wear = go.center + f.rgo;
    go.majorMin = d + es + f.tpl;
    go.minor = {d1 + es - f.tr / 2, d1 + es + f.tr / 2};
    Gauge ng("Z环", "止端工作环规", d2 + es - td - f.tr / 2, f.tr);
    ng.wear = ng.center + f.rng;
    ng.majorMin = d + es + f.tpl;
    ng.minor = {ng.center - truncation - f.tr, ng.center - truncation + f.tr};
    struct Spec {
        const char *code, *name;
        double pd, major, tol, minor;
    };
    const Spec specs[] = {{"TT", "校通-通", go.center - f.m, d + es, f.tpl, d1 + es - f.zr - f.m},
                          {"TZ", "校通-止", go.pd[1], go.pd[1] + truncation, f.tpl / 2, d1 + es - f.tr / 2},
                          {"TS", "校通-损", go.wear, go.wear + truncation, f.tpl / 2, d1 + es - f.tr / 2},
                          {"ZT", "校止-通", ng.center - f.m, d + es, f.tpl, d1 + es - td - f.tr / 2 - f.m},
                          {"ZZ", "校止-止", d2 + es - td, d + es - td, f.tpl, d1 + es - td},
                          {"ZS", "校止-损", ng.wear, d + es - td - f.tr / 2 + f.rng, f.tpl, d1 + es - td}};
    std::vector<Gauge> cal;
    for (const auto &s : specs) {
        Gauge g(s.code, s.name, s.pd, f.tcp);
        g.major = {s.major - s.tol, s.major + s.tol};
        g.minorMax = s.minor;
        cal.push_back(g);
    }
    return {{go, ng}, cal};
}
Table gaugeTable(const std::string &title, const std::vector<Gauge> &gauges, bool includeWear) {
    Table table{title,
                {"代号 / 名称", "中径下限 (mm)", "中径上限 (mm)", "中径名义 (mm)", "大径下限 (mm)",
                 "大径上限 (mm)", "小径界限 (mm)"},
                {}};
    if (includeWear) {
        table.headings.push_back("磨损极限 (mm)");
    }
    table.headings.push_back("螺纹长度 (mm)");
    for (const auto &g : gauges) {
        Cell minor("—");
        if (valid(g.minorMax)) {
            minor = Cell::limit(g.minorMax, "≤");
        } else if (same(g.minor[0], g.minor[1])) {
            minor = Cell::limit(g.minor[0], "≥");
        } else if (valid(g.minor[0])) {
            minor = Cell::pair(g.minor[0], g.minor[1], " ～ ");
        }
        std::vector<Cell> row{g.code + " · " + g.name,
                              g.pd[0],
                              g.pd[1],
                              g.center,
                              valid(g.majorMin) ? Cell::limit(g.majorMin, "≥") : Cell(g.major[0]),
                              g.major[1],
                              minor};
        if (includeWear) {
            row.push_back(g.wear);
        }
        row.push_back(g.length);
        table.rows.push_back(std::move(row));
    }
    return table;
}
void factorMetrics(Report &out, Factors f, bool ring) {
    out.metric("工作塞规公差 TPL (μm)", f.tpl * 1000);
    out.metric("塞规位置 ZPL (μm)", f.zpl * 1000);
    if (ring) {
        out.metric("环规公差 TR (μm)", f.tr * 1000);
        out.metric("校对规公差 TCP (μm)", f.tcp * 1000);
        out.metric("环规位置 ZR (μm)", f.zr * 1000);
        out.metric("校对位置 m (μm)", f.m * 1000);
        out.metric("环规磨损 WGO / WNG (μm)", Cell::pair(f.rgo * 1000, f.rng * 1000));
    }
    out.metric("塞规磨损 WGO / WNG (μm)", Cell::pair(f.sgo * 1000, f.sng * 1000));
}
void referenceTable(Report &out, const std::string &name, double base, double low, double high,
                    std::optional<double> tol) {
    Table table{name + "参考界限（非独立制造公差）",
                {"基本尺寸 (mm)", "参考下界 (mm)", "参考上界 (mm)", "采用" + name + "公差 (μm)"},
                {}};
    table.rows.push_back({base, tol ? low : missing, tol ? high : missing, tol.value_or(missing)});
    out.tables.push_back(table);
    out.process.push_back(L"");
    out.process.push_back(wide(name) + L"参考界限（非独立制造公差）：基本尺寸 = " + traceNumber(base) + L" mm。" + (tol ? L" 查表采用公差 = " + traceNumber(*tol) + L" μm，换算为 mm 后得参考下界 " + traceNumber(low) + L"、上界 " + traceNumber(high) + L" mm。" : L"未收录该组合的公差，不计算界限。"));
    if (!tol) {
        out.note("该组合的" + name + "公差未收录，只列基本尺寸，不据此判定" + name + "合格。");
    }
}
std::vector<std::string> grades(bool trap, double d, double p) {
    std::vector<std::string> out;
    const auto &db = database()["DATA"][trap ? "trap" : "metric"];
    for (bool external : {false, true}) {
        std::vector<int> numbers = trap       ? std::vector<int>{7, 8, 9}
                                   : external ? std::vector<int>{3, 4, 5, 6, 7, 8, 9}
                                              : std::vector<int>{4, 5, 6, 7, 8};
        std::vector<std::string> letters =
            trap ? (external ? std::vector<std::string>{"c", "e", "h"} : std::vector<std::string>{"H"})
                 : (external ? std::vector<std::string>{"e", "f", "g", "h"}
                             : std::vector<std::string>{"H", "G"});
        for (int num : numbers) {
            for (const auto &letter : letters) {
                if (!trap && !external && num < 5 && letter == "G") {
                    continue;
                }
                try {
                    std::vector<std::string> ignored;
                    auto tol = tolerance(db[external ? "td2o" : "td2"], d, p, std::to_string(num), ignored);
                    if (external) {
                        deviation(db["ei"], tol.second,
                                  trap ? "es(" + letter + ")" : "外螺纹" + letter + "-es");
                    }
                    out.push_back(std::to_string(num) + letter);
                } catch (const std::exception &) {
                }
            }
        }
    }
    return out;
}
Report thread(bool trap, double d, double p, const std::string &grade, int lines) {
    const double d2 = d - (trap ? .5 : 3 * std::sqrt(3.) / 8) * p,
                 d1 = d - (trap ? 1. : 5 * std::sqrt(3.) / 8) * p;
    positiveGeometry(d, p, d2, d1);
    require(lines >= 1 && lines <= 5 && (trap || lines == 1), "线数范围不正确");
    const auto available = grades(trap, d, p);
    require(std::find(available.begin(), available.end(), grade) != available.end(),
            "当前直径、螺距没有该公差带的定义");
    const std::string num = grade.substr(0, grade.size() - 1), letter = grade.substr(grade.size() - 1);
    const bool ring = letter[0] >= 'a' && letter[0] <= 'z';
    const auto &db = database()["DATA"][trap ? "trap" : "metric"];
    Report out(std::string(trap ? "Tr" : "M") + rawNumber(d) + "×" + rawNumber(p) + " - " + grade);
    geometry(out, d, p, d2, d1, trap ? 30 : 60, lines);
    const auto toleranceValue = tolerance(db[ring ? "td2o" : "td2"], d, p, num, out.notes);
    const double kv = trap ? std::array<double, 5>{1, 1.12, 1.25, 1.4, 1.6}[lines - 1] : 1,
                 td = toleranceValue.first * kv, used = toleranceValue.second;
    const double offset =
        ring ? deviation(db["ei"], used, trap ? "es(" + letter + ")" : "外螺纹" + letter + "-es")
             : (letter == "H" ? 0 : deviation(db["ei"], used, "内螺纹G-EI"));
    const auto f = factors(trap, td, offset < 0);
    auto c = traceNumber;
    out.process.push_back(L"");
    out.process.push_back(std::wstring(trap ? L"基本中径 D₂ = D − P/2；基本小径 D₁ = D − P。" : L"基本中径 D₂ = D − 3√3P/8；基本小径 D₁ = D − 5√3P/8。"));
    out.process.push_back(L"工件中径公差按尺寸、公差等级及取值螺距 " + c(used) + L" mm 查表；多线系数 = " + c(kv) + L"，公差 = " + c(td) + L" μm，基本偏差 = " + c(offset) + L" μm。代入尺寸公式时除以 1000。");
    out.process.push_back(L"量规要素按工件公差和偏差符号查表并换算为 mm：TPL = " + c(f.tpl) + L"，ZPL = " + c(f.zpl) + L"，TR = " + c(f.tr) + L"，ZR = " + c(f.zr) + L"，TCP = " + c(f.tcp) + L"，m = " + c(f.m) + L"；塞规通/止磨损储备 = " + c(f.sgo) + L" / " + c(f.sng) + L"，环规通/止磨损储备 = " + c(f.rgo) + L" / " + c(f.rng) + L" mm。");
    out.metric("工件中径公差 (μm)", td);
    out.metric(ring ? "基本偏差 es (μm)" : "基本偏差 EI (μm)", offset);
    out.parameter("公差取值螺距 (mm)", used);
    if (trap) {
        out.metric("多线公差系数", kv);
    }
    factorMetrics(out, f, ring);
    if (auto row = closestPitch(db["half"], used)) {
        out.metric("完整牙型半角偏差 (角分)", first(*row, {"完整螺纹牙型的半角偏差", "完整牙型偏差"}));
        out.metric("截断牙型半角偏差 (角分)", first(*row, {"截断螺纹牙型的半角偏差", "截断牙型偏差"}));
    }
    const double ac = p <= 1.5 ? .15 : p <= 5 ? .25 : p <= 12 ? .5 : 1, truncation = trap ? 2 * ac : .2 * p;
    if (trap) {
        out.metric("牙顶间隙 ac (mm)", ac);
        out.metric("内螺纹基本大径 D₄ (mm)", d + 2 * ac);
    }
    if (ring) {
        auto sets = ringSet(d, d2, d1, offset / 1000, td / 1000, f, truncation);
        auto len = lengths(trap, true, d, p, out.notes);
        sets.first[0].length = len.first;
        sets.first[1].length = len.second;
        const auto &go = sets.first[0], &ng = sets.first[1];
        out.process.push_back(L"");
        out.process.push_back(L"通端工作环规：中径名义 = D₂ + es − ZR = " + c(d2) + L" + " + c(offset/1000) + L" − " + c(f.zr) + L" = " + c(go.center) + L"；中径界限 = 名义 ± TR/2 = " + c(go.pd[0]) + L"～" + c(go.pd[1]) + L" mm；磨损极限 = 名义 + 通端磨损储备 = " + c(go.wear) + L" mm。");
        out.process.push_back(L"大径下限 = D + es + TPL = " + c(go.majorMin) + L" mm；小径界限 = D₁ + es ± TR/2 = " + c(go.minor[0]) + L"～" + c(go.minor[1]) + L" mm。");
        out.process.push_back(L"");
        out.process.push_back(L"止端工作环规：中径名义 = D₂ + es − Td − TR/2 = " + c(ng.center) + L" mm；中径界限 = 名义 ± TR/2 = " + c(ng.pd[0]) + L"～" + c(ng.pd[1]) + L" mm；磨损极限 = 名义 + 止端磨损储备 = " + c(ng.wear) + L" mm。");
        out.process.push_back(L"大径下限 = D + es + TPL = " + c(ng.majorMin) + L" mm；小径界限 = 止端中径名义 − 截短量 ± TR = " + c(ng.minor[0]) + L"～" + c(ng.minor[1]) + L" mm；截短量 = " + c(truncation) + L" mm。");
        const wchar_t *centers[] = {L"通端中径名义 − m",L"通端中径上限",L"通端磨损极限",L"止端中径名义 − m",L"D₂ + es − Td",L"止端磨损极限"};
        const wchar_t *majors[] = {L"D + es",L"通端中径上限 + 截短量",L"通端磨损极限 + 截短量",L"D + es",L"D + es − Td",L"D + es − Td − TR/2 + 止端磨损储备"};
        const wchar_t *minors[] = {L"D₁ + es − ZR − m",L"D₁ + es − TR/2",L"D₁ + es − TR/2",L"D₁ + es − Td − TR/2 − m",L"D₁ + es − Td",L"D₁ + es − Td"};
        for (std::size_t i = 0; i < sets.second.size(); ++i) {
            const auto &g = sets.second[i];
            out.process.push_back(L"");
            out.process.push_back(wide(g.code + " " + g.name) + L"：中径名义 = " + centers[i] + L" = " + c(g.center) + L" mm；中径界限 = 名义 ± TCP/2 = " + c(g.pd[0]) + L"～" + c(g.pd[1]) + L" mm。");
            out.process.push_back(L"大径基准 = " + std::wstring(majors[i]) + L"，大径界限 = 基准 ± " + c((i == 1 || i == 2) ? f.tpl/2 : f.tpl) + L" = " + c(g.major[0]) + L"～" + c(g.major[1]) + L" mm；小径上限 = " + minors[i] + L" = " + c(g.minorMax) + L" mm。");
        }
        out.tables.push_back(gaugeTable("工作环规 · 检外螺纹", sets.first));
        out.tables.push_back(gaugeTable("校对塞规 · 检工作环规", sets.second));
        if (trap) {
            const double base = d - p - 2 * ac;
            std::optional<double> minor;
            if (lines == 1) {
                minor = trapezoidalMinorTolerance(d, used, num, letter, out.notes);
            } else {
                out.note("未收录多线外螺纹小径公差修正规则，不沿用单线小径极限。");
            }
            referenceTable(out, "小径", base, base - minor.value_or(0) / 1000, base, minor);
        } else {
            auto majorTol = minorTolerance(db["td"], used, num);
            double upper = d + offset / 1000;
            referenceTable(out, "大径", d, upper - majorTol.value_or(0) / 1000, upper, majorTol);
            out.note("未收录普通外螺纹小径的独立极限规则，不使用大径公差推算小径。");
        }
    } else {
        const double minor = d1 + offset / 1000;
        auto work = plugSet(trap ? d + 2 * ac : d + offset / 1000, d2 + offset / 1000,
                            d2 + offset / 1000 + td / 1000, minor, f, trap, truncation);
        std::pair<double, double> len;
        if (trap) {
            len = lengths(true, false, d, p, out.notes);
        } else {
            auto m = metricLengths(d, p, out.notes);
            out.metric("塞规总长 (mm)", m.total);
            out.metric("配套手柄", m.handle);
            len = {m.go, m.noGo};
        }
        work[0].length = len.first;
        work[1].length = len.second;
        const auto &go = work[0], &ng = work[1];
        double major = trap ? d + 2 * ac : d + offset / 1000, low = d2 + offset / 1000, high = low + td / 1000;
        out.process.push_back(L"");
        out.process.push_back(L"通端工作塞规：中径名义 = 工件中径下限 + ZPL = " + c(low) + L" + " + c(f.zpl) + L" = " + c(go.center) + L" mm；中径界限 = 名义 ± TPL/2 = " + c(go.pd[0]) + L"～" + c(go.pd[1]) + L" mm。");
        out.process.push_back(L"大径界限 = 大径基准 + ZPL ± TPL = " + c(major) + L" + " + c(f.zpl) + L" ± " + c(f.tpl) + L" = " + c(go.major[0]) + L"～" + c(go.major[1]) + L" mm；小径上限 = " + c(minor) + L" mm；磨损极限 = 通端中径下限 − 通端磨损储备 = " + c(go.wear) + L" mm。");
        out.process.push_back(L"");
        out.process.push_back(L"止端工作塞规：中径下限 = 工件中径上限 = " + c(high) + L" mm；中径上限 = 工件中径上限 + TPL = " + c(ng.pd[1]) + L" mm；中径名义 = " + c(ng.center) + L" mm。" + (trap ? L"名义值按上限 + TPL/2 计算。" : L"本模型名义值按工件上限 + TPL 计算，并非界限中点。"));
        out.process.push_back(std::wstring(trap ? L"大径基准 = 中径名义 + 截短量；大径界限 = 基准 ± TPL。" : L"大径上限 = 工件上限 + TPL + 截短量 + TPL/2；大径下限 = 大径上限 − TPL。") + L"截短量 = " + c(truncation) + L" mm；大径界限 = " + c(ng.major[0]) + L"～" + c(ng.major[1]) + L" mm；小径上限 = " + c(ng.minorMax) + L" mm；磨损极限 = 止端中径下限 − 止端磨损储备 = " + c(ng.wear) + L" mm。");
        if (valid(len.first) && valid(len.second)) out.process.push_back(L"螺纹长度按规格查表：通端 = " + c(len.first) + L" mm，止端 = " + c(len.second) + L" mm。");
        out.tables.push_back(gaugeTable("工作塞规 · 检内螺纹", work));
        auto td1 = minorTolerance(db["td1"], used, trap ? "4" : num);
        double base = trap ? d1 : d1 + offset / 1000;
        referenceTable(out, "小径", d1, base, base + td1.value_or(0) / 1000, td1);
    }
    out.notes.erase(std::unique(out.notes.begin(), out.notes.end()), out.notes.end());
    out.checked();
    return out;
}
Report rebar(int angle, std::size_t index, int lines) {
    const auto &rows = database()["REBAR"][std::to_string(angle)].array;
    require(index < rows.size(), "请选择有效钢筋规格");
    require(lines >= 1 && lines <= 4, "钢筋头数须为 1～4");
    const auto &row = rows[index];
    double p = number(row[2]), d = number(row[3]), d2 = number(row[4]), d1 = number(row[5]);
    Report out("钢筋直螺纹 " + row[0].string + " · " + row[1].string + " · " + std::to_string(angle) + "°");
    geometry(out, d, p, d2, d1, angle, lines);
    const auto &db = database()["DATA"]["metric"];
    auto ti = tolerance(db["td2"], d, p, "6", out.notes), te = tolerance(db["td2o"], d, p, "6", out.notes);
    double es = deviation(db["ei"], te.second, "外螺纹f-es") / 1000;
    auto fi = factors(false, ti.first, false), fe = factors(false, te.first, true);
    out.metric("套筒 6H 中径公差 (μm)", ti.first);
    out.metric("丝头 6f 中径公差 (μm)", te.first);
    out.metric("丝头 es (μm)", es * 1000);
    auto start = out.metrics.size();
    factorMetrics(out, fi, false);
    for (std::size_t i = start; i < out.metrics.size(); ++i) {
        out.metrics[i].first = "套筒塞规 · " + out.metrics[i].first;
    }
    start = out.metrics.size();
    factorMetrics(out, fe, true);
    for (std::size_t i = start; i < out.metrics.size(); ++i) {
        out.metrics[i].first = "丝头环规 / 校对规 · " + out.metrics[i].first;
    }
    auto plugs = plugSet(d, d2, d2 + ti.first / 1000, d1, fi, false, .2 * p);
    auto sets = ringSet(d, d2, d1, es, te.first / 1000, fe, .2 * p);
    double zzMajor = d2 + es - te.first / 1000, zsMajor = zzMajor - fe.tr / 2 + fe.rng;
    sets.second[4].major = {zzMajor - fe.tpl, zzMajor + fe.tpl};
    sets.second[5].major = {zsMajor - fe.tpl, zsMajor + fe.tpl};
    auto c = traceNumber;
    out.process.push_back(L"钢筋规格的大径、中径、小径和螺距直接查表；套筒 6H 与丝头 6f 中径公差分别查表 = " + c(ti.first) + L" / " + c(te.first) + L" μm，丝头 es = " + c(es) + L" mm。多头不额外增加公差系数。");
    out.process.push_back(L"套筒量规 TPL = " + c(fi.tpl) + L"，ZPL = " + c(fi.zpl) + L" mm；丝头量规 TPL = " + c(fe.tpl) + L"，TR = " + c(fe.tr) + L"，ZR = " + c(fe.zr) + L"，TCP = " + c(fe.tcp) + L"，m = " + c(fe.m) + L" mm。磨损储备均按公差档查表。");
    out.process.push_back(L"");
    out.process.push_back(L"套筒通端塞规：中径名义 = D₂ + ZPL = " + c(plugs[0].center) + L" mm，中径界限 = 名义 ± TPL/2 = " + c(plugs[0].pd[0]) + L"～" + c(plugs[0].pd[1]) + L" mm；大径界限 = D + ZPL ± TPL = " + c(plugs[0].major[0]) + L"～" + c(plugs[0].major[1]) + L" mm，小径上限 = D₁，磨损极限 = 中径下限 − 通端磨损储备 = " + c(plugs[0].wear) + L" mm。");
    out.process.push_back(L"");
    out.process.push_back(L"套筒止端塞规：中径下限 = D₂ + 工件公差，止端上限 = 下限 + TPL = " + c(plugs[1].pd[0]) + L"～" + c(plugs[1].pd[1]) + L" mm；大径上限 = 工件中径上限 + TPL + 0.2P + TPL/2，下限 = 上限 − TPL = " + c(plugs[1].major[0]) + L"～" + c(plugs[1].major[1]) + L" mm；磨损极限 = 中径下限 − 止端磨损储备 = " + c(plugs[1].wear) + L" mm。");
    out.process.push_back(L"");
    out.process.push_back(L"丝头通端环规：中径名义 = D₂ + es − ZR = " + c(sets.first[0].center) + L" mm，中径界限 = 名义 ± TR/2 = " + c(sets.first[0].pd[0]) + L"～" + c(sets.first[0].pd[1]) + L" mm；磨损极限 = 名义 + 通端磨损储备 = " + c(sets.first[0].wear) + L" mm；小径界限 = D₁ + es ± TR/2 = " + c(sets.first[0].minor[0]) + L"～" + c(sets.first[0].minor[1]) + L" mm。");
    out.process.push_back(L"丝头止端环规：中径名义 = D₂ + es − Td − TR/2 = " + c(sets.first[1].center) + L" mm，中径界限 = 名义 ± TR/2 = " + c(sets.first[1].pd[0]) + L"～" + c(sets.first[1].pd[1]) + L" mm；磨损极限 = 名义 + 止端磨损储备 = " + c(sets.first[1].wear) + L" mm；小径界限 = 名义 − 0.2P ± TR = " + c(sets.first[1].minor[0]) + L"～" + c(sets.first[1].minor[1]) + L" mm。两种环规大径下限均为 D + es + TPL = " + c(sets.first[0].majorMin) + L" mm。");
    const wchar_t *rebarCenters[] = {L"通端中径名义 − m",L"通端中径上限",L"通端磨损极限",L"止端中径名义 − m",L"D₂ + es − Td",L"止端磨损极限"};
    const wchar_t *rebarMajors[] = {L"D + es",L"通端中径上限 + 0.2P",L"通端磨损极限 + 0.2P",L"D + es",L"D₂ + es − Td",L"D₂ + es − Td − TR/2 + 止端磨损储备"};
    const wchar_t *rebarMinors[] = {L"D₁ + es − ZR − m",L"D₁ + es − TR/2",L"D₁ + es − TR/2",L"D₁ + es − Td − TR/2 − m",L"D₁ + es − Td",L"D₁ + es − Td"};
    for (std::size_t i=0; i<sets.second.size(); ++i) {
        const auto &g = sets.second[i];
        out.process.push_back(L"");
        out.process.push_back(wide(g.code + " " + g.name) + L"：中径名义 = " + rebarCenters[i] + L" = " + c(g.center) + L" mm；界限 = 名义 ± TCP/2 = " + c(g.pd[0]) + L"～" + c(g.pd[1]) + L" mm。大径基准 = " + rebarMajors[i] + L"，界限为基准 ± " + c(i == 1 || i == 2 ? fe.tpl/2 : fe.tpl) + L" = " + c(g.major[0]) + L"～" + c(g.major[1]) + L" mm；小径上限 = " + rebarMinors[i] + L" = " + c(g.minorMax) + L" mm。");
    }
    out.tables.push_back(gaugeTable("套筒内螺纹 6H · 工作塞规", plugs));
    out.tables.push_back(gaugeTable("丝头外螺纹 6f · 工作环规", sets.first));
    out.tables.push_back(gaugeTable("校对塞规", sets.second));
    auto td1 = minorTolerance(db["td1"], ti.second, "6");
    referenceTable(out, "小径", d1, d1, d1 + td1.value_or(0) / 1000, td1);
    out.note("工件公差和量规位置要素为借用体系；多头不另增公差系数。具体旋合判定按产品实际执行标准。复核牙型"
             "与标准后使用。");
    out.checked();
    return out;
}
} // 命名空间 toolbox::gauges
