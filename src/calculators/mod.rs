//! 计算页面与桌面专用绘制组件；纯数值模型统一来自 toolbox-core crate。
mod axis;
pub(crate) mod common;
mod curve_fit;
mod division_table;
mod errors;
pub(crate) mod formula_layout;
mod gauges;
mod humidity;
mod its90;
mod math_notation;
pub(crate) mod pressure;
mod radiation;
#[cfg(test)]
mod regression;
mod rtd;
pub(crate) mod spline;
pub(crate) mod sprt;
mod thermistor;
pub(crate) mod thermocouple;
mod transmitter;
mod uncertainty;

pub(crate) use toolbox_core::calculators::{
    calibration, error_process, explanations, format, math, numerics, range_method,
};
