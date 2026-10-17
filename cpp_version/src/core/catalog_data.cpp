// 自动提取的静态目录；中文名称、顺序与 Rust 目录保持一致。
#include "toolbox/models.hpp"
namespace toolbox {
const std::vector<Tool> &tools() {
    static const std::vector<Tool> data{
        {"unit_pressure", L"压力", Group::Units, Kind::Unit, L"P",
         L"压力单位互换，参考页面列出帕、千帕、psi、毫米汞柱等常用单位。", Dimension::Pressure},
        {"unit_temperature", L"温度", Group::Units, Kind::Unit, L"℃",
         L"在常见温标之间换算，正确处理温度零点偏移。", Dimension::Temperature},
        {"unit_mass_flow", L"质量流量", Group::Units, Kind::Unit, L"m", L"质量流量单位换算。",
         Dimension::MassFlow},
        {"unit_volume_flow", L"体积流量", Group::Units, Kind::Unit, L"V", L"体积流量单位换算。",
         Dimension::VolumeFlow},
        {"unit_mass", L"质量", Group::Units, Kind::Unit, L"kg", L"质量单位换算。", Dimension::Mass},
        {"unit_length", L"长度", Group::Units, Kind::Unit, L"L", L"长度单位换算。", Dimension::Length},
        {"unit_area", L"面积", Group::Units, Kind::Unit, L"A", L"面积单位换算。", Dimension::Area},
        {"unit_volume", L"容积", Group::Units, Kind::Unit, L"V", L"容积单位换算。", Dimension::Volume},
        {"unit_density", L"密度", Group::Units, Kind::Unit, L"ρ", L"密度单位换算。", Dimension::Density},
        {"unit_velocity", L"速度", Group::Units, Kind::Unit, L"v", L"速度单位换算。", Dimension::Velocity},
        {"unit_force", L"力", Group::Units, Kind::Unit, L"F", L"力单位换算。", Dimension::Force},
        {"unit_energy", L"能量", Group::Units, Kind::Unit, L"E", L"能量单位换算。", Dimension::Energy},
        {"unit_power", L"功率", Group::Units, Kind::Unit, L"W", L"功率单位换算。", Dimension::Power},
        {"unit_heat_transfer_coefficient", L"传热系数", Group::Units, Kind::Unit, L"h",
         L"表面传热系数单位换算。", Dimension::HeatTransferCoefficient},
        {"unit_heat_conduct_coefficient", L"导热系数", Group::Units, Kind::Unit, L"λ", L"热导率单位换算。",
         Dimension::ThermalConductivity},
        {"unit_dynamic_viscosity", L"动力粘度", Group::Units, Kind::Unit, L"μ", L"动力粘度单位换算。",
         Dimension::DynamicViscosity},
        {"unit_kinematic_viscosity", L"运动粘度", Group::Units, Kind::Unit, L"ν", L"运动粘度单位换算。",
         Dimension::KinematicViscosity},
        {"unit_diffusion_coefficient", L"扩散系数", Group::Units, Kind::Unit, L"D", L"扩散系数单位换算。",
         Dimension::DiffusionCoefficient},
        {"unit_surface_tension", L"表面张力", Group::Units, Kind::Unit, L"γ", L"表面张力单位换算。",
         Dimension::SurfaceTension},
        {"unit_heat_transfer_speed", L"传热速度", Group::Units, Kind::Unit, L"q", L"热流密度单位换算。",
         Dimension::HeatTransferRate},
        {"unit_torque", L"扭矩", Group::Units, Kind::Unit, L"τ", L"扭矩单位换算。", Dimension::Torque},
        {"unit_concentration", L"浓度", Group::Units, Kind::Unit, L"c",
         L"ppm、ppb、ppt、体积比、质量比和等密度基准浓度换算。", Dimension::Concentration},
        {"working_thermocouple", L"工业热电偶", Group::Its90, Kind::WorkingThermocouple, L"TC",
         L"15 种分度号双向换算、参考端补偿、等级允差与分度表。"},
        {"industrial_thermal_resistance", L"工业热电阻", Group::Its90, Kind::IndustrialResistance, L"RTD",
         L"22 种分度选项，铂、铜、镍电阻双向换算与等级允差。"},
        {"standard_thermocouple", L"标准热电偶", Group::Its90, Kind::StandardThermocouple, L"RTC",
         L"S/B 型固定点校准、温区系数拟合与分度表。"},
        {"standard_platinum_resistor_sprt", L"标准铂电阻", Group::Its90, Kind::StandardPlatinumResistor,
         L"SPRT", L"SPRT 子区间偏差、银点修正、固定点拟合与分度表。"},
        {"thermistor", L"热敏电阻", Group::Its90, Kind::Thermistor, L"NTC",
         L"多项式、Steinhart–Hart、校准点拟合及 β 模型。"},
        {"pressure_reference_selection", L"压力表标准器选型", Group::Pressure, Kind::PressureReference, L"表",
         L"数字 / 活塞标准器核验、被检量程与检定范围推荐。"},
        {"pressure_transmitter_reference_selection", L"压力变送器标准器选型", Group::Pressure,
         Kind::TransmitterReference, L"P/I", L"合成压力与电测不确定度，核验并推荐标准器。"},
        {"altitude_difference_correction", L"高度差修正", Group::Pressure, Kind::HeightCorrection, L"Δh",
         L"按介质密度、重力加速度和高差计算液柱静压修正。"},
        {"altitude_pressure_converter", L"气压高度换算", Group::Pressure, Kind::AltitudePressure, L"hPa",
         L"按标准大气模型在气压和气压高度之间互算。"},
        {"temperature_humidity_conversion", L"湿度换算", Group::Humidity, Kind::HumidityConversion, L"RH",
         L"由相对湿度、露/霜点、湿球或混合比换算全部湿空气参数。"},
        {"ear_thermometer_blackbody", L"耳温计黑体发射率修正", Group::Humidity, Kind::EarBlackbody, L"ε",
         L"估算发射率小于 1 时带来的辐射温度修正。"},
        {"blackbody_radiation_source", L"黑体辐射源发射率误差", Group::Humidity, Kind::BlackbodyError, L"εb",
         L"计算黑体发射率偏离 1 时的辐射测温误差。"},
        {"radiation_thermometer", L"辐射温度计示值修正", Group::Humidity, Kind::RadiationCorrection, L"IR",
         L"按灰体辐射模型换算示值修正值。"},
        {"repeatability_calculation", L"重复性计算", Group::Errors, Kind::Repeatability, L"σ",
         L"计算均值、样本标准差、极差与相对标准偏差。"},
        {"range_method_calculation", L"极差法计算", Group::Errors, Kind::RangeMethod, L"R",
         L"按测量次数和极差系数估算重复性标准差及 A 类不确定度。"},
        {"resultant_error", L"合成误差", Group::Errors, Kind::ResultantError, L"Σ",
         L"按平方和根合成独立误差分量。"},
        {"error_analysis", L"误差分析", Group::Errors, Kind::ErrorAnalysis, L"Δ",
         L"计算绝对误差、相对误差和引用误差。"},
        {"uncertainty_calculation", L"不确定度计算", Group::Errors, Kind::Uncertainty, L"u",
         L"合成 A 类和 B 类标准不确定度并求扩展不确定度。"},
        {"transmitter", L"变送器", Group::Other, Kind::Transmitter, L"I/O",
         L"压力、温度和湿度变送器量程换算；压力支持线性与开方输出。"},
        {"bspline_interpolation", L"三次均匀 B 样条插值", Group::Other, Kind::BSplineInterpolation, L"B3",
         L"按自然端点条件拟合 Q 型值点，支持区间内插与端点外推。"},
    };
    return data;
}
const std::vector<Unit> &units(Dimension dimension) {
    constexpr double kIN = 0.0254;
    constexpr double kFT = 0.3048;
    constexpr double kYD = 0.9144;
    constexpr double kLB = 0.45359237;
    constexpr double kG0 = 9.80665;
    constexpr double kLBF = kLB * kG0;
    constexpr double kSLUG = kLBF / kFT;
    constexpr double kUSGAL = 231.0 * kIN * kIN * kIN;
    constexpr double kUKGAL = 0.00454609;
    constexpr double kUSBBL = 42.0 * kUSGAL;
    constexpr double kUKBBL = 35.0 * kUKGAL;
    constexpr double kCAL = 4.1868;
    constexpr double kKCAL = kCAL * 1000.0;
    constexpr double kBTU = 1055.05585262;
    constexpr double kMETRIC_HP = 75.0 * kG0;
    constexpr double kUK_HP = 550.0 * kFT * kLBF;
    switch (dimension) {
    case Dimension::Pressure: {
        static const std::vector<Unit> data{
            {1130, L"帕", L"Pa", 1.0, 0.0},
            {1133, L"千帕", L"kPa", 1000.0, 0.0},
            {1132, L"兆帕", L"MPa", 1e6, 0.0},
            {1136, L"百帕", L"hPa", 100.0, 0.0},
            {1141, L"磅力/平方英寸", L"psi", kLBF / (kIN * kIN), 0.0},
            {1137, L"巴", L"bar", 1e5, 0.0},
            {1138, L"毫巴", L"mbar", 100.0, 0.0},
            {1156, L"英寸汞柱", L"inHg (0°C)", 133.322387415 * 25.4, 0.0},
            {1158, L"毫米汞柱", L"mmHg (0°C)", 133.322387415, 0.0},
            {1147, L"英寸水柱", L"inH₂O (4°C)", 9.80638 * 25.4, 0.0},
            {1150, L"毫米水柱", L"mmH₂O (4°C)", 9.80638, 0.0},
            {1145, L"千克力/平方厘米", L"kgf/cm²", kG0 * 1e4, 0.0},
            {1140, L"标准大气压", L"atm", 101325.0, 0.0},
            {2002, L"磅力/平方英尺", L"lbf/ft²", kLBF / (kFT * kFT), 0.0},
            {1139, L"托", L"torr", 101325.0 / 760.0, 0.0},
        };
        return data;
    }
    case Dimension::Temperature: {
        static const std::vector<Unit> data{
            {1001, L"摄氏度", L"°C", 1.0, 273.15},
            {1000, L"开尔文", L"K", 1.0, 0.0},
            {1002, L"华氏度", L"°F", 5.0 / 9.0, 273.15 - 32.0 * 5.0 / 9.0},
            {1003, L"兰氏度", L"°R", 5.0 / 9.0, 0.0},
            {999, L"列氏度", L"°Re", 1.25, 273.15},
        };
        return data;
    }
    case Dimension::MassFlow: {
        static const std::vector<Unit> data{
            {1322, L"千克/秒", L"kg/s", 1.0, 0.0},
            {1324, L"千克/小时", L"kg/h", 1.0 / 3600.0, 0.0},
            {1328, L"吨/小时", L"t/h", 1000.0 / 3600.0, 0.0},
            {1329, L"吨/日", L"t/d", 1000.0 / 86400.0, 0.0},
            {50667, L"吨/工业年（8000 小时）", L"t/a", 1.0 / 28800.0, 0.0},
            {1330, L"磅/秒", L"lb/s", kLB, 0.0},
            {1332, L"磅/时", L"lb/h", kLB / 3600.0, 0.0},
            {1333, L"磅/日", L"lb/d", kLB / 86400.0, 0.0},
        };
        return data;
    }
    case Dimension::VolumeFlow: {
        static const std::vector<Unit> data{
            {1347, L"立方米/秒", L"m³/s", 1.0, 0.0},
            {1348, L"立方米/分", L"m³/min", 1.0 / 60.0, 0.0},
            {1349, L"立方米/时", L"m³/h", 1.0 / 3600.0, 0.0},
            {1351, L"升/秒", L"L/s", 0.001, 0.0},
            {1352, L"升/分", L"L/min", 0.001 / 60.0, 0.0},
            {1357, L"立方英尺/分", L"CFM", kFT * kFT * kFT / 60.0, 0.0},
            {1358, L"立方英尺/小时", L"CFH", kFT * kFT * kFT / 3600.0, 0.0},
            {1371, L"美油桶/秒", L"bbl/s", kUSBBL, 0.0},
            {1362, L"美加仑/秒", L"gal/s", kUSGAL, 0.0},
            {1367, L"英加仑/秒", L"ImpGal/s", kUKGAL, 0.0},
        };
        return data;
    }
    case Dimension::Mass: {
        static const std::vector<Unit> data{
            {1092, L"吨", L"t", 1000.0, 0.0},
            {1088, L"千克", L"kg", 1.0, 0.0},
            {1089, L"克", L"g", 0.001, 0.0},
            {1090, L"毫克", L"mg", 1e-6, 0.0},
            {49156, L"市担", L"Chinese picul", 50.0, 0.0},
            {49157, L"市斤", L"Chinese pound", 0.5, 0.0},
            {1094, L"磅", L"lb", kLB, 0.0},
            {1093, L"盎司", L"oz", kLB / 16.0, 0.0},
            {1096, L"英吨", L"ton", kLB * 2240.0, 0.0},
            {1095, L"美吨", L"sh.ton", kLB * 2000.0, 0.0},
            {49153, L"英担", L"cwt", kLB * 112.0, 0.0},
            {49152, L"美担", L"sh.cwt", kLB * 100.0, 0.0},
            {49154, L"格令", L"gr", kLB / 7000.0, 0.0},
            {49155, L"打兰", L"dr", kLB / 256.0, 0.0},
        };
        return data;
    }
    case Dimension::Length: {
        static const std::vector<Unit> data{
            {1011, L"公里", L"km", 1000.0, 0.0},
            {1010, L"米", L"m", 1.0, 0.0},
            {1694, L"分米", L"dm", 0.1, 0.0},
            {1012, L"厘米", L"cm", 0.01, 0.0},
            {1013, L"毫米", L"mm", 0.001, 0.0},
            {1014, L"微米", L"um", 1e-6, 0.0},
            {49254, L"密耳", L"mil", kIN / 1000.0, 0.0},
            {49253, L"市尺", L"Chinese feet", 1.0 / 3.0, 0.0},
            {1019, L"英寸", L"in", kIN, 0.0},
            {1018, L"英尺", L"ft", kFT, 0.0},
            {1020, L"码", L"yd", kYD, 0.0},
            {1021, L"英里", L"mile", 1609.344, 0.0},
            {1022, L"海里", L"n mile", 1852.0, 0.0},
        };
        return data;
    }
    case Dimension::Area: {
        static const std::vector<Unit> data{
            {1024, L"平方公里", L"km²", 1e6, 0.0},
            {1023, L"平方米", L"m²", 1.0, 0.0},
            {1025, L"平方厘米", L"cm²", 1e-4, 0.0},
            {1027, L"平方毫米", L"mm²", 1e-6, 0.0},
            {1029, L"公顷", L"ha", 1e4, 0.0},
            {49354, L"市亩", L"Chinese acre", 2000.0 / 3.0, 0.0},
            {1030, L"平方英寸", L"in²", kIN * kIN, 0.0},
            {1031, L"平方英尺", L"ft²", kFT * kFT, 0.0},
            {1032, L"平方码", L"yd²", kYD * kYD, 0.0},
            {1033, L"平方英里", L"mile²", 1609.344 * 1609.344, 0.0},
            {49355, L"英亩", L"acre", 43560.0 * kFT * kFT, 0.0},
            {49356, L"坪", L"Pyeong", 400.0 / 121.0, 0.0},
        };
        return data;
    }
    case Dimension::Volume: {
        static const std::vector<Unit> data{
            {1034, L"立方米", L"m³", 1.0, 0.0},
            {1035, L"立方分米", L"dm³", 0.001, 0.0},
            {1036, L"立方厘米", L"cm³", 1e-6, 0.0},
            {1037, L"立方毫米", L"mm³", 1e-9, 0.0},
            {1517, L"千升", L"kL", 1.0, 0.0},
            {1038, L"公升", L"L", 0.001, 0.0},
            {1040, L"毫升", L"ml", 1e-6, 0.0},
            {1042, L"立方英寸", L"in³", kIN * kIN * kIN, 0.0},
            {1043, L"立方英尺", L"ft³", kFT * kFT * kFT, 0.0},
            {1044, L"立方码", L"yd³", kYD * kYD * kYD, 0.0},
            {1049, L"英加仑", L"ImpGal", kUKGAL, 0.0},
            {1048, L"美加仑", L"gallon", kUSGAL, 0.0},
            {49455, L"英制石油桶（35 英制加仑）", L"UKbbl", kUKBBL, 0.0},
            {49456, L"美制石油桶（42 美制加仑）", L"USbbl", kUSBBL, 0.0},
        };
        return data;
    }
    case Dimension::Density: {
        static const std::vector<Unit> data{
            {1102, L"吨/立方米", L"t/m³", 1000.0, 0.0},
            {1097, L"千克/立方米", L"kg/m³", 1.0, 0.0},
            {1103, L"千克/升", L"kg/L", 1000.0, 0.0},
            {1101, L"克/立方米", L"g/m³", 0.001, 0.0},
            {1100, L"克/立方厘米", L"g/cm³", 1000.0, 0.0},
            {1104, L"克/毫升", L"g/ml", 1000.0, 0.0},
            {1106, L"磅/立方英寸", L"lb/in³", kLB / (kIN * kIN * kIN), 0.0},
            {1107, L"磅/立方英尺", L"lb/ft³", kLB / (kFT * kFT * kFT), 0.0},
            {49556, L"英吨/立方码", L"UKton/yd³", 2240.0 * kLB / (kYD * kYD * kYD), 0.0},
            {1430, L"磅/英加仑", L"lb/ImpGal", kLB / kUKGAL, 0.0},
            {1108, L"磅/美加仑", L"lb/gal", kLB / kUSGAL, 0.0},
            {49557, L"工程密度（重力制）", L"kgf·s²/m⁴", kG0, 0.0},
        };
        return data;
    }
    case Dimension::Velocity: {
        static const std::vector<Unit> data{
            {1061, L"米/秒", L"m/s", 1.0, 0.0},
            {49658, L"米/分", L"m/min", 1.0 / 60.0, 0.0},
            {1063, L"米/时", L"m/h", 1.0 / 3600.0, 0.0},
            {1064, L"千米/小时", L"km/h", 1.0 / 3.6, 0.0},
            {1066, L"英寸/秒", L"in/s", kIN, 0.0},
            {1072, L"英寸/小时", L"in/h", kIN / 3600.0, 0.0},
            {1067, L"英尺/秒", L"ft/s", kFT, 0.0},
            {1070, L"英尺/分", L"ft/min", kFT / 60.0, 0.0},
            {1068, L"码/秒", L"yd/s", kYD, 0.0},
            {1074, L"码/小时", L"yd/h", kYD / 3600.0, 0.0},
            {1075, L"英里/小时", L"mile/h", 1609.344 / 3600.0, 0.0},
            {49659, L"海里/小时", L"n mile/h", 1852.0 / 3600.0, 0.0},
            {1065, L"节", L"knots", 1852.0 / 3600.0, 0.0},
            {49657, L"英节", L"UK knot", 6080.0 * kFT / 3600.0, 0.0},
        };
        return data;
    }
    case Dimension::Force: {
        static const std::vector<Unit> data{
            {49758, L"吨力", L"tf", kG0 * 1000.0, 0.0},
            {49759, L"千克力", L"kgf", kG0, 0.0},
            {49760, L"克力", L"gf", kG0 / 1000.0, 0.0},
            {1120, L"牛", L"N", 1.0, 0.0},
            {49767, L"千牛", L"kN", 1000.0, 0.0},
            {49768, L"毫牛", L"mN", 0.001, 0.0},
            {49769, L"纳牛", L"nN", 1e-9, 0.0},
            {49770, L"微牛", L"uN", 1e-6, 0.0},
            {49761, L"达因", L"dyn", 1e-5, 0.0},
            {49762, L"磅力", L"lbf", kLBF, 0.0},
            {49763, L"磅达", L"pdl", kLB * kFT, 0.0},
            {49764, L"英吨力", L"tonf", 2240.0 * kLBF, 0.0},
            {49765, L"美吨力", L"US tonf", 2000.0 * kLBF, 0.0},
            {49766, L"千磅力", L"kip", 1000.0 * kLBF, 0.0},
        };
        return data;
    }
    case Dimension::Energy: {
        static const std::vector<Unit> data{
            {1167, L"焦耳", L"J", 1.0, 0.0},
            {1174, L"毫焦耳", L"mJ", 0.001, 0.0},
            {1173, L"千焦", L"kJ", 1000.0, 0.0},
            {1172, L"兆焦", L"MJ", 1e6, 0.0},
            {49866, L"牛顿·米", L"N·m", 1.0, 0.0},
            {1179, L"千瓦·小时", L"kWh", 3.6e6, 0.0},
            {49859, L"瓦特·秒", L"W·s", 1.0, 0.0},
            {49860, L"千克力·米", L"kgf·m", kG0, 0.0},
            {49861, L"升·标准大气压", L"l·atm", 101.325, 0.0},
            {1180, L"卡（国际蒸汽表）", L"cal", kCAL, 0.0},
            {1181, L"千卡（国际蒸汽表）", L"kcal", kKCAL, 0.0},
            {49862, L"公制马力·小时", L"Hp·h", kMETRIC_HP * 3600.0, 0.0},
            {49863, L"英制机械马力·小时", L"UKHp·h", kUK_HP * 3600.0, 0.0},
            {49864, L"英尺磅力", L"ft·lbf", kFT * kLBF, 0.0},
            {1183, L"英热单位（国际蒸汽表）", L"Btu", kBTU, 0.0},
            {49865, L"尔格", L"erg", 1e-7, 0.0},
        };
        return data;
    }
    case Dimension::Power: {
        static const std::vector<Unit> data{
            {1191, L"毫瓦", L"mW", 0.001, 0.0},
            {1186, L"瓦", L"W", 1.0, 0.0},
            {1190, L"千瓦", L"kW", 1000.0, 0.0},
            {1189, L"兆瓦", L"MW", 1e6, 0.0},
            {49963, L"千克力米每秒", L"kgf·m/s", kG0, 0.0},
            {1198, L"公制马力", L"hp", kMETRIC_HP, 0.0},
            {49960, L"英尺磅力每秒", L"ft·lbf/s", kFT * kLBF, 0.0},
            {49961, L"英制机械马力", L"UKHp", kUK_HP, 0.0},
            {49962, L"卡/秒", L"cal/s", kCAL, 0.0},
            {1433, L"千卡/小时", L"kcal/h", kKCAL / 3600.0, 0.0},
            {1197, L"英热单位/小时", L"Btu/h", kBTU / 3600.0, 0.0},
            {49964, L"尔格/秒", L"erg/s", 1e-7, 0.0},
        };
        return data;
    }
    case Dimension::HeatTransferCoefficient: {
        static const std::vector<Unit> data{
            {50061, L"瓦/平方米·K", L"W/(m²·K)", 1.0, 0.0},
            {50062, L"瓦/平方厘米·K", L"W/(cm²·K)", 1e4, 0.0},
            {50063, L"千卡/平方米·秒·℃", L"kcal/(m²·s·°C)", kKCAL, 0.0},
            {50064, L"千卡/平方米·时·℃", L"kcal/(m²·h·°C)", kKCAL / 3600.0, 0.0},
            {50065, L"英热单位/平方英尺·秒·℉", L"Btu/(ft²·s·°F)", kBTU / (kFT * kFT) * 1.8, 0.0},
            {50066, L"英热单位/平方英尺·时·℉", L"Btu/(ft²·h·°F)", kBTU / (kFT * kFT * 3600.0) * 1.8, 0.0},
        };
        return data;
    }
    case Dimension::ThermalConductivity: {
        static const std::vector<Unit> data{
            {50162, L"瓦/米·K", L"W/(m·K)", 1.0, 0.0},
            {50163, L"瓦/厘米·K", L"W/(cm·K)", 100.0, 0.0},
            {50164, L"千卡/米·时·℃", L"kcal/(m·h·°C)", kKCAL / 3600.0, 0.0},
            {50165, L"千卡/米·秒·℃", L"kcal/(m·s·°C)", kKCAL, 0.0},
            {50166, L"卡/厘米·秒·℃", L"cal/(cm·s·°C)", kCAL / 0.01, 0.0},
            {50167, L"焦耳/厘米·秒·℃", L"J/(cm·s·°C)", 100.0, 0.0},
            {50168, L"英热单位/英寸·秒·℉", L"Btu/(in·s·°F)", kBTU / kIN * 1.8, 0.0},
            {50169, L"英热单位/英寸·时·℉", L"Btu/(in·h·°F)", kBTU / (kIN * 3600.0) * 1.8, 0.0},
            {50170, L"英热单位/英尺·秒·℉", L"Btu/(ft·s·°F)", kBTU / kFT * 1.8, 0.0},
            {50171, L"英热单位/英尺·时·℉", L"Btu/(ft·h·°F)", kBTU / (kFT * 3600.0) * 1.8, 0.0},
        };
        return data;
    }
    case Dimension::DynamicViscosity: {
        static const std::vector<Unit> data{
            {1159, L"帕·秒", L"Pa·s", 1.0, 0.0},
            {50263, L"毫帕斯卡·秒", L"mPa·s", 0.001, 0.0},
            {1161, L"泊", L"P", 0.1, 0.0},
            {1162, L"厘泊", L"cP", 0.001, 0.0},
            {50264, L"千克/米·秒", L"kg/(m·s)", 1.0, 0.0},
            {50265, L"千克/米·时", L"kg/(m·h)", 1.0 / 3600.0, 0.0},
            {50266, L"千克力·秒/平方米", L"kgf·s/m²", kG0, 0.0},
            {50267, L"达因·秒/平方厘米", L"dyn·s/cm²", 0.1, 0.0},
            {50268, L"磅达·秒/平方英尺", L"pdl·s/ft²", kLB / kFT, 0.0},
            {50269, L"磅力·秒/平方英尺", L"lbf·s/ft²", kLBF / (kFT * kFT), 0.0},
            {50270, L"磅力·时/平方英尺", L"lbf·h/ft²", kLBF * 3600.0 / (kFT * kFT), 0.0},
            {50271, L"雷恩", L"reyn", kLBF / (kIN * kIN), 0.0},
            {50272, L"磅力·秒/平方英寸", L"lbf·s/in²", kLBF / (kIN * kIN), 0.0},
            {50273, L"磅/英尺·秒", L"lb/(ft·s)", kLB / kFT, 0.0},
            {50274, L"磅/英尺·时", L"lb/(ft·h)", kLB / (kFT * 3600.0), 0.0},
            {50275, L"斯勒格/英尺·秒", L"slug/(ft·s)", kSLUG / kFT, 0.0},
        };
        return data;
    }
    case Dimension::KinematicViscosity: {
        static const std::vector<Unit> data{
            {1160, L"平方米/秒", L"m²/s", 1.0, 0.0},
            {50364, L"平方厘米/秒", L"cm²/s", 1e-4, 0.0},
            {50365, L"平方毫米/秒", L"mm²/s", 1e-6, 0.0},
            {1163, L"斯托克斯", L"St", 1e-4, 0.0},
            {1164, L"厘斯托克斯", L"cSt", 1e-6, 0.0},
            {50366, L"平方米/时", L"m²/h", 1.0 / 3600.0, 0.0},
            {50367, L"平方英寸/秒", L"in²/s", kIN * kIN, 0.0},
            {50368, L"平方英寸/时", L"in²/h", kIN * kIN / 3600.0, 0.0},
            {50369, L"平方英尺/秒", L"ft²/s", kFT * kFT, 0.0},
            {50370, L"平方英尺/时", L"ft²/h", kFT * kFT / 3600.0, 0.0},
        };
        return data;
    }
    case Dimension::DiffusionCoefficient: {
        static const std::vector<Unit> data{
            {1160, L"平方米/秒", L"m²/s", 1.0, 0.0},
            {50364, L"平方厘米/秒", L"cm²/s", 1e-4, 0.0},
            {50366, L"平方米/时", L"m²/h", 1.0 / 3600.0, 0.0},
            {50367, L"平方英寸/秒", L"in²/s", kIN * kIN, 0.0},
            {50370, L"平方英尺/时", L"ft²/h", kFT * kFT / 3600.0, 0.0},
        };
        return data;
    }
    case Dimension::SurfaceTension: {
        static const std::vector<Unit> data{
            {1115, L"千克力/米", L"kgf/m", kG0, 0.0},
            {50465, L"克力/厘米", L"gf/cm", kG0 / 10.0, 0.0},
            {1165, L"牛顿/米", L"N/m", 1.0, 0.0},
            {1166, L"毫牛/米", L"mN/m", 0.001, 0.0},
            {50466, L"达因/厘米", L"dyn/cm", 0.001, 0.0},
            {1715, L"磅力/英尺", L"lbf/ft", kLBF / kFT, 0.0},
            {50467, L"磅力/英寸", L"lbf/inch", kLBF / kIN, 0.0},
            {50468, L"牛顿/毫米", L"N/mm", 1000.0, 0.0},
        };
        return data;
    }
    case Dimension::HeatTransferRate: {
        static const std::vector<Unit> data{
            {50566, L"瓦/平方厘米", L"W/cm²", 1e4, 0.0},
            {50573, L"千瓦/平方米", L"kW/m²", 1000.0, 0.0},
            {50567, L"卡/平方厘米·秒", L"cal/(cm²·s)", kCAL * 1e4, 0.0},
            {50568, L"卡/平方厘米·时", L"cal/(cm²·h)", kCAL * 1e4 / 3600.0, 0.0},
            {50569, L"千卡/平方米·秒", L"kcal/(m²·s)", kKCAL, 0.0},
            {50570, L"千卡/平方米·时", L"kcal/(m²·h)", kKCAL / 3600.0, 0.0},
            {50571, L"英热单位/平方英尺·秒", L"Btu/(ft²·s)", kBTU / (kFT * kFT), 0.0},
            {50572, L"英热单位/平方英尺·时", L"Btu/(ft²·h)", kBTU / (kFT * kFT * 3600.0), 0.0},
        };
        return data;
    }
    case Dimension::Torque: {
        static const std::vector<Unit> data{
            {50768, L"毫牛顿·米", L"mN·m", 0.001, 0.0},
            {50769, L"牛顿·厘米", L"N·cm", 0.01, 0.0},
            {50770, L"牛顿·米", L"N·m", 1.0, 0.0},
            {50771, L"克力·厘米", L"gf·cm", kG0 * 1e-5, 0.0},
            {50772, L"千克力·厘米", L"kgf·cm", kG0 * 0.01, 0.0},
            {50773, L"千克力·米", L"kgf·m", kG0, 0.0},
            {50774, L"盎司力·英寸", L"ozf·in", kLBF / 16.0 * kIN, 0.0},
            {50775, L"磅力·英寸", L"lbf·in", kLBF * kIN, 0.0},
            {50776, L"磅力·英尺", L"lbf·ft", kLBF * kFT, 0.0},
        };
        return data;
    }
    case Dimension::Concentration: {
        static const std::vector<Unit> data{
            {1423, L"百万分之一", L"ppm", 1.0, 0.0},
            {1424, L"十亿分之一", L"ppb", 0.001, 0.0},
            {1425, L"万亿分之一", L"ppt", 1e-6, 0.0},
            {50869, L"液量盎司 / 加仑(UK)", L"fl.oz/gallon(UK)", 1e6 / 160.0, 0.0},
            {50870, L"液量盎司 / 加仑(US)", L"fl.oz/gallon(US)", 1e6 / 128.0, 0.0},
            {50871, L"立方英寸 /立方英尺", L"cu.in/cu.ft", 1e6 / 1728.0, 0.0},
            {50872, L"立方英寸 / 立方码", L"cu.in/cu.yard", 1e6 / 46656.0, 0.0},
            {50873, L"滴/ 加仑(UK)", L"drops/gallon(UK)", 1.0 / (kUKGAL * 20.0), 0.0},
            {50874, L"滴/ 加仑(US)", L"drops/gallon(US)", 1.0 / (kUSGAL * 20.0), 0.0},
            {50875, L"滴/ 立方英尺", L"drops/cu.ft", 1.0 / (kFT * kFT * kFT * 20.0), 0.0},
            {50876, L"盎司 / 磅", L"oz/pound", 1e6 / 16.0, 0.0},
            {50877, L"盎司 / 吨(UK)", L"oz/ton(UK)", 1e6 / (2240.0 * 16.0), 0.0},
            {50878, L"盎司 / 吨(US)", L"oz/ton(US)", 1e6 / (2000.0 * 16.0), 0.0},
            {50879, L"毫升 / 升", L"mL/litre", 1000.0, 0.0},
            {50880, L"毫升 / 兆升", L"mL/megalitre", 0.001, 0.0},
            {50881, L"毫升 / 立方米", L"mL/cu.metre", 1.0, 0.0},
            {50882, L"滴 / 毫升", L"drops/mL", 50000.0, 0.0},
            {50883, L"滴 / 升", L"drops/litre", 50.0, 0.0},
            {50884, L"滴 / 立方米", L"drops/cu.metre", 0.05, 0.0},
            {50885, L"毫克 / 公斤", L"milligrams/kg", 1.0, 0.0},
            {50886, L"克 / 公斤", L"grams/kg", 1000.0, 0.0},
            {50887, L"克 / 吨", L"grams/tonne", 1.0, 0.0},
            {50888, L"斯勒格/立方英尺", L"slug/ft³", kSLUG / (kFT * kFT * kFT) * 1000.0, 0.0},
            {50889, L"斯勒格/立方英寸", L"slug/in³", kSLUG / (kIN * kIN * kIN) * 1000.0, 0.0},
            {50890, L"斯勒格/立方码", L"slug/yd³", kSLUG / (kYD * kYD * kYD) * 1000.0, 0.0},
        };
        return data;
    }
    }
    throw std::runtime_error("未知单位类别");
}
std::wstring groupName(Group group) {
    static const wchar_t *names[] = {L"单位换算",   L"ITS90 温标换算", L"压力工具",
                                     L"温湿度工具", L"误差工具",       L"其他工具"};
    return names[static_cast<int>(group)];
}
std::wstring shortGroupName(Group group) {
    static const wchar_t *names[] = {L"单位", L"温标", L"压力", L"温湿度", L"误差", L"其他"};
    return names[static_cast<int>(group)];
}
std::wstring unitNotes(Dimension dimension) {
    if (dimension == Dimension::Temperature) {
        return L"本页换算绝对温度值：K = °C + 273.15，°F = 1.8×°C + 32，°R = 1.8×K，°Re = "
               L"0.8×°C（列氏温标）；例如 0 °C = 273.15 K = 32 °F。输入不能低于 0 "
               L"K；温差换算不能套用温度值的零点偏移。";
    }
    if (dimension == Dimension::Pressure) {
        return L"以 Pa 为基准换算。液柱按标注温度及标准重力计算，水柱为 4 °C、汞柱为 0 °C；1 in = 25.4 mm，1 "
               L"torr = 1 "
               L"atm/"
               L"760。换算保留输入压力的参考基准，只改变数值单位；液柱结果使用约定系数，未按现场介质密度或当"
               L"地重力调整。";
    }
    if (dimension == Dimension::MassFlow) {
        return L"按质量与时间倍率换算，1 t = 1000 kg，1 d = 24 h，1 lb = 0.45359237 kg。t/a 按每年 8000 "
               L"运行小时计算：1 kg/s = 28800 t/a。";
    }
    if (dimension == Dimension::VolumeFlow) {
        return L"按体积与时间倍率换算。CFM、CFH 分别为立方英尺/分钟和立方英尺/小时；gal/s "
               L"为美制液量加仑/秒，ImpGal/s 为英制加仑/秒；bbl/s 按 42 美制加仑的石油桶计算。";
    }
    if (dimension == Dimension::Velocity) {
        return L"各速度单位按长度与时间倍率换算。1 mile = 1609.344 m；国际海里为 1852 m，knots 与 n mile/h "
               L"等值。UK knot 按历史英制海里 6080 ft/h 换算。";
    }
    if (dimension == Dimension::Force) {
        return L"目标数值 = 输入数值 × 输入单位的 SI 倍率 / 目标单位的 SI 倍率。kgf 与 tf 按标准重力加速度 "
               L"9.80665 m/s² 定义，1 kgf = 9.80665 N，1 tf = 1000 kgf。";
    }
    if (dimension == Dimension::Torque) {
        return L"力矩按力与力臂长度倍率换算，以 N·m 为基准。kgf 按标准重力加速度 9.80665 m/s² 定义；lbf·ft "
               L"与 lbf·in 分别计入英尺和英寸的长度倍率。1 lbf·ft = 12 lbf·in，1 lbf·in = 16 ozf·in。";
    }
    if (dimension == Dimension::Energy) {
        return L"热量采用国际蒸汽表定义，1 cal = 4.1868 J，1 kcal = 4186.8 J，1 Btu = 1055.05585262 J。Hp·h "
               L"采用公制马力，UKHp·h 采用英制机械马力；1 kWh = 3600000 J。";
    }
    if (dimension == Dimension::Power) {
        return L"功率按能量/时间换算，1 W = 1 J/s。hp 表示公制马力（735.49875 W），UKHp "
               L"表示英制机械马力（550 ft·lbf/s）。cal、kcal 与 Btu 均采用国际蒸汽表定义，1 kcal/h = "
               L"4186.8/3600 W。";
    }
    if (dimension == Dimension::HeatTransferCoefficient) {
        return L"按能量、时间、面积及温差的倍率换算。分母中的温度单位表示温差，1 °C 温差 = 1 K，1 °F 温差 = "
               L"5/9 K；cal、kcal、Btu 统一采用国际蒸汽表定义（1 cal = 4.1868 J）。";
    }
    if (dimension == Dimension::ThermalConductivity) {
        return L"按能量、时间、长度及温差的倍率换算，1 °C 温差 = 1 K，1 °F 温差 = 5/9 K；cal、kcal、Btu "
               L"统一采用国际蒸汽表定义（1 cal = 4.1868 J）。kcal/(m·s·°C) 按符号中的米计算，1 kcal/(m·s·°C) "
               L"= 4186.8 W/(m·K)。";
    }
    if (dimension == Dimension::HeatTransferRate) {
        return L"本页“传热速度”指热流密度，按功率/面积换算。1 W/cm² = 10 kW/m²。cal、kcal 与 Btu "
               L"采用国际蒸汽表定义（1 cal = 4.1868 J，1 Btu = 1055.05585262 J）；面积及时间倍率同时计入。";
    }
    if (dimension == Dimension::Volume) {
        return L"按体积单位倍率换算，1 L = 0.001 m³，1 kL = 1 m³。ImpGal 为英制加仑（4.54609 L），gallon "
               L"为美制液量加仑（3.785411784 L）。USbbl 按 42 美制加仑，UKbbl 按 35 英制加仑计算。";
    }
    if (dimension == Dimension::Density) {
        return L"按质量/体积的单位倍率换算，1 g/cm³ = 1000 kg/m³，1 kg/L = 1000 kg/m³。英吨取 2240 "
               L"lb，英制与美制加仑分别计算。工程密度单位为 kgf·s²/m⁴，1 kgf·s²/m⁴ = 9.80665 kg/m³。";
    }
    if (dimension == Dimension::DynamicViscosity) {
        return L"动力粘度以 Pa·s 为基准，1 P = 0.1 Pa·s，1 cP = 1 mPa·s。dyn·s/cm² 与 P 等值；reyn 与 "
               L"lbf·s/in² 等值。力、质量、面积和小时/秒的倍率分别计入，lb 为质量，lbf 为力。";
    }
    if (dimension == Dimension::SurfaceTension) {
        return L"表面张力按力/长度换算，以 N/m 为基准。kgf/m、gf/cm、lbf/ft、lbf/inch 中的分子均表示力；1 "
               L"kgf/m = 9.80665 N/m，1 dyn/cm = 1 mN/m。";
    }
    if (dimension == Dimension::Concentration) {
        return L"ppm、ppb、ppt 分别表示百万、十亿、万亿分之一；1 ppm = 1000 ppb = 1000000 ppt。滴的约定为 20 "
               L"滴/mL。\n质量比与体积比按等密度基准列示；slug/体积按溶液及溶质密度均为 1000 kg/m³ "
               L"折算，因此 1 ppm 对应 1 mg/kg、1 mL/m³、0.001 "
               L"kg/m³。真实物质的质量比、体积比、质量浓度互换需要实际密度，不能直接套用此基准。";
    }
    if (dimension == Dimension::Mass) {
        return L"公制质量以 kg 为基准。1 市担 = 50 kg，1 市斤 = 0.5 kg；lb 为常衡磅（0.45359237 kg），1 oz = "
               L"1/16 lb，1 dr = 1/256 lb，1 gr = 1/7000 lb。英吨、英担分别为 2240 lb、112 "
               L"lb，美吨、美担分别为 2000 lb、100 lb。";
    }
    if (dimension == Dimension::Length) {
        return L"按长度倍率换算：1 in = 0.0254 m，1 ft = 0.3048 m，1 yd = 0.9144 m，1 mile = 1609.344 m，1 n "
               L"mile = 1852 m。1 市尺 = 1/3 m；1 mil（密耳）= 0.001 in = 0.0254 mm；um 表示微米。";
    }
    if (dimension == Dimension::Area) {
        return L"面积倍率按长度倍率的平方计算：1 ft² = 0.09290304 m²。1 ha = 10000 m²，1 市亩 = 2000/3 m²，1 "
               L"坪 = 400/121 m²。本页 acre 按 43560 平方国际英尺计算，等于 4046.8564224 m²。";
    }
    if (dimension == Dimension::KinematicViscosity) {
        return L"运动粘度量纲为面积/时间，按长度倍率的平方及时间倍率换算：1 St = 1 cm²/s = 0.0001 m²/s，1 "
               L"cSt = 1 mm²/s = 0.000001 m²/s。1 m²/h = 1/3600 "
               L"m²/s；平方英尺、平方英寸分别计入长度倍率的平方。";
    }
    if (dimension == Dimension::DiffusionCoefficient) {
        return L"扩散系数量纲为面积/时间，按长度倍率的平方及时间倍率换算：1 cm²/s = 0.0001 m²/s，1 m²/h = "
               L"1/3600 m²/s，1 in²/s = 0.00064516 m²/s。1 ft = 0.3048 m，平方英尺单位按该长度的平方计算。";
    }
    return L"";
}
} // namespace toolbox
