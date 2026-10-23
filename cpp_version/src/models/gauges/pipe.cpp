// R/NPT 保留各自查表口径，结构尺寸为 mm，制造/磨损偏差为 μm，角偏差为角分。
#include "toolbox/gauges.hpp"

namespace toolbox::gauges {
std::vector<std::string> pipeSizes(bool npt) {
    std::vector<std::string> out;
    const auto &table = npt ? database()["NPT_PGS"] : database()["DATA"]["pipe"]["dim"];
    for (const auto &r : table.array) {
        if (r[npt ? "规格" : "尺寸代号"].tag == Data::Tag::String) {
            out.push_back(r[npt ? "规格" : "尺寸代号"].string);
        }
    }
    return out;
}
namespace {
const Data &find(const Data &table, const char *key, const std::string &code) {
    for (const auto &r : table.array) {
        if (r[key].string == code && r[key].tag == Data::Tag::String) {
            return r;
        }
    }
    throw std::invalid_argument("没有收录尺寸代号 " + code + "，请从标准系列选择");
}
using Property = std::array<const char *, 3>;
Table properties(const std::string &title, const Data &row, std::initializer_list<Property> fields) {
    Table table{title, {"参数", "数值", "单位"}, {}};
    for (const auto &field : fields) {
        table.rows.push_back({field[1], Cell::value(row[field[0]]), field[2]});
    }
    return table;
}
} // namespace
Report pipe(bool npt, const std::string &code) {
    // 交互入口仅传标准代号，避免把不同管螺纹类型的同名规格混在一起。
    if (npt) {
        const auto &pg = find(database()["NPT_PGS"], "规格", code);
        auto rg = find(database()["NPT_RNG"], "规格", code);
        if (code == "6") {
            rg.object["D2"] = Data{}; // 原表 6 inch 环规中径存疑，不补造替代数值。
        }
        Report out("NPT " + code);
        geometry(out, n(pg, "d"), n(pg, "P"), n(pg, "d2"), n(pg, "d1"), 60, 1);
        out.parameter("TPI (牙/inch)", n(pg, "tpi"));
        out.metric("锥度", "1:16");
        out.tables.push_back(properties("NPT 工作塞规 · 基面尺寸", pg,
                                        {{"L1", "基面距离 L1", "mm"},
                                         {"L", "螺纹长 L", "mm"},
                                         {"L2", "螺纹上限 L2", "mm"},
                                         {"L3", "螺纹下限 L3", "mm"},
                                         {"d", "基面大径 d", "mm"},
                                         {"d2", "基面中径 d₂", "mm"},
                                         {"d1", "基面小径 ≤d₁", "mm"},
                                         {"L0", "总长 L0", "mm"},
                                         {"xh", "型号", ""}}));
        out.tables.push_back(properties("NPT 工作环规 · 基面尺寸", rg,
                                        {{"D", "基面大径 D", "mm"},
                                         {"D2", "基面中径 D₂", "mm"},
                                         {"D1", "基面小径 ≤D₁", "mm"},
                                         {"OD", "环规外径", "mm"},
                                         {"TH", "环规厚", "mm"},
                                         {"xh", "型号", ""}}));
        out.note("按内置规格表查询，基面为管端起 L1 处；螺距和尺寸保留表中修约口径，不从 TPI "
                 "重新推算已列尺寸。查表覆盖 1/16～6，包含 3-1/2。");
        if (code == "6") {
            out.note("NPT 6 环规中径原表值存在疑点，暂不输出；请按实际执行标准或量规图纸确认该中径。");
        }
        out.checked();
        return out;
    }
    const auto &db = database()["DATA"]["pipe"];
    const auto &dim = find(db["dim"], "尺寸代号", code), &tol = find(db["tol"], "尺寸代号", code),
               &wear = find(db["wear"], "尺寸代号", code);
    Report out("R " + code);
    geometry(out, n(dim, "D"), n(dim, "P"), n(dim, "D2"), n(dim, "D1"), 55, 1);
    out.parameter("TPI (牙/inch)", n(dim, "n"));
    out.metric("锥度", "1:16");
    out.tables.push_back(properties("螺纹量规结构尺寸", dim,
                                    {{"L0", "基准距离 L0", "mm"},
                                     {"L1", "管端至基面 L1", "mm"},
                                     {"L2", "装板距离 L2", "mm"},
                                     {"b3", "塞规台阶宽 b3", "mm"},
                                     {"b3偏差", "b3 对称偏差", "mm"},
                                     {"c", "沉孔 c", "mm"},
                                     {"L3尺寸", "L3", "mm"},
                                     {"L4", "L4", "mm"},
                                     {"T1尺寸", "台阶检验 T1", "mm"},
                                     {"g", "基面长 g", "mm"},
                                     {"L5", "L5", "mm"}}));
    out.tables.push_back(properties("螺距、半角与锥度偏差", find(db["dev"], "尺寸代号", code),
                                    {{"螺距公差", "螺距公差", "mm"},
                                     {"牙型半角偏差", "牙型半角偏差 ±", "角分"},
                                     {"塞规锥度偏差", "塞规锥度上偏差（下偏差 0）", "mm"},
                                     {"环规锥度偏差", "环规锥度下偏差（上偏差 0）", "mm"},
                                     {"塞规光滑锥度偏差", "光滑塞规锥度偏差", "mm"},
                                     {"环规光滑锥度偏差", "光滑环规锥度偏差", "mm"}}));
    out.tables.push_back(properties(
        "光滑圆锥塞规 · 检内螺纹小径", find(db["sp"], "尺寸代号", code),
        {{"D1", "D₁", "mm"}, {"L", "L", "mm"}, {"k", "k", "mm"}, {"j", "j", "mm"}, {"c", "c", "mm"}}));
    out.tables.push_back(properties("光滑圆锥环规 · 检外螺纹大径", find(db["sr"], "尺寸代号", code),
                                    {{"d", "d", "mm"},
                                     {"D4", "D₄", "mm"},
                                     {"L6", "L6", "mm"},
                                     {"L7", "L7", "mm"},
                                     {"L8", "L8", "mm"},
                                     {"L9", "L9", "mm"}}));
    out.note("R 中径以基准平面为准，锥度按 "
             "1:16；台阶判定及配作要求按实际标准执行，不能把锥螺纹当作直螺纹通止规。");
    out.tables.push_back(properties(
        "制造公差与位置要素", tol,
        {{"TPL", "工作塞规 TPL", "μm"}, {"TR", "工作环规 TR", "μm"}, {"TCP", "校对塞规 TCP", "μm"}}));
    out.tables.push_back(
        properties("中径磨损偏差（保留表中符号）", wear, {{"塞规", "塞规", "μm"}, {"环规", "环规", "μm"}}));
    out.checked();
    return out;
}
} // namespace toolbox::gauges
