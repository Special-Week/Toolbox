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
    pub(crate) fn format_process_number(value: f64) -> String {
        if value.is_finite() {
            value.to_string()
        } else {
            "—".into()
        }
    }
    pub(crate) fn format_process_computed(value: f64) -> String {
        format!("\u{2063}{}\u{2063}", format_process_number(value))
    }
    pub(crate) fn format_process_rounded(value: f64, digits: Option<usize>) -> String {
        if digits.is_some_and(|d| d > 13) {
            format_process_number(value)
        } else {
            format_process_computed(value)
        }
    }
    pub(crate) fn temperature_conversion_note(symbol: &str) -> &'static str {
        match symbol {
            "°F" => "温标换算：t(°C) = [t(°F) − 32] / 1.8；温差只除以 1.8。",
            "K" => "温标换算：t(°C) = T(K) − 273.15；开尔文温差与摄氏温差数值相同。",
            _ => "参考函数中的温度 t 使用摄氏度 °C。",
        }
    }
}
pub(crate) mod curve_fit;
#[path = "../../../../src/calculators/error_process.rs"]
pub(crate) mod error_process;
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
