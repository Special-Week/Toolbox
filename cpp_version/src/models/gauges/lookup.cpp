// 查表统一采用下界开、上界闭；最近档替代必须附带说明，缺失值不能补零。
#include "toolbox/gauges.hpp"

namespace toolbox::gauges {
double pitch(const Data &row) { return first(row, {"螺距P", "螺距"}); }
namespace {
double lower(const Data &row) {
    return first(row, {"基本大径D＞", "基本大径d＞", "基本大径>", "公称直径d>"});
}
double upper(const Data &row) { return first(row, {"基本大径D≤", "基本大径d≤", "基本大径≤", "公称直径d≤"}); }
double nearest(std::vector<double> values, double target) {
    double best = missing;
    for (double value : values) {
        if (valid(value) && (!valid(best) || std::abs(value - target) < std::abs(best - target))) {
            best = value;
        }
    }
    return best;
}
} // namespace
const Data *closestPitch(const Data &table, double p) {
    const Data *best = nullptr;
    for (const auto &row : table.array) {
        if (!best || std::abs(pitch(row) - p) < std::abs(pitch(*best) - p)) {
            best = &row;
        }
    }
    return best;
}
std::pair<double, double> tolerance(const Data &table, double d, double p, const std::string &grade,
                                    std::vector<std::string> &notes) {
    std::vector<double> pitches;
    double minimum = std::numeric_limits<double>::infinity();
    for (const auto &row : table.array) {
        if (valid(pitch(row))) {
            pitches.push_back(pitch(row));
        }
        minimum = std::fmin(minimum, lower(row));
    }
    std::sort(pitches.begin(), pitches.end());
    double used = nearest(pitches, p);
    require(valid(used), "公差表未收录该螺距");
    if (!same(used, p)) {
        notes.push_back("公差、偏差按 P=" + rawNumber(used) + " mm 档取值；几何尺寸按输入螺距计算。");
    }
    const bool clamped = d <= minimum;
    const Data *hit = nullptr;
    for (const auto &row : table.array) {
        if (same(pitch(row), used) &&
            ((d > lower(row) && d <= upper(row)) || (clamped && same(lower(row), minimum)))) {
            hit = &row;
            break;
        }
    }
    if (hit) {
        if (clamped) {
            notes.push_back("直径 " + rawNumber(d) + " mm 低于公差表下限，采用最小直径档。");
        }
    } else {
        notes.push_back("直径 " + rawNumber(d) + " mm 与 P=" + rawNumber(used) +
                        " mm 的标准档不匹配，采用该螺距首次出现的直径档，仅作参考。");
        for (const auto &row : table.array) {
            if (same(pitch(row), used)) {
                hit = &row;
                break;
            }
        }
    }
    require(hit != nullptr, "没有对应公差档");
    double value = n(*hit, "公差等级" + grade);
    require(valid(value),
            ("P=" + rawNumber(used) + " mm 档没有 " + grade + " 级中径公差定义，请更换公差带。").c_str());
    return {value, used};
}
double deviation(const Data &table, double p, const std::string &key) {
    auto row = closestPitch(table, p);
    require(row != nullptr, "未收录基本偏差");
    double value = n(*row, key);
    require(valid(value), ("该螺距档没有 " + key + " 的定义").c_str());
    return value;
}
std::optional<double> minorTolerance(const Data &table, double p, const std::string &grade) {
    auto row = closestPitch(table, p);
    if (!row) {
        return {};
    }
    for (const auto &key : {"公差等级" + grade, grade + "级公差"}) {
        if ((*row)[key].tag != Data::Tag::Null) {
            double v = n(*row, key);
            return valid(v) ? std::optional<double>(v) : std::nullopt;
        }
    }
    return {};
}
std::optional<double> trapezoidalMinorTolerance(double d, double p, const std::string &grade,
                                                const std::string &letter, std::vector<std::string> &notes) {
    const auto &table = database()["DATA"]["trap"]["tdo"];
    std::vector<double> pitches;
    for (const auto &r : table.array) {
        pitches.push_back(pitch(r));
    }
    std::sort(pitches.begin(), pitches.end());
    double used = nearest(pitches, p);
    const Data *hit = nullptr, *best = nullptr;
    double distance = std::numeric_limits<double>::infinity();
    for (const auto &r : table.array) {
        if (same(pitch(r), used)) {
            if (d > lower(r) && d <= upper(r) && !hit) {
                hit = &r;
            }
            double delta = std::max(0., std::max(lower(r) - d, d - upper(r)));
            if (delta < distance) {
                distance = delta;
                best = &r;
            }
        }
    }
    if (!hit) {
        notes.push_back("外螺纹小径公差采用该螺距最近的直径档，仅作参考。");
        hit = best;
    }
    if (!hit) {
        return {};
    }
    double v = n(*hit, letter + grade);
    return valid(v) ? std::optional<double>(v) : std::nullopt;
}
Factors factors(bool trap, double tol, bool negativeEs) {
    const auto &table = database()["DATA"][trap ? "trap" : "metric"]["zf"];
    const Data *hit = nullptr;
    for (const auto &row : table.array) {
        if (tol > n(row, trap ? "Td2/TD2>" : "Td2或者TD2＞") &&
            tol <= n(row, trap ? "Td2/TD2≤" : "Td2或者TD2≤")) {
            hit = &row;
            break;
        }
    }
    require(hit != nullptr, ("工件中径公差 " + fixed(tol) + " μm 超出位置要素表范围").c_str());
    auto value = [&](const std::string &key) { return n(*hit, key) / 1000.; };
    std::string ring = trap ? "环规" : "螺纹环规", plug = trap ? "塞规" : "螺纹塞规";
    return {value("TR"),
            value("TPL"),
            value("TCP"),
            value("m"),
            value(trap ? (negativeEs ? "ZR(es<0)" : "ZR(es=0)") : "ZR"),
            value("ZPL"),
            value(ring + "WGO"),
            value(ring + "WNG"),
            value(plug + "WGO"),
            value(plug + "WNG")};
}
std::pair<double, double> lengths(bool trap, bool ring, double d, double p, std::vector<std::string> &notes) {
    const auto &table = database()["DATA"][trap ? "trap" : "metric"][ring ? "rl" : "sl"];
    std::vector<const Data *> candidates;
    if (trap) {
        std::vector<double> diameters;
        for (const auto &r : table.array) {
            diameters.push_back(n(r, "公称直径"));
        }
        double chosen = nearest(diameters, d);
        if (!same(chosen, d)) {
            notes.push_back("量规长度采用最近标准直径 Tr" + rawNumber(chosen) + " 档。");
        }
        for (const auto &r : table.array) {
            if (same(n(r, "公称直径"), chosen)) {
                candidates.push_back(&r);
            }
        }
    } else {
        double minimum = std::numeric_limits<double>::infinity();
        for (const auto &r : table.array) {
            minimum = std::fmin(minimum, lower(r));
        }
        for (const auto &r : table.array) {
            if ((d > lower(r) && d <= upper(r)) || (d <= minimum && same(lower(r), minimum))) {
                candidates.push_back(&r);
            }
        }
        if (candidates.empty()) {
            notes.push_back("量规长度：直径超出对应档，采用该螺距首次出现的长度档参考。");
            for (const auto &r : table.array) {
                if (same(pitch(r), p)) {
                    candidates.push_back(&r);
                    break;
                }
            }
        } else if (d <= minimum) {
            notes.push_back("量规长度：直径低于表下限，采用最小直径档参考。");
        }
    }
    std::vector<double> pitches;
    for (auto r : candidates) {
        pitches.push_back(pitch(*r));
    }
    double used = nearest(pitches, p);
    if (valid(used) && !same(used, p)) {
        notes.push_back("量规长度采用最近标准螺距 P=" + rawNumber(used) + " mm 档。");
    }
    if (!valid(used)) {
        notes.push_back("未收录该规格的量规长度，长度显示“—”。");
    }
    for (auto r : candidates) {
        if (same(pitch(*r), used)) {
            return {n(*r, "通端"), n(*r, "止端")};
        }
    }
    return {missing, missing};
}
MetricLengths metricLengths(double d, double p, std::vector<std::string> &notes) {
    if (d > 100) {
        notes.push_back("D＞100 mm：锥度锁紧式长度和手柄表不适用，需另定结构。");
        return {missing, missing, missing, "—"};
    }
    const auto &db = database();
    auto pick = [&](const char *key) -> const Data * {
        for (const auto &r : db[key].array) {
            if (std::max(d, 1.) >= number(r[0]) && std::max(d, 1.) <= number(r[1])) {
                return &r[2];
            }
        }
        return nullptr;
    };
    const Data *t5 = same(d, 62) ? &db["T5_62"] : pick("T5"), *t6 = same(d, 62) ? &db["T6_62"] : pick("T6");
    if (!t5 || t5->tag != Data::Tag::Object) {
        return {missing, missing, missing, "—"};
    }
    std::vector<double> pitches;
    for (const auto &entry : t5->object) {
        try {
            pitches.push_back(toolbox::parse(entry.first));
        } catch (const std::exception &) {
        }
    }
    // 与 Rust 一致：整数螺距优先，其次非整数升序，等距离保留先出现项。
    std::sort(pitches.begin(), pitches.end(), [](double a, double b) {
        bool ai = std::floor(a) == a, bi = std::floor(b) == b;
        return ai == bi ? a < b : ai;
    });
    double used = nearest(pitches, p);
    if (!valid(used)) {
        used = p;
    }
    std::string key;
    for (const auto &entry : t5->object) {
        try {
            if (same(toolbox::parse(entry.first), used)) {
                key = entry.first;
                break;
            }
        } catch (const std::exception &) {
        }
    }
    if (!same(p, used)) {
        notes.push_back("塞规长度采用该直径档最近螺距 P=" + rawNumber(used) + " mm。");
    }
    if (d < 1) {
        notes.push_back("D＜1 mm：塞规长度按 1～3 mm 直径档参考。");
    }
    std::string handle = "—";
    for (const auto &r : db["HANDLE"].array) {
        if (std::max(d, 1.) >= number(r[0]) && std::max(d, 1.) <= number(r[1])) {
            if (r[2].tag == Data::Tag::String) {
                handle = r[2].string;
            }
            break;
        }
    }
    static const Data empty;
    const auto &pair = t6 ? (*t6)[key] : empty;
    return {number(pair[0]), number(pair[1]), number((*t5)[key]), handle + "#"};
}
} // namespace toolbox::gauges
