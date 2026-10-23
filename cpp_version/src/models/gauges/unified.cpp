// UN 采用与 Rust 相同的工件模型和制造公差 U；所有英寸运算保留原始精度。
#include "toolbox/gauges.hpp"
#include <cctype>
#include <sstream>

namespace toolbox::gauges {
namespace {
std::string trim(const std::string &s) {
    auto a = s.find_first_not_of(" \r\n\t"), b = s.find_last_not_of(" \r\n\t");
    return a == std::string::npos ? "" : s.substr(a, b - a + 1);
}
std::string lowerAscii(std::string s) {
    for (auto &c : s) {
        c = char(std::tolower(static_cast<unsigned char>(c)));
    }
    return s;
}
const Data *band(const Data &table, double d, double tpi) {
    for (const auto &row : table.array) {
        double teeth = n(row, "牙数");
        if (!std::isfinite(teeth)) {
            try {
                teeth = fraction(row["牙数"].string);
            } catch (const std::exception &) {
            }
        }
        if (same(teeth, tpi) && d > first(row, {"公称直径>", "公称直径＞"}) && d <= n(row, "公称直径≤")) {
            return &row;
        }
    }
    return nullptr;
}
double optionalMm(const Data *row, const char *key) {
    if (!row) {
        return missing;
    }
    double v = n(*row, key);
    return valid(v) && v > 0 ? v * inch : missing;
}
double manufacturing(double d, double tpi) {
    const Data *hit = nullptr;
    for (const auto &r : unifiedData()["manufacturing"].array) {
        if (same(n(r, "tpi"), tpi) && d > n(r, "diameter_lower_in") && d <= n(r, "diameter_upper_in")) {
            hit = &r;
            break;
        }
    }
    require(hit != nullptr, "未收录该牙数或直径档的中径制造公差 U");
    double u = n(*hit, "tolerance_in");
    require(valid(u) && u > 0, "该牙数与直径组合的中径制造公差 U 未定义，请核对规格");
    return u * inch;
}
} // namespace
double fraction(const std::string &text) {
    auto cleaned = trim(text);
    require(cleaned.empty() || cleaned[0] != '-', "公称直径须大于零");
    require(std::count(cleaned.begin(), cleaned.end(), '-') <= 1, "英寸带分数格式无效");
    while (!cleaned.empty() && cleaned.back() == '"') {
        cleaned.pop_back();
    }
    const std::string quote = "″";
    while (cleaned.size() >= quote.size() && cleaned.substr(cleaned.size() - quote.size()) == quote) {
        cleaned.resize(cleaned.size() - quote.size());
    }
    std::replace(cleaned.begin(), cleaned.end(), '-', ' ');
    std::istringstream stream(cleaned);
    std::vector<std::string> parts;
    std::string part;
    while (stream >> part) {
        parts.push_back(part);
    }
    require(!parts.empty() && parts.size() <= 2, "英寸直径可填 0.5、1/2 或 1 1/4");
    if (parts.size() == 2) {
        require(std::all_of(parts[0].begin(), parts[0].end(), [](char c) { return c >= '0' && c <= '9'; }) &&
                    parts[1].find('/') != std::string::npos,
                "带分数须为整数加真分数，例如 1 1/4");
    }
    double value = 0;
    for (const auto &s : parts) {
        auto slash = s.find('/');
        if (slash == std::string::npos) {
            value += toolbox::parse(s);
        } else {
            double a = toolbox::parse(s.substr(0, slash)), b = toolbox::parse(s.substr(slash + 1));
            require(a >= 0 && b > 0, "分数须为非负数且分母大于零");
            if (parts.size() == 2) {
                require(a < b, "带分数的小数部分须小于 1");
            }
            value += a / b;
        }
    }
    require(std::isfinite(value) && value > 0 && value < 1e5, "英寸直径超出有效范围");
    return value;
}
double unDiameter(const std::string &text) {
    auto size = trim(text);
    if (!size.empty() && size[0] == '#') {
        size = "No." + trim(size.substr(1));
    }
    auto normalized = lowerAscii(size);
    for (const auto &row : unifiedData()["series"].array) {
        if (lowerAscii(row["size"].string) == normalized) {
            return n(row, "diameter_in");
        }
    }
    return fraction(size);
}
std::vector<std::pair<std::string, double>> unVariants(const std::string &size) {
    std::vector<std::pair<std::string, double>> result;
    double d;
    try {
        d = unDiameter(size);
    } catch (const std::exception &) {
        return result;
    }
    for (const char *series :
         {"UNC", "UNF", "UNEF", "4UN", "6UN", "8UN", "12UN", "16UN", "20UN", "28UN", "32UN"}) {
        for (const auto &row : unifiedData()["series"].array) {
            if (same(d, n(row, "diameter_in"))) {
                for (const auto &v : row["variants"].array) {
                    if (v["series"].string == series && valid(n(v, "tpi")) && n(v, "tpi") > 0) {
                        auto item = std::make_pair(std::string(series), n(v, "tpi"));
                        if (std::find(result.begin(), result.end(), item) == result.end()) {
                            result.push_back(item);
                        }
                    }
                }
            }
        }
    }
    return result;
}
std::string unSeries(const std::string &size, double tpi) {
    for (const auto &v : unVariants(size)) {
        if (same(v.second, tpi)) {
            return v.first;
        }
    }
    return "";
}
Report unified(const std::string &size, double tpi, const std::string &grade) {
    require(std::isfinite(tpi) && tpi > 0, "TPI 须大于零");
    double dIn = unDiameter(size);
    const std::vector<std::string> classes{"1A", "2A", "3A", "1B", "2B", "3B"};
    auto pos = std::find(classes.begin(), classes.end(), grade);
    require(pos != classes.end(), "请选择 1A、2A、3A 或 1B、2B、3B 工件公差等级");
    const bool external = grade[1] == 'A';
    const int level = grade[0] - '0';
    const std::string series = unSeries(size, tpi);
    double pIn = 1 / tpi;
    double le = (series == "UNC" || series == "UNF" ||
                 ((series == "4UN" || series == "6UN" || series == "8UN") && level != 1))
                    ? dIn
                    : 9 * pIn;
    double d2In = dIn - 3 * std::sqrt(3.) / 8 * pIn;
    double base = .0015 * std::cbrt(dIn) + .0015 * std::sqrt(le) + .015 * std::pow(pIn, 2. / 3.);
    double factor = level == 1 ? 1.5 : level == 2 ? 1 : .75;
    if (!external) {
        factor *= 1.3;
    }
    const double tol = base * factor, allowance = external && level <= 2 ? .3 * base : 0,
                 hiIn = external ? d2In - allowance : d2In + tol, loIn = external ? hiIn - tol : d2In;
    require(std::isfinite(d2In) && std::isfinite(loIn) && std::isfinite(hiIn) && std::isfinite(tol) &&
                std::isfinite(le) && d2In > 0 && loIn > 0 && hiIn > 0 && tol > 0 && le > 0 && loIn < hiIn &&
                hiIn < dIn - allowance,
            "当前直径、牙数和等级不能形成有效的工件中径公差带");
    const double d = dIn * inch, p = inch / tpi, d2 = d2In * inch, d1 = d - 5 * std::sqrt(3.) / 8 * p,
                 root = d - 17 * std::sqrt(3.) / 24 * p;
    positiveGeometry(d, p, d2, root);
    double lo = loIn * inch, hi = hiIn * inch, major = (dIn - allowance) * inch;
    Report out("UN " + trim(size) + "-" + rawNumber(tpi) + " " + (series.empty() ? "非标" : series) + "-" +
               grade);
    out.metric("工件公差等级", grade);
    out.metric("工件中径下限 (mm)", lo);
    out.metric("工件中径上限 (mm)", hi);
    out.metric("工件中径公差 (mm)", tol * inch);
    out.metric("工件容隙 (mm)", allowance * inch);
    out.metric("旋合长度 Le (mm)", le * inch);
    geometry(out, d, p, d2, d1, 60, 1);
    if (external) {
        out.metric("基本外螺纹小径 (mm)", root);
    }
    out.parameter("公称直径 (inch)", dIn);
    out.parameter("TPI (牙/inch)", tpi);
    out.metric("螺纹系列", series.empty() ? "非标" : series);
    out.metric("2A 基础中径公差 (mm)", base * inch);
    if (external) {
        out.metric("工件大径上限 (mm)", major);
    }
    double u = manufacturing(dIn, tpi);
    const auto &db = database()["DATA"]["un"]["am"];
    double ta = optionalMm(band(db["x"], dIn, tpi), "顶径公差"),
           tw = optionalMm(band(db["w"], dIn, tpi), "中径公差");
    out.metric("中径制造公差 U (mm)", u);
    out.metric("顶径公差 Ta · 参考 (mm)", ta);
    out.note("工作量规中径按 U 计算；大径、小径及校对尺寸供参考。");
    // 无独立磨损数据：不显示空磨损列，也不生成 TS/ZS 校损规。
    if (!external) {
        Gauge go("GO", "通端工作塞规", lo + u / 2, u);
        go.major = {d - ta, d};
        go.minorMax = d1;
        Gauge ng("NO GO", "止端工作塞规", hi - u / 2, u);
        double ngMajor = ng.pd[1] + .2 * p + u / 2;
        ng.major = {ngMajor - ta, ngMajor};
        ng.minorMax = d1;
        out.tables.push_back(gaugeTable("工作塞规 · 检内螺纹", {go, ng}, false));
        out.showInches();
        out.checked();
        return out;
    }
    out.metric("校对中径公差 · 参考 (mm)", tw);
    Gauge rg("GO", "通端工作环规", hi - u / 2, u);
    rg.majorMin = major;
    rg.minor = {root, root};
    Gauge rn("NO GO", "止端工作环规", lo + u / 2, u);
    rn.majorMin = major;
    rn.minor = {rn.center - .2 * p, rn.center - .2 * p};
    out.tables.push_back(gaugeTable("工作环规 · 检外螺纹", {rg, rn}, false));
    struct Spec {
        const char *code, *name;
        double upper, major, half;
    };
    const Spec specs[] = {{"TT", "校通-通", rg.pd[0], major, u},
                          {"TZ", "校通-止", rg.pd[1], rg.pd[1] + .2 * p, u / 2},
                          {"ZT", "校止-通", rn.pd[0], major, u},
                          {"ZZ", "校止-止", rn.pd[1], lo, u}};
    std::vector<Gauge> cal;
    for (auto s : specs) {
        Gauge g(s.code, s.name, s.upper - tw / 2, tw);
        g.major = {s.major - s.half, s.major + s.half};
        g.minorMax = root;
        cal.push_back(g);
    }
    if (!valid(tw)) {
        out.note("该规格未收录校对中径公差，相应界限不计算。");
    }
    out.tables.push_back(gaugeTable("校对塞规 · 参考", cal, false));
    out.showInches();
    out.checked();
    return out;
}
} // namespace toolbox::gauges
