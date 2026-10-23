// 显示层统一五位小数，英寸附值从未修约数值独立换算。
#include "toolbox/gauges.hpp"
#include <iomanip>
#include <locale>
#include <sstream>

namespace toolbox::gauges {
std::string fixed(double value) {
    if (!valid(value)) {
        return "—";
    }
    std::ostringstream out;
    out.imbue(std::locale::classic());
    out << std::fixed << std::setprecision(5) << (std::abs(value) < 0.000005 ? 0. : value);
    return out.str();
}
Cell::Cell(double value) : display(fixed(value)), raw(valid(value) ? rawNumber(value) : "—") {}
Cell Cell::parameter(double value) {
    Cell cell(value);
    cell.display = valid(value) ? utf8(format(value)) : "—";
    return cell;
}
Cell Cell::limit(double value, const std::string &sign) {
    auto cell = Cell(value);
    if (valid(value)) {
        cell.display = sign + " " + cell.display;
        cell.raw = sign + " " + cell.raw;
    }
    return cell;
}
Cell Cell::pair(double a, double b, const std::string &separator) {
    auto cell = Cell("");
    cell.display = fixed(a) + separator + fixed(b);
    cell.raw = Cell(a).raw + separator + Cell(b).raw;
    return cell;
}
Cell Cell::value(const Data &v) {
    auto x = number(v);
    if (std::isfinite(x)) {
        return Cell(x);
    }
    return Cell(v.tag == Data::Tag::String ? v.string : "—");
}
void Report::metric(const std::string &name, Cell value) { metrics.emplace_back(name, std::move(value)); }
void Report::parameter(const std::string &name, double value) { metric(name, Cell::parameter(value)); }
void Report::note(const std::string &text) { notes.push_back(text); }
void Report::checked() const {
    for (const auto &table : tables) {
        for (const auto &row : table.rows) {
            for (std::size_t i = 0; i < row.size(); ++i) {
                const auto &heading = table.headings[i];
                if (heading.size() < 4 || heading.substr(heading.size() - 4) != "(mm)") {
                    continue;
                }
                std::istringstream tokens(row[i].raw);
                std::string token;
                while (tokens >> token) {
                    double v;
                    try {
                        v = toolbox::parse(token);
                    } catch (const std::exception &) {
                        continue;
                    }
                    require(std::isfinite(v) && v > 0, ("当前参数及公差产生非正或无效尺寸（" + table.title +
                                                        "：" + heading + "），请核对直径、螺距和公差带。")
                                                           .c_str());
                }
            }
        }
    }
}
void Report::showInches() {
    note("单位换算按 1 inch = 25.4 mm 使用未修约值计算；mm 与 inch "
         "分别保留五位小数，直接用显示值换算可能出现末位差异。");
    auto annotate = [](Cell &cell) {
        std::istringstream tokens(cell.raw);
        std::string token, converted;
        bool found = false;
        while (tokens >> token) {
            try {
                token = fixed(toolbox::parse(token) / inch);
                found = true;
            } catch (const std::exception &) {
            }
            if (!converted.empty()) {
                converted += ' ';
            }
            converted += token;
        }
        if (found) {
            cell.display += "\n(" + converted + " inch)";
        }
    };
    for (auto &metric : metrics) {
        if (metric.first.find("(mm)") != std::string::npos) {
            annotate(metric.second);
        }
    }
    for (auto &table : tables) {
        for (auto &row : table.rows) {
            for (std::size_t i = 0; i < row.size(); ++i) {
                if (table.headings[i].find("(mm)") != std::string::npos) {
                    annotate(row[i]);
                }
            }
        }
    }
}
std::string Report::text(Type type) const {
    auto single = [](std::string s) {
        std::replace(s.begin(), s.end(), '\n', ' ');
        return s;
    };
    std::string out = title + "\n";
    for (const auto &m : metrics) {
        out += m.first + "：" + single(m.second.display) + "\n";
    }
    for (const auto &t : tables) {
        out += "\n" + t.title + "\n";
        for (std::size_t i = 0; i < t.headings.size(); ++i) {
            out += (i ? "\t" : "") + t.headings[i];
        }
        out += '\n';
        for (const auto &row : t.rows) {
            for (std::size_t i = 0; i < row.size(); ++i) {
                out += (i ? "\t" : "") + single(row[i].display);
            }
            out += '\n';
        }
    }
    for (const auto &note : notes) {
        out += "取值说明：" + note + "\n";
    }
    return out + "\n计算说明\n" + explanation(type) + "\n\n详细公式与取值\n" + convention(type) + "\n";
}
void geometry(Report &out, double d, double p, double d2, double d1, double angle, int lines) {
    out.parameter("基本大径 D (mm)", d);
    out.metric("基本中径 D₂ (mm)", d2);
    out.metric("基本小径 D₁ (mm)", d1);
    out.parameter("螺距 P (mm)", p);
    out.parameter("牙型角 (°)", angle);
    out.parameter("半角 (°)", angle / 2);
    out.parameter("线数 / 头数", lines);
    out.metric("导程 L (mm)", lines * p);
    const double pi = std::acos(-1.);
    out.metric("螺旋升角 ψ (°)", std::atan(lines * p / (pi * d2)) * 180 / pi);
    out.metric("最佳三针直径 d₀ (mm)", p / (2 * std::cos(angle / 2 * pi / 180)));
}
void positiveGeometry(double d, double p, double d2, double d1) {
    require(std::isfinite(d) && std::isfinite(p) && std::isfinite(d2) && std::isfinite(d1) && d > 0 &&
                p > 0 && d2 > 0 && d1 > 0 && d <= 1e6 && p <= 1e5,
            "直径和螺距须为有效正数，并形成大径＞中径＞小径＞0 的几何关系");
    require(d > d2 && d2 > d1, "当前直径与螺距不能形成有效牙型");
}
} // namespace toolbox::gauges
