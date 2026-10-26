//! 引用桌面纯模型的模块树；避免维护第二份公式、系数或单位目录。
#[path = "../../../../src/calculators/calibration.rs"]
pub(crate) mod calibration;
#[path = "../../../../src/calculators/explanations.rs"]
pub(crate) mod explanations;
#[path = "../../../../src/calculators/format.rs"]
pub(crate) mod format;
#[path = "../../../../src/calculators/math.rs"]
pub(crate) mod math;
#[path = "../../../../src/calculators/numerics.rs"]
pub(crate) mod numerics;
#[path = "../../../../src/calculators/range_method.rs"]
pub(crate) mod range_method;
#[path = "../../../../src/calculators/thermistor.rs"]
pub(crate) mod thermistor;
#[path = "../../../../src/calculators/thermocouple.rs"]
pub(crate) mod thermocouple;
pub(crate) mod common {
    pub(crate) use super::format::format_number;
}
pub(crate) mod curve_fit;
pub(crate) mod humidity;
pub(crate) mod pressure;
pub(crate) mod radiation;
pub(crate) mod rtd;
pub(crate) mod spline;
pub(crate) mod sprt;
pub(crate) mod transmitter;
pub(crate) mod uncertainty;

#[path = "../../../../src/calculators/gauges/mod.rs"]
pub(crate) mod gauges;
