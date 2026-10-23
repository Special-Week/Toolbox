//! 工具目录是导航、首页和页面分派的共同数据源；id 同时用于输入状态命名空间。
pub(crate) use crate::units::{convert_unit, units_for};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Group {
    Units,
    Its90,
    Pressure,
    Humidity,
    Errors,
    Gauges,
    Other,
}

impl Group {
    pub(crate) const ALL: [Group; 7] = [
        Group::Units,
        Group::Its90,
        Group::Pressure,
        Group::Humidity,
        Group::Errors,
        Group::Gauges,
        Group::Other,
    ];

    pub(crate) fn title(self) -> &'static str {
        match self {
            Group::Units => "单位换算",
            Group::Its90 => "ITS90 温标换算",
            Group::Pressure => "压力工具",
            Group::Humidity => "温湿度工具",
            Group::Errors => "误差工具",
            Group::Gauges => "量规计算",
            Group::Other => "其他工具",
        }
    }

    /// 折叠导航保留能辨认含义的短名称，不截取中文首字或 ITS90 的首字母。
    pub(crate) fn short_title(self) -> &'static str {
        match self {
            Group::Units => "单位",
            Group::Its90 => "温标",
            Group::Pressure => "压力",
            Group::Humidity => "温湿度",
            Group::Errors => "误差",
            Group::Gauges => "量规",
            Group::Other => "其他",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Dimension {
    Pressure,
    Temperature,
    MassFlow,
    VolumeFlow,
    Mass,
    Length,
    Area,
    Volume,
    Density,
    Velocity,
    Force,
    Energy,
    Power,
    HeatTransferCoefficient,
    ThermalConductivity,
    DynamicViscosity,
    KinematicViscosity,
    DiffusionCoefficient,
    SurfaceTension,
    HeatTransferRate,
    Torque,
    Concentration,
}

#[derive(Clone, Copy)]
pub(crate) enum ToolKind {
    Gauge(GaugeKind),
    Unit(Dimension),
    WorkingThermocouple,
    IndustrialResistance,
    StandardThermocouple,
    StandardPlatinumResistor,
    Thermistor,
    PressureReference,
    TransmitterReference,
    HeightCorrection,
    AltitudePressure,
    HumidityConversion,
    EarBlackbody,
    BlackbodyError,
    RadiationCorrection,
    Repeatability,
    RangeMethod,
    ResultantError,
    ErrorAnalysis,
    Uncertainty,
    Transmitter,
    BSplineInterpolation,
    CalibrationCurveFit,
}

pub(crate) struct ToolDef {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) group: Group,
    pub(crate) kind: ToolKind,
    pub(crate) icon: &'static str,
    pub(crate) description: &'static str,
}

/// 六种量规工具独立编号，各自保存输入参数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GaugeKind {
    Metric,
    Trapezoidal,
    Unified,
    Pipe,
    Rebar,
    Smooth,
}

macro_rules! tool {
    ($id:literal, $name:literal, $group:ident, $kind:expr, $icon:literal, $desc:literal) => {
        ToolDef {
            id: $id,
            name: $name,
            group: Group::$group,
            kind: $kind,
            icon: $icon,
            description: $desc,
        }
    };
}

pub(crate) const TOOLS: &[ToolDef] = &[
    tool!(
        "unit_pressure",
        "压力",
        Units,
        ToolKind::Unit(Dimension::Pressure),
        "P",
        "压力单位互换，包含帕、千帕、psi、毫米汞柱等常用单位。"
    ),
    tool!(
        "unit_temperature",
        "温度",
        Units,
        ToolKind::Unit(Dimension::Temperature),
        "℃",
        "在常见温标之间换算，正确处理温度零点偏移。"
    ),
    tool!(
        "unit_mass_flow",
        "质量流量",
        Units,
        ToolKind::Unit(Dimension::MassFlow),
        "m",
        "质量流量单位换算。"
    ),
    tool!(
        "unit_volume_flow",
        "体积流量",
        Units,
        ToolKind::Unit(Dimension::VolumeFlow),
        "V",
        "体积流量单位换算。"
    ),
    tool!(
        "unit_mass",
        "质量",
        Units,
        ToolKind::Unit(Dimension::Mass),
        "kg",
        "质量单位换算。"
    ),
    tool!(
        "unit_length",
        "长度",
        Units,
        ToolKind::Unit(Dimension::Length),
        "L",
        "长度单位换算。"
    ),
    tool!(
        "unit_area",
        "面积",
        Units,
        ToolKind::Unit(Dimension::Area),
        "A",
        "面积单位换算。"
    ),
    tool!(
        "unit_volume",
        "容积",
        Units,
        ToolKind::Unit(Dimension::Volume),
        "V",
        "容积单位换算。"
    ),
    tool!(
        "unit_density",
        "密度",
        Units,
        ToolKind::Unit(Dimension::Density),
        "ρ",
        "密度单位换算。"
    ),
    tool!(
        "unit_velocity",
        "速度",
        Units,
        ToolKind::Unit(Dimension::Velocity),
        "v",
        "速度单位换算。"
    ),
    tool!(
        "unit_force",
        "力",
        Units,
        ToolKind::Unit(Dimension::Force),
        "F",
        "力单位换算。"
    ),
    tool!(
        "unit_energy",
        "能量",
        Units,
        ToolKind::Unit(Dimension::Energy),
        "E",
        "能量单位换算。"
    ),
    tool!(
        "unit_power",
        "功率",
        Units,
        ToolKind::Unit(Dimension::Power),
        "W",
        "功率单位换算。"
    ),
    tool!(
        "unit_heat_transfer_coefficient",
        "传热系数",
        Units,
        ToolKind::Unit(Dimension::HeatTransferCoefficient),
        "h",
        "表面传热系数单位换算。"
    ),
    tool!(
        "unit_heat_conduct_coefficient",
        "导热系数",
        Units,
        ToolKind::Unit(Dimension::ThermalConductivity),
        "λ",
        "热导率单位换算。"
    ),
    tool!(
        "unit_dynamic_viscosity",
        "动力粘度",
        Units,
        ToolKind::Unit(Dimension::DynamicViscosity),
        "μ",
        "动力粘度单位换算。"
    ),
    tool!(
        "unit_kinematic_viscosity",
        "运动粘度",
        Units,
        ToolKind::Unit(Dimension::KinematicViscosity),
        "ν",
        "运动粘度单位换算。"
    ),
    tool!(
        "unit_diffusion_coefficient",
        "扩散系数",
        Units,
        ToolKind::Unit(Dimension::DiffusionCoefficient),
        "D",
        "扩散系数单位换算。"
    ),
    tool!(
        "unit_surface_tension",
        "表面张力",
        Units,
        ToolKind::Unit(Dimension::SurfaceTension),
        "γ",
        "表面张力单位换算。"
    ),
    tool!(
        "unit_heat_transfer_speed",
        "传热速度",
        Units,
        ToolKind::Unit(Dimension::HeatTransferRate),
        "q",
        "热流密度单位换算。"
    ),
    tool!(
        "unit_torque",
        "扭矩",
        Units,
        ToolKind::Unit(Dimension::Torque),
        "τ",
        "扭矩单位换算。"
    ),
    tool!(
        "unit_concentration",
        "浓度",
        Units,
        ToolKind::Unit(Dimension::Concentration),
        "c",
        "ppm、ppb、ppt、体积比、质量比和等密度基准浓度换算。"
    ),
    tool!(
        "working_thermocouple",
        "工业热电偶",
        Its90,
        ToolKind::WorkingThermocouple,
        "TC",
        "15 种分度号双向换算、参考端补偿、等级允差与分度表。"
    ),
    tool!(
        "industrial_thermal_resistance",
        "工业热电阻",
        Its90,
        ToolKind::IndustrialResistance,
        "RTD",
        "22 种分度选项，铂、铜、镍电阻双向换算与等级允差。"
    ),
    tool!(
        "standard_thermocouple",
        "标准热电偶",
        Its90,
        ToolKind::StandardThermocouple,
        "RTC",
        "S/B 型固定点校准、温区系数拟合与分度表。"
    ),
    tool!(
        "standard_platinum_resistor_sprt",
        "标准铂电阻",
        Its90,
        ToolKind::StandardPlatinumResistor,
        "SPRT",
        "SPRT 子区间偏差、银点修正、固定点拟合与分度表。"
    ),
    tool!(
        "thermistor",
        "热敏电阻",
        Its90,
        ToolKind::Thermistor,
        "NTC",
        "多项式、Steinhart–Hart、校准点拟合及 β 模型。"
    ),
    tool!(
        "pressure_reference_selection",
        "压力表标准器选型",
        Pressure,
        ToolKind::PressureReference,
        "表",
        "数字 / 活塞标准器核验、被检量程与检定范围推荐。"
    ),
    tool!(
        "pressure_transmitter_reference_selection",
        "压力变送器标准器选型",
        Pressure,
        ToolKind::TransmitterReference,
        "P/I",
        "合成压力与电测不确定度，核验并推荐标准器。"
    ),
    tool!(
        "altitude_difference_correction",
        "高度差修正",
        Pressure,
        ToolKind::HeightCorrection,
        "Δh",
        "按介质密度、重力加速度和高差计算液柱静压修正。"
    ),
    tool!(
        "altitude_pressure_converter",
        "气压高度换算",
        Pressure,
        ToolKind::AltitudePressure,
        "hPa",
        "按标准大气模型在气压和气压高度之间互算。"
    ),
    tool!(
        "temperature_humidity_conversion",
        "湿度换算",
        Humidity,
        ToolKind::HumidityConversion,
        "RH",
        "由相对湿度、露/霜点、湿球或混合比换算全部湿空气参数。"
    ),
    tool!(
        "ear_thermometer_blackbody",
        "耳温计黑体发射率修正",
        Humidity,
        ToolKind::EarBlackbody,
        "ε",
        "估算发射率小于 1 时带来的辐射温度修正。"
    ),
    tool!(
        "blackbody_radiation_source",
        "黑体辐射源发射率误差",
        Humidity,
        ToolKind::BlackbodyError,
        "εb",
        "计算黑体发射率偏离 1 时的辐射测温误差。"
    ),
    tool!(
        "radiation_thermometer",
        "辐射温度计示值修正",
        Humidity,
        ToolKind::RadiationCorrection,
        "IR",
        "按灰体辐射模型换算示值修正值。"
    ),
    tool!(
        "repeatability_calculation",
        "重复性计算",
        Errors,
        ToolKind::Repeatability,
        "σ",
        "计算均值、样本标准差、极差与相对标准偏差。"
    ),
    tool!(
        "range_method_calculation",
        "极差法计算",
        Errors,
        ToolKind::RangeMethod,
        "R",
        "按测量次数和极差系数估算重复性标准差及 A 类不确定度。"
    ),
    tool!(
        "resultant_error",
        "合成误差",
        Errors,
        ToolKind::ResultantError,
        "Σ",
        "按平方和根合成独立误差分量。"
    ),
    tool!(
        "error_analysis",
        "误差分析",
        Errors,
        ToolKind::ErrorAnalysis,
        "Δ",
        "计算绝对误差、相对误差和引用误差。"
    ),
    tool!(
        "uncertainty_calculation",
        "不确定度计算",
        Errors,
        ToolKind::Uncertainty,
        "u",
        "合成 A 类和 B 类标准不确定度并求扩展不确定度。"
    ),
    tool!(
        "gauge_metric",
        "公制普通螺纹 M",
        Gauges,
        ToolKind::Gauge(GaugeKind::Metric),
        "M",
        "内外螺纹工作量规、校对塞规、中径公差、磨损极限与长度。"
    ),
    tool!(
        "gauge_trapezoidal",
        "梯形螺纹 Tr",
        Gauges,
        ToolKind::Gauge(GaugeKind::Trapezoidal),
        "Tr",
        "30° 梯形螺纹，多线修正、工作塞规、环规和校对规。"
    ),
    tool!(
        "gauge_unified",
        "美制统一螺纹 UN",
        Gauges,
        ToolKind::Gauge(GaugeKind::Unified),
        "UN",
        "按 1A～3A、1B～3B 自动计算工件中径极限及对应量规，支持系列与 TPI 联动。"
    ),
    tool!(
        "gauge_pipe",
        "管螺纹",
        Gauges,
        ToolKind::Gauge(GaugeKind::Pipe),
        "R/NPT",
        "R、NPT 密封锥管螺纹尺寸、量规公差及生产尺寸查询。"
    ),
    tool!(
        "gauge_rebar",
        "钢筋直螺纹",
        Gauges,
        ToolKind::Gauge(GaugeKind::Rebar),
        "钢筋",
        "60°、70°、75° 钢筋直螺纹，套筒与丝头量规参考计算。"
    ),
    tool!(
        "gauge_smooth",
        "光滑量规",
        Gauges,
        ToolKind::Gauge(GaugeKind::Smooth),
        "IT",
        "孔用塞规、轴用环规与卡规，IT6～IT16 公差带查询。"
    ),
    tool!(
        "transmitter",
        "变送器",
        Other,
        ToolKind::Transmitter,
        "I/O",
        "压力、温度和湿度变送器量程换算；压力支持线性与开方输出。"
    ),
    tool!(
        "bspline_interpolation",
        "三次均匀 B 样条插值",
        Other,
        ToolKind::BSplineInterpolation,
        "B3",
        "按自然端点条件拟合 Q 型值点，支持区间内插与端点外推。"
    ),
    tool!(
        "calibration_curve_fit",
        "校准曲线拟合",
        Other,
        ToolKind::CalibrationCurveFit,
        "Fit",
        "根据成对数据拟合方程，计算系数、相关系数、拟合优度与残差。"
    ),
];
