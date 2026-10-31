//! 网页边界只负责 JSON、字段校验和结果组织；公式直接引用桌面纯模型。
#![allow(dead_code, unused_imports)]

mod api;
mod calculators;
#[path = "../../../src/catalog.rs"]
mod catalog;
mod curves;
mod engineering;
mod sensors;
mod statistics;
#[path = "../../../src/units/mod.rs"]
mod units;

use api::{Input, Output};
use catalog::{TOOLS, ToolKind};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

/// 工具、单位、分度号及说明均来自已有目录，前端不维护第二份系数。
#[wasm_bindgen]
pub fn catalog_json() -> String {
    let tools: Vec<_> = TOOLS.iter().map(|t| {
        let (kind, options, note) = match t.kind {
            ToolKind::Unit(d) => ("unit", json!(units::units_for(d).iter().map(|u| json!({"name":u.name,"code":u.code})).collect::<Vec<_>>()), units::notes::for_dimension(d)),
            _ => (kind_name(t.kind), Value::Null, calculators::explanations::for_tool(t.kind)),
        };
        let formula = calculators::explanations::formula(t.kind);
        let note_modes: Vec<_> = calculators::explanations::variants(t.kind).iter()
            .map(|(mode, note, formula)| json!({"mode": mode, "note": note, "formula": formula}))
            .collect();
        json!({"id":t.id,"name":t.name,"group":t.group.title(),"icon":t.icon,"description":t.description,"kind":kind,"units":options,"note":note,"formula":formula,"noteModes":note_modes,"rememberInputs":t.remembers_inputs()})
    }).collect();
    json!({"tools":tools,"rtd":sensors::rtd_catalog(),"zones":sensors::zone_catalog(),"lowRanges":sensors::low_catalog(),"defaults":sensors::defaults()}).to_string()
}

fn kind_name(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::WorkingThermocouple => "thermocouple",
        ToolKind::IndustrialResistance => "rtd",
        ToolKind::StandardThermocouple => "standard_couple",
        ToolKind::StandardPlatinumResistor => "sprt",
        ToolKind::Thermistor => "thermistor",
        ToolKind::PressureReference => "pressure_reference",
        ToolKind::TransmitterReference => "transmitter_reference",
        ToolKind::HeightCorrection => "height",
        ToolKind::AltitudePressure => "altitude",
        ToolKind::HumidityConversion => "humidity",
        ToolKind::EarBlackbody => "ear",
        ToolKind::BlackbodyError => "blackbody",
        ToolKind::RadiationCorrection => "radiation",
        ToolKind::Repeatability => "repeatability",
        ToolKind::RangeMethod => "range",
        ToolKind::ResultantError => "resultant",
        ToolKind::ErrorAnalysis => "error",
        ToolKind::Uncertainty => "uncertainty",
        ToolKind::Transmitter => "transmitter",
        ToolKind::BSplineInterpolation => "spline",
        ToolKind::CalibrationCurveFit => "fit",
        ToolKind::Unit(_) => "unit",
        ToolKind::Gauge(_) => "gauge",
    }
}

pub(crate) fn calculate_inner(id: &str, input: &str) -> Result<Output, String> {
    let tool = TOOLS.iter().find(|t| t.id == id).ok_or("工具不存在")?;
    let input = Input::new(input)?;
    match tool.kind {
        ToolKind::Unit(d) => {
            let units = units::units_for(d);
            let source = input.index("source", 0, units.len())?;
            let value = input.number("value", 1.0)?;
            units::convert_unit(d, value, source, source)?;
            let mut out = Output::default();
            out.headings = vec!["换算结果".into(), "单位名称".into(), "单位符号".into()];
            for (i, u) in units.iter().enumerate() {
                let result = units::convert_unit(d, value, source, i)
                    .map(|v| json!(v))
                    .unwrap_or_else(|e| json!(e));
                out.rows.push(vec![result, json!(u.name), json!(u.code)]);
            }
            out.metric("输入数值", value, units[source].code);
            Ok(out)
        }
        ToolKind::WorkingThermocouple
        | ToolKind::IndustrialResistance
        | ToolKind::StandardThermocouple
        | ToolKind::StandardPlatinumResistor
        | ToolKind::Thermistor => sensors::calculate(tool.kind, &input),
        ToolKind::Repeatability
        | ToolKind::RangeMethod
        | ToolKind::ResultantError
        | ToolKind::ErrorAnalysis
        | ToolKind::Uncertainty => statistics::calculate(tool.kind, &input),
        ToolKind::BSplineInterpolation | ToolKind::CalibrationCurveFit => {
            curves::calculate(tool.kind, &input)
        }
        _ => engineering::calculate(tool.kind, &input),
    }
}

#[wasm_bindgen]
pub fn calculate(id: &str, input: &str) -> Result<String, JsValue> {
    calculate_inner(id, input)
        .and_then(|o| serde_json::to_string(&o).map_err(|e| e.to_string()))
        .map_err(|e| JsValue::from_str(&e))
}

/// 大量样条采样按块返回，浏览器可逐块写入文件并在块间响应取消。
#[wasm_bindgen]
pub fn spline_chunk(input: &str, offset: usize, limit: usize) -> Result<String, JsValue> {
    curves::spline_csv(
        &Input::new(input).map_err(|e| JsValue::from_str(&e))?,
        offset,
        limit,
    )
    .map_err(|e| JsValue::from_str(&e))
}

/// 压力单位切换保持原量程的物理意义，由共享换算函数转换所有相关字段。
#[wasm_bindgen]
pub fn convert_pressure(input: &str, source: usize, target: usize) -> Result<String, JsValue> {
    let v: Value = serde_json::from_str(input).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let values = v
        .as_array()
        .ok_or_else(|| JsValue::from_str("量程数据格式无效"))?;
    let converted = values
        .iter()
        .map(|v| {
            let n = api::value_number(v)?;
            units::convert_unit(catalog::Dimension::Pressure, n, source, target)
                .map(|v| v.to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&converted).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// 返回量规字段、联动后的输入与报告；输入不完整时仍返回字段以支持连续编辑。
#[wasm_bindgen]
pub fn gauge_page(id: &str, input: &str, changed: &str) -> Result<String, JsValue> {
    let tool = TOOLS
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| JsValue::from_str("工具不存在"))?;
    let ToolKind::Gauge(kind) = tool.kind else {
        return Err(JsValue::from_str("不是量规工具"));
    };
    let state: Value =
        serde_json::from_str(input).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(calculators::gauges::api::page(kind, state, changed).to_string())
}
