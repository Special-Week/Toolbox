// 从 Rust 的量规说明同步生成；更新请运行 scripts/sync-gauges.py。
#include "toolbox/gauges.hpp"
namespace toolbox::gauges {
std::string explanation(Type type) {
    switch (type) {
    case Type::Metric:
        return R"GAUGE(按 GB/T 197、GB/T 3934 查询工件公差和量规要素。小径参考界限不含独立光滑量规公差。
塞规长度按 GB/T 10920-2008，D>100 mm 时长度及手柄另定。)GAUGE";
    case Type::Trapezoidal:
        return R"GAUGE(多线中径公差按线数修正；内螺纹小径取 4 级，单线外螺纹小径按 c/e 带查表。
多线及未收录公差带的小径界限不计算。)GAUGE";
    case Type::Unified:
        return R"GAUGE(工件等级决定工件极限；工作量规中径按制造公差 U 计算。
大径、小径及校对尺寸供参考；未收录磨损储备。)GAUGE";
    case Type::Pipe:
        return R"GAUGE(R 为 55°、NPT 为 60° 密封锥管螺纹，锥度均为 1:16。
尺寸用 mm，制造公差、磨损及半角偏差按表头单位读取。)GAUGE";
    case Type::Rebar:
        return R"GAUGE(基本尺寸按所选 60°、70°、75° 牙型查表，规格为 Φ16～Φ40。
套筒、丝头分别借用 6H、6f 及 GB/T 3934 量规要素；不构成产品标准的完整符合性判定。)GAUGE";
    case Type::Smooth:
        return R"GAUGE(适用 0<D≤500 mm、IT6～IT16；卡规与轴用环规采用同一数值。
P～ZC 的 6、7 级孔公差带涉及未收录 Δ 修正，不计算该组合。)GAUGE";
    }
    return "";
}
std::string convention(Type type) {
    switch (type) {
    case Type::Metric:
        return R"GAUGE(公制普通螺纹 M · GB/T 3934 / GB/T 197
D₂=D−3√3·P/8；D₁=D−5√3·P/8；截短量 2F₁=0.2P。
塞规磨损极限=中径下限−W，环规磨损极限=中径名义值+W。
内螺纹小径参考界限为 D₁+EI 至 D₁+EI+T_D1；外螺纹 td 仅用于大径。
最佳针径 d₀=P/[2cos(30°)]。)GAUGE";
    case Type::Trapezoidal:
        return R"GAUGE(梯形螺纹 Tr · GB/T 8124 / GB/T 5796
d₂=d−0.5P；D₁=d−P；d₃=d−P−2ac；D₄=d+2ac。
ac 按螺距取 0.15、0.25、0.5、1 mm；止端及校对规尺寸偏移采用 2ac。
2～5 线中径公差系数依次为 1.12、1.25、1.4、1.6，修正后查询位置要素。
单线外螺纹小径公差向基本小径以下布置，不叠加中径偏差。
塞规磨损按中径下限−W，环规按中径名义值+W；长度查梯形专用表。
导程 L=nP，升角 ψ=atan[L/(πd₂)]；最佳针径 d₀=P/[2cos(15°)]。)GAUGE";
    case Type::Unified:
        return R"GAUGE(美制统一螺纹 UN · 公式与取值
以下长度均用 inch，P=1/TPI，d₂=D−3√3·P/8。
2A 基础公差 T=0.0015D^(1/3)+0.0015√Le+0.015P^(2/3)。
等级公差：1A=1.5T，2A=T，3A=0.75T，1B=1.95T，2B=1.3T，3B=0.975T。
1A、2A 容隙 a=0.3T，其余为 0。外螺纹上限=d₂−a、下限=上限−等级公差；内螺纹下限=d₂、上限=d₂+等级公差。
Le：UNC/UNF 取 D；4UN/6UN/8UN 的 1 级取 9P、2/3 级取 D；其他系列及非标组合取 9P。
设工件中径下限 m、上限 M：通端塞规 [m,m+U]，止端塞规 [M−U,M]；通端环规 [M−U,M]，止端环规 [m,m+U]。
U 按牙数与直径 (0,1.5]、(1.5,4]、(4,8]、(8,12] 分档，塞规、环规共用，无定义组合不补值。
塞规对应 B 级、环规对应 A 级；校对规为参考尺寸，不生成 TS/ZS 校损规。)GAUGE";
    case Type::Pipe:
        return R"GAUGE(管螺纹 R / NPT · JB/T 10031 / ASME B1.20.1
R 按内置 JB/T 10031-2019 尺寸、公差、磨损及光滑圆锥规表查询。
NPT 按 1/16～6 inch 规格表查询，螺距采用表中修约值。
NPT 6 inch 环规中径原表存疑，选中时相应结果不计算。半角偏差单位为角分。)GAUGE";
    case Type::Rebar:
        return R"GAUGE(钢筋直螺纹 · 参考取值
牙型选项：75°、60°、70°；70° 为非标牙型。
套筒内螺纹按 6H，丝头外螺纹按 6f；工件公差借用 GB/T 197，量规要素借用 GB/T 3934。
1～4 头仅改变导程与升角，基本大径、中径、小径不变。
塞规磨损按中径下限−W，环规按中径名义值+W。ZZ、ZS 校对规大径按丝头中径极限为基准。
L=nP，ψ=atan[L/(πD₂)]，最佳针径 d₀=P/[2cos(牙型半角)]。)GAUGE";
    case Type::Smooth:
        return R"GAUGE(光滑量规 · GB/T 1957 / JJG 343-2012
IT、T1、Z1 的表值由 μm 换算为 mm。
孔通规：D+EI+Z1±T1/2；孔止规：[D+ES−T1,D+ES]；磨损极限 D+EI。
轴通规：D+es−Z1±T1/2；轴止规：[D+ei,D+ei+T1]；磨损极限 D+es。
JS/js 对称布置。需要等级相关 Δ 修正但未收录规则的组合不计算。)GAUGE";
    }
    return "";
}
} // namespace toolbox::gauges
