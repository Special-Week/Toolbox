// 光滑量规：公差等级和基本偏差分别查表，缺少等级修正时明确拒绝推算。
#include "toolbox/gauges.hpp"

namespace toolbox::gauges {
std::vector<std::string> letters(bool hole) {
    std::vector<std::string> out;
    for (const auto &v : database()[hole ? "SM_H_LETTERS" : "SM_S_LETTERS"].array) {
        if (v.tag == Data::Tag::String) {
            out.push_back(v.string);
        }
    }
    return out;
}
Report smooth(double d, int mode, const std::string &letter, int grade) {
    require(std::isfinite(d) && d > 0 && d <= 500, "基本尺寸范围为 0＜D≤500 mm");
    require(grade >= 6 && grade <= 16 && mode >= 0 && mode <= 2, "请选择 IT6～IT16 和有效量规类型");
    bool hole = mode == 0;
    auto available = letters(hole);
    require(std::find(available.begin(), available.end(), letter) != available.end(),
            "孔/轴公差带字母与量规类型不匹配");
    const auto &db = database()["DATA"]["smooth"];
    auto find = [&](const Data &table) -> const Data * {
        for (const auto &r : table.array) {
            if (d > n(r, "基本尺寸＞") && d <= n(r, "基本尺寸≤")) {
                return &r;
            }
        }
        return nullptr;
    };
    auto row = find(db["it"]);
    require(row != nullptr, "基本尺寸没有公差档");
    auto prefix = "IT" + std::to_string(grade);
    double itu = n(*row, prefix + "公差"), t1u = n(*row, prefix + "T1"), z1u = n(*row, prefix + "Z1");
    require(valid(itu) && valid(t1u) && valid(z1u), "该尺寸档的工件公差或量规位置要素未定义");
    double it = itu / 1000, t1 = t1u / 1000, z1 = z1u / 1000;
    auto eb = find(db[hole ? "ebH" : "ebS"]);
    require(eb != nullptr, "没有对应基本偏差档");
    const bool symmetric = letter == "JS" || letter == "js";
    const std::vector<std::string> corrected{"P", "R", "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC"};
    require(
        !(hole && grade <= 7 && std::find(corrected.begin(), corrected.end(), letter) != corrected.end()),
        ("需等级相关的 Δ 修正，内置表未完整收录，不能直接计算 " + letter + std::to_string(grade)).c_str());
    require(!(hole && d <= 1 && (letter == "A" || letter == "B")),
            "基本尺寸不大于 1 mm 时不采用 A、B 孔公差带");
    double v = n(*eb, letter);
    require(symmetric || valid(v),
            "当前公差带没有可直接使用的基本偏差；等级/Δ 修正规则未完整收录，不能推算。");
    double ei, es;
    if (symmetric) {
        ei = -it / 2;
        es = it / 2;
    } else if (hole) {
        if (std::string("ABCDEFGH").find(letter) != std::string::npos) {
            ei = v / 1000;
            es = ei + it;
        } else {
            es = v / 1000;
            ei = es - it;
        }
    } else if (std::string("abcdefgh").find(letter) != std::string::npos) {
        es = v / 1000;
        ei = es - it;
    } else {
        ei = v / 1000;
        es = ei + it;
    }
    std::array<double, 2> go = hole ? std::array<double, 2>{d + ei + z1 - t1 / 2, d + ei + z1 + t1 / 2}
                                    : std::array<double, 2>{d + es - z1 - t1 / 2, d + es - z1 + t1 / 2};
    std::array<double, 2> ng =
        hole ? std::array<double, 2>{d + es - t1, d + es} : std::array<double, 2>{d + ei, d + ei + t1};
    double wear = hole ? d + ei : d + es;
    require(std::isfinite(go[0]) && std::isfinite(go[1]) && std::isfinite(ng[0]) && std::isfinite(ng[1]) &&
                go[0] > 0 && go[1] > 0 && ng[0] > 0 && ng[1] > 0,
            "该尺寸与公差组合产生非正量规尺寸，请核对参数");
    std::string label = hole ? "孔用塞规" : mode == 2 ? "轴用卡规" : "轴用环规";
    Report out("光滑量规 · " + label + " · " + rawNumber(d) + " " + letter + std::to_string(grade));
    out.parameter("基本尺寸 D (mm)", d);
    out.metric("工件公差带", letter + std::to_string(grade));
    out.metric("工件公差 IT (μm)", itu);
    out.metric(hole ? "下偏差 EI (mm)" : "下偏差 ei (mm)", ei);
    out.metric(hole ? "上偏差 ES (mm)" : "上偏差 es (mm)", es);
    out.metric("制造公差 T1 (μm)", t1u);
    out.metric("位置要素 Z1 (μm)", z1u);
    out.tables.push_back({label,
                          {"量规", "尺寸下限 (mm)", "尺寸上限 (mm)", "磨损极限 (mm)"},
                          {{"GO · 通端", go[0], go[1], wear}, {"NO GO · 止端", ng[0], ng[1], "—"}}});
    out.metric("工件尺寸下限 (mm)", d + ei);
    out.metric("工件尺寸上限 (mm)", d + es);
    if (mode == 2) {
        out.note("卡规与环规使用同组制造公差，卡规为两平行面、非全形检验，应结合工件形状要求。");
    }
    out.note("基本偏差按内置表直接取值；特殊字母的完整等级与 Δ 修正规则未收录。结果须结合实际执行标准核验。");
    out.checked();
    return out;
}
} // namespace toolbox::gauges
