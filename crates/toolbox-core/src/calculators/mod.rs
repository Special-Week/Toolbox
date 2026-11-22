//! 纯数值计算与过程生成模块，由桌面端与 Web WASM 端共同依赖。
pub mod calibration;
pub mod curve_fit;
pub mod error_process;
pub mod explanations;
pub mod format;
pub mod gauges;
pub mod humidity;
pub mod math;
pub mod numerics;
pub mod pressure;
pub mod radiation;
pub mod range_method;
pub mod rtd;
pub mod spline;
pub mod sprt;
pub mod thermistor;
pub mod thermocouple;
pub mod transmitter;
pub mod uncertainty;

pub mod common {
    pub use super::format::format_number;

    pub fn format_process_number(value: f64) -> String {
        if value.is_finite() {
            value.to_string()
        } else {
            "—".into()
        }
    }

    pub fn format_process_computed(value: f64) -> String {
        format!("\u{2063}{}\u{2063}", format_process_number(value))
    }

    pub fn format_process_rounded(value: f64, digits: Option<usize>) -> String {
        if digits.is_some_and(|d| d > 13) {
            format_process_number(value)
        } else {
            format_process_computed(value)
        }
    }

    pub fn temperature_conversion_note(symbol: &str) -> &'static str {
        match symbol {
            "°F" => "温标换算：t(°C) = [t(°F) − 32] / 1.8；温差只除以 1.8。",
            "K" => "温标换算：t(°C) = T(K) − 273.15；开尔文温差与摄氏温差数值相同。",
            _ => "参考函数中的温度 t 使用摄氏度 °C。",
        }
    }
}
