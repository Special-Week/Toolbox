//! 单位换算只说明特殊定义与参考基准。
use crate::catalog::Dimension;

pub(crate) fn for_dimension(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::Temperature => "温度值换算，不适用于温差。输入不得低于绝对零度。",
        Dimension::Pressure => {
            "仅换算单位，表压或绝压基准不变。液柱按标准重力及标注温度换算（水 4 °C、汞 0 °C），不作现场密度、重力修正。"
        }
        Dimension::MassFlow => "t/a 按每年 8000 运行小时计算。",
        Dimension::VolumeFlow => {
            "gal 为美制液量加仑，ImpGal 为英制加仑；石油桶 bbl 按 42 美制加仑计算。"
        }
        Dimension::Velocity => "海里采用国际海里；UK knot 按历史英制海里 6080 ft/h 计算。",
        Dimension::Force => "kgf、tf 按标准重力加速度 9.80665 m/s² 定义。",
        Dimension::Torque => "kgf 按标准重力定义；lb、lbf 分别表示质量磅与磅力。",
        Dimension::Energy => {
            "cal、kcal、Btu 采用国际蒸汽表定义（1 cal=4.1868 J）。Hp·h 为公制马力时，UKHp·h 为英制机械马力时。"
        }
        Dimension::Power => {
            "hp 为公制马力，UKHp 为英制机械马力；cal、kcal、Btu 采用国际蒸汽表定义。"
        }
        Dimension::HeatTransferCoefficient => {
            "分母温度表示温差（1 °F 温差=5/9 K）。cal、kcal、Btu 采用国际蒸汽表定义。"
        }
        Dimension::ThermalConductivity => {
            "分母温度表示温差（1 °F 温差=5/9 K）。cal、kcal、Btu 采用国际蒸汽表定义；kcal/(m·s·°C) 中长度为米。"
        }
        Dimension::HeatTransferRate => {
            "本页“传热速度”指热流密度。cal、kcal、Btu 采用国际蒸汽表定义。"
        }
        Dimension::Volume => {
            "ImpGal 为英制加仑，gallon 为美制液量加仑；USbbl、UKbbl 分别按 42 美制、35 英制加仑计算。"
        }
        Dimension::Density => {
            "英吨按 2240 lb；英美加仑分别换算。工程密度单位 kgf·s²/m⁴ 按标准重力定义。"
        }
        Dimension::DynamicViscosity => "reyn 与 lbf·s/in² 等值；lb 表示质量磅，lbf 表示磅力。",
        Dimension::SurfaceTension => "kgf、gf 按标准重力定义；lbf 表示磅力。",
        Dimension::Concentration => {
            "ppm、ppb、ppt 分别表示 10⁻⁶、10⁻⁹、10⁻¹²；滴按 20 滴/mL。\n质量比、体积比及质量浓度按等密度基准列示，溶液与溶质密度均取 1000 kg/m³。实际物质互换须使用实际密度。"
        }
        Dimension::Mass => {
            "市担=50 kg，市斤=0.5 kg；lb 为常衡磅。英吨/英担为 2240/112 lb，美吨/美担为 2000/100 lb。"
        }
        Dimension::Length => "市尺=1/3 m；mil（密耳）=0.001 in，um 表示微米；海里采用国际海里。",
        Dimension::Area => "市亩=2000/3 m²，坪=400/121 m²；acre 按 43560 平方国际英尺计算。",
        Dimension::KinematicViscosity => "cSt 与 mm²/s 等值。",
        Dimension::DiffusionCoefficient => "",
    }
}
