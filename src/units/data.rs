//! 256 项静态单位目录：保存名称、符号及由基本单位定义推导的换算系数。
use super::UnitDef;
use crate::catalog::Dimension;

const IN: f64 = 0.0254;
const FT: f64 = 0.3048;
const YD: f64 = 0.9144;
const LB: f64 = 0.453_592_37;
const G0: f64 = 9.806_65;
const LBF: f64 = LB * G0;
const SLUG: f64 = LBF / FT;
const USGAL: f64 = 231.0 * IN * IN * IN;
const UKGAL: f64 = 0.004_546_09;
const USBBL: f64 = 42.0 * USGAL;
const UKBBL: f64 = 35.0 * UKGAL;
const CAL: f64 = 4.1868;
const KCAL: f64 = CAL * 1000.0;
const BTU: f64 = 1055.055_852_62;
const METRIC_HP: f64 = 75.0 * G0;
const UK_HP: f64 = 550.0 * FT * LBF;

macro_rules! unit {
    ($id:expr, $name:expr, $code:expr, $factor:expr, $offset:expr) => {
        UnitDef {
            reference_id: $id,
            name: $name,
            code: $code,
            factor: $factor,
            offset: $offset,
        }
    };
}

const PRESSURE: &[UnitDef] = &[
    unit!(1130, "帕", "Pa", 1.0, 0.0),
    unit!(1133, "千帕", "kPa", 1000.0, 0.0),
    unit!(1132, "兆帕", "MPa", 1e6, 0.0),
    unit!(1136, "百帕", "hPa", 100.0, 0.0),
    unit!(1141, "磅力/平方英寸", "psi", LBF / (IN * IN), 0.0),
    unit!(1137, "巴", "bar", 1e5, 0.0),
    unit!(1138, "毫巴", "mbar", 100.0, 0.0),
    unit!(1156, "英寸汞柱", "inHg (0°C)", 133.322387415 * 25.4, 0.0),
    unit!(1158, "毫米汞柱", "mmHg (0°C)", 133.322387415, 0.0),
    unit!(1147, "英寸水柱", "inH₂O (4°C)", 9.80638 * 25.4, 0.0),
    unit!(1150, "毫米水柱", "mmH₂O (4°C)", 9.80638, 0.0),
    unit!(1145, "千克力/平方厘米", "kgf/cm²", G0 * 1e4, 0.0),
    unit!(1140, "标准大气压", "atm", 101325.0, 0.0),
    unit!(2002, "磅力/平方英尺", "lbf/ft²", LBF / (FT * FT), 0.0),
    unit!(1139, "托", "torr", 101325.0 / 760.0, 0.0),
];

const TEMPERATURE: &[UnitDef] = &[
    unit!(1001, "摄氏度", "°C", 1.0, 273.15),
    unit!(1000, "开尔文", "K", 1.0, 0.0),
    unit!(1002, "华氏度", "°F", 5.0 / 9.0, 273.15 - 32.0 * 5.0 / 9.0),
    unit!(1003, "兰氏度", "°R", 5.0 / 9.0, 0.0),
    unit!(999, "列氏度", "°Re", 1.25, 273.15),
];

const MASS_FLOW: &[UnitDef] = &[
    unit!(1322, "千克/秒", "kg/s", 1.0, 0.0),
    unit!(1324, "千克/小时", "kg/h", 1.0 / 3600.0, 0.0),
    unit!(1328, "吨/小时", "t/h", 1000.0 / 3600.0, 0.0),
    unit!(1329, "吨/日", "t/d", 1000.0 / 86400.0, 0.0),
    unit!(50667, "吨/工业年（8000 小时）", "t/a", 1.0 / 28800.0, 0.0),
    unit!(1330, "磅/秒", "lb/s", LB, 0.0),
    unit!(1332, "磅/时", "lb/h", LB / 3600.0, 0.0),
    unit!(1333, "磅/日", "lb/d", LB / 86400.0, 0.0),
];

const VOLUME_FLOW: &[UnitDef] = &[
    unit!(1347, "立方米/秒", "m³/s", 1.0, 0.0),
    unit!(1348, "立方米/分", "m³/min", 1.0 / 60.0, 0.0),
    unit!(1349, "立方米/时", "m³/h", 1.0 / 3600.0, 0.0),
    unit!(1351, "升/秒", "L/s", 0.001, 0.0),
    unit!(1352, "升/分", "L/min", 0.001 / 60.0, 0.0),
    unit!(1357, "立方英尺/分", "CFM", FT * FT * FT / 60.0, 0.0),
    unit!(1358, "立方英尺/小时", "CFH", FT * FT * FT / 3600.0, 0.0),
    unit!(1371, "美油桶/秒", "bbl/s", USBBL, 0.0),
    unit!(1362, "美加仑/秒", "gal/s", USGAL, 0.0),
    unit!(1367, "英加仑/秒", "ImpGal/s", UKGAL, 0.0),
];

const MASS: &[UnitDef] = &[
    unit!(1092, "吨", "t", 1000.0, 0.0),
    unit!(1088, "千克", "kg", 1.0, 0.0),
    unit!(1089, "克", "g", 0.001, 0.0),
    unit!(1090, "毫克", "mg", 1e-6, 0.0),
    unit!(49156, "市担", "Chinese picul", 50.0, 0.0),
    unit!(49157, "市斤", "Chinese pound", 0.5, 0.0),
    unit!(1094, "磅", "lb", LB, 0.0),
    unit!(1093, "盎司", "oz", LB / 16.0, 0.0),
    unit!(1096, "英吨", "ton", LB * 2240.0, 0.0),
    unit!(1095, "美吨", "sh.ton", LB * 2000.0, 0.0),
    unit!(49153, "英担", "cwt", LB * 112.0, 0.0),
    unit!(49152, "美担", "sh.cwt", LB * 100.0, 0.0),
    unit!(49154, "格令", "gr", LB / 7000.0, 0.0),
    unit!(49155, "打兰", "dr", LB / 256.0, 0.0),
];

const LENGTH: &[UnitDef] = &[
    unit!(1011, "公里", "km", 1000.0, 0.0),
    unit!(1010, "米", "m", 1.0, 0.0),
    unit!(1694, "分米", "dm", 0.1, 0.0),
    unit!(1012, "厘米", "cm", 0.01, 0.0),
    unit!(1013, "毫米", "mm", 0.001, 0.0),
    unit!(1014, "微米", "um", 1e-6, 0.0),
    unit!(49254, "密耳", "mil", IN / 1000.0, 0.0),
    unit!(49253, "市尺", "Chinese feet", 1.0 / 3.0, 0.0),
    unit!(1019, "英寸", "in", IN, 0.0),
    unit!(1018, "英尺", "ft", FT, 0.0),
    unit!(1020, "码", "yd", YD, 0.0),
    unit!(1021, "英里", "mile", 1609.344, 0.0),
    unit!(1022, "海里", "n mile", 1852.0, 0.0),
];

const AREA: &[UnitDef] = &[
    unit!(1024, "平方公里", "km²", 1e6, 0.0),
    unit!(1023, "平方米", "m²", 1.0, 0.0),
    unit!(1025, "平方厘米", "cm²", 1e-4, 0.0),
    unit!(1027, "平方毫米", "mm²", 1e-6, 0.0),
    unit!(1029, "公顷", "ha", 1e4, 0.0),
    unit!(49354, "市亩", "Chinese acre", 2000.0 / 3.0, 0.0),
    unit!(1030, "平方英寸", "in²", IN * IN, 0.0),
    unit!(1031, "平方英尺", "ft²", FT * FT, 0.0),
    unit!(1032, "平方码", "yd²", YD * YD, 0.0),
    unit!(1033, "平方英里", "mile²", 1609.344 * 1609.344, 0.0),
    unit!(49355, "英亩", "acre", 43560.0 * FT * FT, 0.0),
    unit!(49356, "坪", "Pyeong", 400.0 / 121.0, 0.0),
];

const VOLUME: &[UnitDef] = &[
    unit!(1034, "立方米", "m³", 1.0, 0.0),
    unit!(1035, "立方分米", "dm³", 0.001, 0.0),
    unit!(1036, "立方厘米", "cm³", 1e-6, 0.0),
    unit!(1037, "立方毫米", "mm³", 1e-9, 0.0),
    unit!(1517, "千升", "kL", 1.0, 0.0),
    unit!(1038, "公升", "L", 0.001, 0.0),
    unit!(1040, "毫升", "ml", 1e-6, 0.0),
    unit!(1042, "立方英寸", "in³", IN * IN * IN, 0.0),
    unit!(1043, "立方英尺", "ft³", FT * FT * FT, 0.0),
    unit!(1044, "立方码", "yd³", YD * YD * YD, 0.0),
    unit!(1049, "英加仑", "ImpGal", UKGAL, 0.0),
    unit!(1048, "美加仑", "gallon", USGAL, 0.0),
    unit!(49455, "英制石油桶（35 英制加仑）", "UKbbl", UKBBL, 0.0),
    unit!(49456, "美制石油桶（42 美制加仑）", "USbbl", USBBL, 0.0),
];

const DENSITY: &[UnitDef] = &[
    unit!(1102, "吨/立方米", "t/m³", 1000.0, 0.0),
    unit!(1097, "千克/立方米", "kg/m³", 1.0, 0.0),
    unit!(1103, "千克/升", "kg/L", 1000.0, 0.0),
    unit!(1101, "克/立方米", "g/m³", 0.001, 0.0),
    unit!(1100, "克/立方厘米", "g/cm³", 1000.0, 0.0),
    unit!(1104, "克/毫升", "g/ml", 1000.0, 0.0),
    unit!(1106, "磅/立方英寸", "lb/in³", LB / (IN * IN * IN), 0.0),
    unit!(1107, "磅/立方英尺", "lb/ft³", LB / (FT * FT * FT), 0.0),
    unit!(
        49556,
        "英吨/立方码",
        "UKton/yd³",
        2240.0 * LB / (YD * YD * YD),
        0.0
    ),
    unit!(1430, "磅/英加仑", "lb/ImpGal", LB / UKGAL, 0.0),
    unit!(1108, "磅/美加仑", "lb/gal", LB / USGAL, 0.0),
    unit!(49557, "工程密度（重力制）", "kgf·s²/m⁴", G0, 0.0),
];

const VELOCITY: &[UnitDef] = &[
    unit!(1061, "米/秒", "m/s", 1.0, 0.0),
    unit!(49658, "米/分", "m/min", 1.0 / 60.0, 0.0),
    unit!(1063, "米/时", "m/h", 1.0 / 3600.0, 0.0),
    unit!(1064, "千米/小时", "km/h", 1.0 / 3.6, 0.0),
    unit!(1066, "英寸/秒", "in/s", IN, 0.0),
    unit!(1072, "英寸/小时", "in/h", IN / 3600.0, 0.0),
    unit!(1067, "英尺/秒", "ft/s", FT, 0.0),
    unit!(1070, "英尺/分", "ft/min", FT / 60.0, 0.0),
    unit!(1068, "码/秒", "yd/s", YD, 0.0),
    unit!(1074, "码/小时", "yd/h", YD / 3600.0, 0.0),
    unit!(1075, "英里/小时", "mile/h", 1609.344 / 3600.0, 0.0),
    unit!(49659, "海里/小时", "n mile/h", 1852.0 / 3600.0, 0.0),
    unit!(1065, "节", "knots", 1852.0 / 3600.0, 0.0),
    unit!(49657, "英节", "UK knot", 6080.0 * FT / 3600.0, 0.0),
];

const FORCE: &[UnitDef] = &[
    unit!(49758, "吨力", "tf", G0 * 1000.0, 0.0),
    unit!(49759, "千克力", "kgf", G0, 0.0),
    unit!(49760, "克力", "gf", G0 / 1000.0, 0.0),
    unit!(1120, "牛", "N", 1.0, 0.0),
    unit!(49767, "千牛", "kN", 1000.0, 0.0),
    unit!(49768, "毫牛", "mN", 0.001, 0.0),
    unit!(49769, "纳牛", "nN", 1e-9, 0.0),
    unit!(49770, "微牛", "uN", 1e-6, 0.0),
    unit!(49761, "达因", "dyn", 1e-5, 0.0),
    unit!(49762, "磅力", "lbf", LBF, 0.0),
    unit!(49763, "磅达", "pdl", LB * FT, 0.0),
    unit!(49764, "英吨力", "tonf", 2240.0 * LBF, 0.0),
    unit!(49765, "美吨力", "US tonf", 2000.0 * LBF, 0.0),
    unit!(49766, "千磅力", "kip", 1000.0 * LBF, 0.0),
];

const ENERGY: &[UnitDef] = &[
    unit!(1167, "焦耳", "J", 1.0, 0.0),
    unit!(1174, "毫焦耳", "mJ", 0.001, 0.0),
    unit!(1173, "千焦", "kJ", 1000.0, 0.0),
    unit!(1172, "兆焦", "MJ", 1e6, 0.0),
    unit!(49866, "牛顿·米", "N·m", 1.0, 0.0),
    unit!(1179, "千瓦·小时", "kWh", 3.6e6, 0.0),
    unit!(49859, "瓦特·秒", "W·s", 1.0, 0.0),
    unit!(49860, "千克力·米", "kgf·m", G0, 0.0),
    unit!(49861, "升·标准大气压", "l·atm", 101.325, 0.0),
    unit!(1180, "卡（国际蒸汽表）", "cal", CAL, 0.0),
    unit!(1181, "千卡（国际蒸汽表）", "kcal", KCAL, 0.0),
    unit!(49862, "公制马力·小时", "Hp·h", METRIC_HP * 3600.0, 0.0),
    unit!(49863, "英制机械马力·小时", "UKHp·h", UK_HP * 3600.0, 0.0),
    unit!(49864, "英尺磅力", "ft·lbf", FT * LBF, 0.0),
    unit!(1183, "英热单位（国际蒸汽表）", "Btu", BTU, 0.0),
    unit!(49865, "尔格", "erg", 1e-7, 0.0),
];

const POWER: &[UnitDef] = &[
    unit!(1191, "毫瓦", "mW", 0.001, 0.0),
    unit!(1186, "瓦", "W", 1.0, 0.0),
    unit!(1190, "千瓦", "kW", 1000.0, 0.0),
    unit!(1189, "兆瓦", "MW", 1e6, 0.0),
    unit!(49963, "千克力米每秒", "kgf·m/s", G0, 0.0),
    unit!(1198, "公制马力", "hp", METRIC_HP, 0.0),
    unit!(49960, "英尺磅力每秒", "ft·lbf/s", FT * LBF, 0.0),
    unit!(49961, "英制机械马力", "UKHp", UK_HP, 0.0),
    unit!(49962, "卡/秒", "cal/s", CAL, 0.0),
    unit!(1433, "千卡/小时", "kcal/h", KCAL / 3600.0, 0.0),
    unit!(1197, "英热单位/小时", "Btu/h", BTU / 3600.0, 0.0),
    unit!(49964, "尔格/秒", "erg/s", 1e-7, 0.0),
];

const HEAT_TRANSFER_COEFFICIENT: &[UnitDef] = &[
    unit!(50061, "瓦/平方米·K", "W/(m²·K)", 1.0, 0.0),
    unit!(50062, "瓦/平方厘米·K", "W/(cm²·K)", 1e4, 0.0),
    unit!(50063, "千卡/平方米·秒·℃", "kcal/(m²·s·°C)", KCAL, 0.0),
    unit!(
        50064,
        "千卡/平方米·时·℃",
        "kcal/(m²·h·°C)",
        KCAL / 3600.0,
        0.0
    ),
    unit!(
        50065,
        "英热单位/平方英尺·秒·℉",
        "Btu/(ft²·s·°F)",
        BTU / (FT * FT) * 1.8,
        0.0
    ),
    unit!(
        50066,
        "英热单位/平方英尺·时·℉",
        "Btu/(ft²·h·°F)",
        BTU / (FT * FT * 3600.0) * 1.8,
        0.0
    ),
];

const HEAT_CONDUCT_COEFFICIENT: &[UnitDef] = &[
    unit!(50162, "瓦/米·K", "W/(m·K)", 1.0, 0.0),
    unit!(50163, "瓦/厘米·K", "W/(cm·K)", 100.0, 0.0),
    unit!(50164, "千卡/米·时·℃", "kcal/(m·h·°C)", KCAL / 3600.0, 0.0),
    unit!(50165, "千卡/米·秒·℃", "kcal/(m·s·°C)", KCAL, 0.0),
    unit!(50166, "卡/厘米·秒·℃", "cal/(cm·s·°C)", CAL / 0.01, 0.0),
    unit!(50167, "焦耳/厘米·秒·℃", "J/(cm·s·°C)", 100.0, 0.0),
    unit!(
        50168,
        "英热单位/英寸·秒·℉",
        "Btu/(in·s·°F)",
        BTU / IN * 1.8,
        0.0
    ),
    unit!(
        50169,
        "英热单位/英寸·时·℉",
        "Btu/(in·h·°F)",
        BTU / (IN * 3600.0) * 1.8,
        0.0
    ),
    unit!(
        50170,
        "英热单位/英尺·秒·℉",
        "Btu/(ft·s·°F)",
        BTU / FT * 1.8,
        0.0
    ),
    unit!(
        50171,
        "英热单位/英尺·时·℉",
        "Btu/(ft·h·°F)",
        BTU / (FT * 3600.0) * 1.8,
        0.0
    ),
];

const DYNAMIC_VISCOSITY: &[UnitDef] = &[
    unit!(1159, "帕·秒", "Pa·s", 1.0, 0.0),
    unit!(50263, "毫帕斯卡·秒", "mPa·s", 0.001, 0.0),
    unit!(1161, "泊", "P", 0.1, 0.0),
    unit!(1162, "厘泊", "cP", 0.001, 0.0),
    unit!(50264, "千克/米·秒", "kg/(m·s)", 1.0, 0.0),
    unit!(50265, "千克/米·时", "kg/(m·h)", 1.0 / 3600.0, 0.0),
    unit!(50266, "千克力·秒/平方米", "kgf·s/m²", G0, 0.0),
    unit!(50267, "达因·秒/平方厘米", "dyn·s/cm²", 0.1, 0.0),
    unit!(50268, "磅达·秒/平方英尺", "pdl·s/ft²", LB / FT, 0.0),
    unit!(50269, "磅力·秒/平方英尺", "lbf·s/ft²", LBF / (FT * FT), 0.0),
    unit!(
        50270,
        "磅力·时/平方英尺",
        "lbf·h/ft²",
        LBF * 3600.0 / (FT * FT),
        0.0
    ),
    unit!(50271, "雷恩", "reyn", LBF / (IN * IN), 0.0),
    unit!(50272, "磅力·秒/平方英寸", "lbf·s/in²", LBF / (IN * IN), 0.0),
    unit!(50273, "磅/英尺·秒", "lb/(ft·s)", LB / FT, 0.0),
    unit!(50274, "磅/英尺·时", "lb/(ft·h)", LB / (FT * 3600.0), 0.0),
    unit!(50275, "斯勒格/英尺·秒", "slug/(ft·s)", SLUG / FT, 0.0),
];

const KINEMATIC_VISCOSITY: &[UnitDef] = &[
    unit!(1160, "平方米/秒", "m²/s", 1.0, 0.0),
    unit!(50364, "平方厘米/秒", "cm²/s", 1e-4, 0.0),
    unit!(50365, "平方毫米/秒", "mm²/s", 1e-6, 0.0),
    unit!(1163, "斯托克斯", "St", 1e-4, 0.0),
    unit!(1164, "厘斯托克斯", "cSt", 1e-6, 0.0),
    unit!(50366, "平方米/时", "m²/h", 1.0 / 3600.0, 0.0),
    unit!(50367, "平方英寸/秒", "in²/s", IN * IN, 0.0),
    unit!(50368, "平方英寸/时", "in²/h", IN * IN / 3600.0, 0.0),
    unit!(50369, "平方英尺/秒", "ft²/s", FT * FT, 0.0),
    unit!(50370, "平方英尺/时", "ft²/h", FT * FT / 3600.0, 0.0),
];

const DIFFUSION_COEFFICIENT: &[UnitDef] = &[
    unit!(1160, "平方米/秒", "m²/s", 1.0, 0.0),
    unit!(50364, "平方厘米/秒", "cm²/s", 1e-4, 0.0),
    unit!(50366, "平方米/时", "m²/h", 1.0 / 3600.0, 0.0),
    unit!(50367, "平方英寸/秒", "in²/s", IN * IN, 0.0),
    unit!(50370, "平方英尺/时", "ft²/h", FT * FT / 3600.0, 0.0),
];

const SURFACE_TENSION: &[UnitDef] = &[
    unit!(1115, "千克力/米", "kgf/m", G0, 0.0),
    unit!(50465, "克力/厘米", "gf/cm", G0 / 10.0, 0.0),
    unit!(1165, "牛顿/米", "N/m", 1.0, 0.0),
    unit!(1166, "毫牛/米", "mN/m", 0.001, 0.0),
    unit!(50466, "达因/厘米", "dyn/cm", 0.001, 0.0),
    unit!(1715, "磅力/英尺", "lbf/ft", LBF / FT, 0.0),
    unit!(50467, "磅力/英寸", "lbf/inch", LBF / IN, 0.0),
    unit!(50468, "牛顿/毫米", "N/mm", 1000.0, 0.0),
];

const HEAT_TRANSFER_SPEED: &[UnitDef] = &[
    unit!(50566, "瓦/平方厘米", "W/cm²", 1e4, 0.0),
    unit!(50573, "千瓦/平方米", "kW/m²", 1000.0, 0.0),
    unit!(50567, "卡/平方厘米·秒", "cal/(cm²·s)", CAL * 1e4, 0.0),
    unit!(
        50568,
        "卡/平方厘米·时",
        "cal/(cm²·h)",
        CAL * 1e4 / 3600.0,
        0.0
    ),
    unit!(50569, "千卡/平方米·秒", "kcal/(m²·s)", KCAL, 0.0),
    unit!(50570, "千卡/平方米·时", "kcal/(m²·h)", KCAL / 3600.0, 0.0),
    unit!(
        50571,
        "英热单位/平方英尺·秒",
        "Btu/(ft²·s)",
        BTU / (FT * FT),
        0.0
    ),
    unit!(
        50572,
        "英热单位/平方英尺·时",
        "Btu/(ft²·h)",
        BTU / (FT * FT * 3600.0),
        0.0
    ),
];

const TORQUE: &[UnitDef] = &[
    unit!(50768, "毫牛顿·米", "mN·m", 0.001, 0.0),
    unit!(50769, "牛顿·厘米", "N·cm", 0.01, 0.0),
    unit!(50770, "牛顿·米", "N·m", 1.0, 0.0),
    unit!(50771, "克力·厘米", "gf·cm", G0 * 1e-5, 0.0),
    unit!(50772, "千克力·厘米", "kgf·cm", G0 * 0.01, 0.0),
    unit!(50773, "千克力·米", "kgf·m", G0, 0.0),
    unit!(50774, "盎司力·英寸", "ozf·in", LBF / 16.0 * IN, 0.0),
    unit!(50775, "磅力·英寸", "lbf·in", LBF * IN, 0.0),
    unit!(50776, "磅力·英尺", "lbf·ft", LBF * FT, 0.0),
];

const CONCENTRATION: &[UnitDef] = &[
    unit!(1423, "百万分之一", "ppm", 1.0, 0.0),
    unit!(1424, "十亿分之一", "ppb", 0.001, 0.0),
    unit!(1425, "万亿分之一", "ppt", 1e-6, 0.0),
    unit!(
        50869,
        "液量盎司 / 加仑(UK)",
        "fl.oz/gallon(UK)",
        1e6 / 160.0,
        0.0
    ),
    unit!(
        50870,
        "液量盎司 / 加仑(US)",
        "fl.oz/gallon(US)",
        1e6 / 128.0,
        0.0
    ),
    unit!(
        50871,
        "立方英寸 /立方英尺",
        "cu.in/cu.ft",
        1e6 / 1728.0,
        0.0
    ),
    unit!(
        50872,
        "立方英寸 / 立方码",
        "cu.in/cu.yard",
        1e6 / 46656.0,
        0.0
    ),
    unit!(
        50873,
        "滴/ 加仑(UK)",
        "drops/gallon(UK)",
        1.0 / (UKGAL * 20.0),
        0.0
    ),
    unit!(
        50874,
        "滴/ 加仑(US)",
        "drops/gallon(US)",
        1.0 / (USGAL * 20.0),
        0.0
    ),
    unit!(
        50875,
        "滴/ 立方英尺",
        "drops/cu.ft",
        1.0 / (FT * FT * FT * 20.0),
        0.0
    ),
    unit!(50876, "盎司 / 磅", "oz/pound", 1e6 / 16.0, 0.0),
    unit!(
        50877,
        "盎司 / 吨(UK)",
        "oz/ton(UK)",
        1e6 / (2240.0 * 16.0),
        0.0
    ),
    unit!(
        50878,
        "盎司 / 吨(US)",
        "oz/ton(US)",
        1e6 / (2000.0 * 16.0),
        0.0
    ),
    unit!(50879, "毫升 / 升", "mL/litre", 1000.0, 0.0),
    unit!(50880, "毫升 / 兆升", "mL/megalitre", 0.001, 0.0),
    unit!(50881, "毫升 / 立方米", "mL/cu.metre", 1.0, 0.0),
    unit!(50882, "滴 / 毫升", "drops/mL", 50000.0, 0.0),
    unit!(50883, "滴 / 升", "drops/litre", 50.0, 0.0),
    unit!(50884, "滴 / 立方米", "drops/cu.metre", 0.05, 0.0),
    unit!(50885, "毫克 / 公斤", "milligrams/kg", 1.0, 0.0),
    unit!(50886, "克 / 公斤", "grams/kg", 1000.0, 0.0),
    unit!(50887, "克 / 吨", "grams/tonne", 1.0, 0.0),
    unit!(
        50888,
        "斯勒格/立方英尺",
        "slug/ft³",
        SLUG / (FT * FT * FT) * 1000.0,
        0.0
    ),
    unit!(
        50889,
        "斯勒格/立方英寸",
        "slug/in³",
        SLUG / (IN * IN * IN) * 1000.0,
        0.0
    ),
    unit!(
        50890,
        "斯勒格/立方码",
        "slug/yd³",
        SLUG / (YD * YD * YD) * 1000.0,
        0.0
    ),
];

pub(crate) fn units_for(dimension: Dimension) -> &'static [UnitDef] {
    match dimension {
        Dimension::Pressure => PRESSURE,
        Dimension::Temperature => TEMPERATURE,
        Dimension::MassFlow => MASS_FLOW,
        Dimension::VolumeFlow => VOLUME_FLOW,
        Dimension::Mass => MASS,
        Dimension::Length => LENGTH,
        Dimension::Area => AREA,
        Dimension::Volume => VOLUME,
        Dimension::Density => DENSITY,
        Dimension::Velocity => VELOCITY,
        Dimension::Force => FORCE,
        Dimension::Energy => ENERGY,
        Dimension::Power => POWER,
        Dimension::HeatTransferCoefficient => HEAT_TRANSFER_COEFFICIENT,
        Dimension::ThermalConductivity => HEAT_CONDUCT_COEFFICIENT,
        Dimension::DynamicViscosity => DYNAMIC_VISCOSITY,
        Dimension::KinematicViscosity => KINEMATIC_VISCOSITY,
        Dimension::DiffusionCoefficient => DIFFUSION_COEFFICIENT,
        Dimension::SurfaceTension => SURFACE_TENSION,
        Dimension::HeatTransferRate => HEAT_TRANSFER_SPEED,
        Dimension::Torque => TORQUE,
        Dimension::Concentration => CONCENTRATION,
    }
}
