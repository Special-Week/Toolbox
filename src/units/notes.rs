//! 单位类别的计算说明，记录温差、工业年、英美定义及浓度基准等非显然约定。
use crate::catalog::Dimension;

pub(crate) fn for_dimension(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::Temperature => {
            "本页换算绝对温度值：K = °C + 273.15，°F = 1.8×°C + 32，°R = 1.8×K，°Re = 0.8×°C（列氏温标）；例如 0 °C = 273.15 K = 32 °F。输入不能低于 0 K；温差换算不能套用温度值的零点偏移。"
        }
        Dimension::Pressure => {
            "以 Pa 为基准换算。液柱按标注温度及标准重力计算，水柱为 4 °C、汞柱为 0 °C；1 in = 25.4 mm，1 torr = 1 atm/760。换算保留输入压力的参考基准，只改变数值单位；液柱结果使用约定系数，未按现场介质密度或当地重力调整。"
        }
        Dimension::MassFlow => {
            "按质量与时间倍率换算，1 t = 1000 kg，1 d = 24 h，1 lb = 0.45359237 kg。t/a 按每年 8000 运行小时计算：1 kg/s = 28800 t/a。"
        }
        Dimension::VolumeFlow => {
            "按体积与时间倍率换算。CFM、CFH 分别为立方英尺/分钟和立方英尺/小时；gal/s 为美制液量加仑/秒，ImpGal/s 为英制加仑/秒；bbl/s 按 42 美制加仑的石油桶计算。"
        }
        Dimension::Velocity => {
            "各速度单位按长度与时间倍率换算。1 mile = 1609.344 m；国际海里为 1852 m，knots 与 n mile/h 等值。UK knot 按历史英制海里 6080 ft/h 换算。"
        }
        Dimension::Force => {
            "目标数值 = 输入数值 × 输入单位的 SI 倍率 / 目标单位的 SI 倍率。kgf 与 tf 按标准重力加速度 9.80665 m/s² 定义，1 kgf = 9.80665 N，1 tf = 1000 kgf。"
        }
        Dimension::Torque => {
            "力矩按力与力臂长度倍率换算，以 N·m 为基准。kgf 按标准重力加速度 9.80665 m/s² 定义；lbf·ft 与 lbf·in 分别计入英尺和英寸的长度倍率。1 lbf·ft = 12 lbf·in，1 lbf·in = 16 ozf·in。"
        }
        Dimension::Energy => {
            "热量采用国际蒸汽表定义，1 cal = 4.1868 J，1 kcal = 4186.8 J，1 Btu = 1055.05585262 J。Hp·h 采用公制马力，UKHp·h 采用英制机械马力；1 kWh = 3600000 J。"
        }
        Dimension::Power => {
            "功率按能量/时间换算，1 W = 1 J/s。hp 表示公制马力（735.49875 W），UKHp 表示英制机械马力（550 ft·lbf/s）。cal、kcal 与 Btu 均采用国际蒸汽表定义，1 kcal/h = 4186.8/3600 W。"
        }
        Dimension::HeatTransferCoefficient => {
            "按能量、时间、面积及温差的倍率换算。分母中的温度单位表示温差，1 °C 温差 = 1 K，1 °F 温差 = 5/9 K；cal、kcal、Btu 统一采用国际蒸汽表定义（1 cal = 4.1868 J）。"
        }
        Dimension::ThermalConductivity => {
            "按能量、时间、长度及温差的倍率换算，1 °C 温差 = 1 K，1 °F 温差 = 5/9 K；cal、kcal、Btu 统一采用国际蒸汽表定义（1 cal = 4.1868 J）。kcal/(m·s·°C) 按符号中的米计算，1 kcal/(m·s·°C) = 4186.8 W/(m·K)。"
        }
        Dimension::HeatTransferRate => {
            "本页“传热速度”指热流密度，按功率/面积换算。1 W/cm² = 10 kW/m²。cal、kcal 与 Btu 采用国际蒸汽表定义（1 cal = 4.1868 J，1 Btu = 1055.05585262 J）；面积及时间倍率同时计入。"
        }
        Dimension::Volume => {
            "按体积单位倍率换算，1 L = 0.001 m³，1 kL = 1 m³。ImpGal 为英制加仑（4.54609 L），gallon 为美制液量加仑（3.785411784 L）。USbbl 按 42 美制加仑，UKbbl 按 35 英制加仑计算。"
        }
        Dimension::Density => {
            "按质量/体积的单位倍率换算，1 g/cm³ = 1000 kg/m³，1 kg/L = 1000 kg/m³。英吨取 2240 lb，英制与美制加仑分别计算。工程密度单位为 kgf·s²/m⁴，1 kgf·s²/m⁴ = 9.80665 kg/m³。"
        }
        Dimension::DynamicViscosity => {
            "动力粘度以 Pa·s 为基准，1 P = 0.1 Pa·s，1 cP = 1 mPa·s。dyn·s/cm² 与 P 等值；reyn 与 lbf·s/in² 等值。力、质量、面积和小时/秒的倍率分别计入，lb 为质量，lbf 为力。"
        }
        Dimension::SurfaceTension => {
            "表面张力按力/长度换算，以 N/m 为基准。kgf/m、gf/cm、lbf/ft、lbf/inch 中的分子均表示力；1 kgf/m = 9.80665 N/m，1 dyn/cm = 1 mN/m。"
        }
        Dimension::Concentration => {
            "ppm、ppb、ppt 分别表示百万、十亿、万亿分之一；1 ppm = 1000 ppb = 1000000 ppt。滴的约定为 20 滴/mL。\n质量比与体积比按等密度基准列示；slug/体积按溶液及溶质密度均为 1000 kg/m³ 折算，因此 1 ppm 对应 1 mg/kg、1 mL/m³、0.001 kg/m³。真实物质的质量比、体积比、质量浓度互换需要实际密度，不能直接套用此基准。"
        }
        Dimension::Mass => {
            "公制质量以 kg 为基准。1 市担 = 50 kg，1 市斤 = 0.5 kg；lb 为常衡磅（0.45359237 kg），1 oz = 1/16 lb，1 dr = 1/256 lb，1 gr = 1/7000 lb。英吨、英担分别为 2240 lb、112 lb，美吨、美担分别为 2000 lb、100 lb。"
        }
        Dimension::Length => {
            "按长度倍率换算：1 in = 0.0254 m，1 ft = 0.3048 m，1 yd = 0.9144 m，1 mile = 1609.344 m，1 n mile = 1852 m。1 市尺 = 1/3 m；1 mil（密耳）= 0.001 in = 0.0254 mm；um 表示微米。"
        }
        Dimension::Area => {
            "面积倍率按长度倍率的平方计算：1 ft² = 0.09290304 m²。1 ha = 10000 m²，1 市亩 = 2000/3 m²，1 坪 = 400/121 m²。本页 acre 按 43560 平方国际英尺计算，等于 4046.8564224 m²。"
        }
        Dimension::KinematicViscosity => {
            "运动粘度量纲为面积/时间，按长度倍率的平方及时间倍率换算：1 St = 1 cm²/s = 0.0001 m²/s，1 cSt = 1 mm²/s = 0.000001 m²/s。1 m²/h = 1/3600 m²/s；平方英尺、平方英寸分别计入长度倍率的平方。"
        }
        Dimension::DiffusionCoefficient => {
            "扩散系数量纲为面积/时间，按长度倍率的平方及时间倍率换算：1 cm²/s = 0.0001 m²/s，1 m²/h = 1/3600 m²/s，1 in²/s = 0.00064516 m²/s。1 ft = 0.3048 m，平方英尺单位按该长度的平方计算。"
        }
    }
}
