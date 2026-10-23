// 字段与纯数值模型分离：类型切换分别保存公差带，UN 规格/牙数/系列双向联动。
#include "toolbox/gauges.hpp"

namespace toolbox::gauges {
namespace {
std::vector<Option> options(const std::vector<std::string> &values) {
    std::vector<Option> out;
    for (const auto &s : values) {
        out.push_back({s, s});
    }
    return out;
}
std::vector<Option> numericOptions(std::vector<double> values, bool descending = false) {
    values.erase(std::remove_if(values.begin(), values.end(), [](double v) { return !valid(v) || v <= 0; }),
                 values.end());
    std::sort(values.begin(), values.end(), [&](double a, double b) { return descending ? a > b : a < b; });
    values.erase(std::unique(values.begin(), values.end(), same), values.end());
    std::vector<Option> out;
    for (double v : values) {
        out.push_back({rawNumber(v), rawNumber(v)});
    }
    return out;
}
class Form {
  public:
    PageData data;
    explicit Form(State state) { data.state = std::move(state); }
    std::string edit(const std::string &key, const std::string &label, const std::string &initial,
                     std::vector<Option> suggestions = {}) {
        auto value = data.state.emplace(key, initial).first->second;
        data.fields.push_back({key, label, false, std::move(suggestions)});
        return value;
    }
    std::string choice(const std::string &key, const std::string &label,
                       const std::vector<std::string> &values, const std::string &initial) {
        auto saved = data.state.find(key);
        std::string value = saved == data.state.end() ? initial : saved->second;
        if (std::find(values.begin(), values.end(), value) == values.end()) {
            value = std::find(values.begin(), values.end(), initial) != values.end() ? initial
                    : values.empty()                                                 ? ""
                                                                                     : values.front();
        }
        if (!values.empty()) {
            data.state[key] = value;
        }
        data.fields.push_back({key, label, true, options(values)});
        return value;
    }
    double read(const std::string &key) const {
        auto it = data.state.find(key);
        return toolbox::parse(it == data.state.end() ? "" : it->second);
    }
    double preview(const std::string &key) const {
        try {
            return read(key);
        } catch (const std::exception &) {
            return missing;
        }
    }
    bool side() {
        return choice("side", "量规类型", {"塞规（内螺纹）", "环规（外螺纹）"}, "塞规（内螺纹）") ==
               "环规（外螺纹）";
    }
    Report build(Type type, const std::string &changed) {
        if (type == Type::Metric || type == Type::Trapezoidal) {
            bool trap = type == Type::Trapezoidal, ring = side();
            const auto &db = database()["DATA"][trap ? "trap" : "metric"];
            std::vector<double> diameters;
            for (const auto &r : db[trap ? "series" : "combo"].array) {
                diameters.push_back(n(r, "公称直径"));
            }
            edit("diameter", "公称直径 D (mm)", trap ? "24" : "20", numericOptions(diameters));
            double d = preview("diameter");
            std::vector<double> pitches;
            if (trap) {
                for (const auto &r : db["series"].array) {
                    for (const char *key : {"螺距P1", "螺距P2", "螺距P3"}) {
                        pitches.push_back(n(r, key));
                    }
                }
            } else {
                bool found = false;
                for (const auto &r : db["combo"].array) {
                    if (same(n(r, "公称直径"), std::isfinite(d) ? d : 20)) {
                        found = true;
                        for (const auto &e : r.object) {
                            if (e.first == "粗牙" || e.first.find("细牙") == 0) {
                                pitches.push_back(number(e.second));
                            }
                        }
                    }
                }
                if (!found) {
                    for (const auto &r : db["combo"].array) {
                        pitches.push_back(n(r, "粗牙"));
                    }
                }
            }
            edit("pitch", "螺距 P (mm)", trap ? "5" : "2.5", numericOptions(pitches, true));
            double p = preview("pitch");
            std::vector<std::string> available;
            if (std::isfinite(d) && std::isfinite(p)) {
                for (const auto &g : grades(trap, d, p)) {
                    if ((g.back() >= 'a' && g.back() <= 'z') == ring) {
                        available.push_back(g);
                    }
                }
            }
            std::string grade =
                choice(ring ? "grade.ring" : "grade.plug", ring ? "外螺纹公差带" : "内螺纹公差带", available,
                       trap ? (ring ? "8e" : "8H") : (ring ? "6g" : "6H"));
            int lines = trap ? std::stoi(choice("lines", "线数 / 头数", {"1", "2", "3", "4", "5"}, "1")) : 1;
            return thread(trap, read("diameter"), read("pitch"), grade, lines);
        }
        if (type == Type::Unified) {
            bool ring = side();
            std::vector<Option> sizes;
            for (const auto &r : unifiedData()["series"].array) {
                const auto &s = r["size"].string;
                double d = n(r, "diameter_in");
                sizes.push_back(
                    {s, s + "（" + utf8(format(d)) + " inch / " + utf8(format(d * inch)) + " mm）"});
            }
            auto size = edit("size", "公称直径 (inch，可填 1/2、1 1/4)", "1/4", sizes);
            auto variants = unVariants(size);
            if (changed == "size" && !variants.empty()) {
                auto chosen = variants.begin();
                for (auto it = variants.begin(); it != variants.end(); ++it) {
                    if (it->first == data.state["_lastSeries"]) {
                        chosen = it;
                        break;
                    }
                }
                data.state["tpi"] = rawNumber(chosen->second);
            }
            if (changed == "series") {
                for (auto v : variants) {
                    if (v.first == data.state["series"]) {
                        data.state["tpi"] = rawNumber(v.second);
                        break;
                    }
                }
            }
            std::vector<double> tpis;
            for (auto v : variants) {
                tpis.push_back(v.second);
            }
            edit("tpi", "每英寸牙数 TPI", "20", numericOptions(tpis, true));
            double tpi = preview("tpi");
            auto detected = unSeries(size, tpi);
            if (detected.empty()) {
                detected = "非标 / 自定义";
            }
            std::vector<std::string> available;
            if (detected == "非标 / 自定义") {
                available.push_back(detected);
            }
            for (auto v : variants) {
                available.push_back(v.first);
            }
            data.state["series"] = detected;
            data.state["_lastSeries"] = detected;
            choice("series", "螺纹系列", available, detected);
            auto grade = choice(ring ? "class.ring" : "class.plug", "工件公差等级",
                                ring ? std::vector<std::string>{"1A", "2A", "3A"}
                                     : std::vector<std::string>{"1B", "2B", "3B"},
                                ring ? "2A" : "2B");
            return unified(size, read("tpi"), grade);
        }
        if (type == Type::Pipe) {
            bool npt = choice("system", "管螺纹类型", {"R · 55° 密封锥螺纹", "NPT · 60° 密封锥螺纹"},
                              "R · 55° 密封锥螺纹")
                           .find("NPT") == 0;
            auto code = choice(npt ? "spec.npt" : "spec.r", "尺寸代号", pipeSizes(npt), "1/2");
            return pipe(npt, code);
        }
        if (type == Type::Rebar) {
            auto angle = choice("angle", "牙型角 (°)", {"75", "60", "70"}, "75");
            std::vector<std::string> specs;
            for (const auto &r : database()["REBAR"][angle].array) {
                specs.push_back(r[0].string + " · " + r[1].string);
            }
            auto spec =
                choice("spec." + angle, "钢筋规格 / 螺纹代号", specs, specs.empty() ? "" : specs.front());
            int lines = std::stoi(choice("lines", "线数 / 头数", {"1", "2", "3", "4"}, "1"));
            return rebar(std::stoi(angle),
                         std::size_t(std::find(specs.begin(), specs.end(), spec) - specs.begin()), lines);
        }
        auto label = choice("mode", "量规类型", {"孔用塞规", "轴用环规", "轴用卡规"}, "孔用塞规");
        int mode = label == "孔用塞规" ? 0 : label == "轴用卡规" ? 2 : 1;
        edit("diameter", "基本尺寸 D (mm)", "20");
        auto letter = choice(mode == 0 ? "letter.hole" : "letter.shaft", "公差带字母", letters(mode == 0),
                             mode == 0 ? "H" : "h");
        std::vector<std::string> its;
        for (int i = 6; i <= 16; ++i) {
            its.push_back("IT" + std::to_string(i));
        }
        auto grade = choice("it", "公差等级", its, "IT7");
        return smooth(read("diameter"), mode, letter, std::stoi(grade.substr(2)));
    }
};
} // namespace
PageData page(Type type, State state, const std::string &changed) {
    Form form(std::move(state));
    try {
        form.data.report = form.build(type, changed);
    } catch (const std::exception &e) {
        form.data.error = e.what();
    }
    return std::move(form.data);
}
} // namespace toolbox::gauges
