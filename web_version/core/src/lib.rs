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

#[cfg(test)]
mod explanation_catalog_tests {
    use super::*;

    #[test]
    fn all_explanations_and_variants_reach_the_web_catalog() {
        let catalog: Value = serde_json::from_str(&catalog_json()).unwrap();
        let tools = catalog["tools"].as_array().unwrap();
        assert_eq!(tools.len(), TOOLS.len());
        for tool in TOOLS {
            let exported = tools.iter().find(|item| item["id"] == tool.id).unwrap();
            assert_eq!(
                exported["note"],
                calculators::explanations::for_tool(tool.kind)
            );
            assert_eq!(
                exported["formula"],
                calculators::explanations::formula(tool.kind)
            );
            let variants = exported["noteModes"].as_array().unwrap();
            assert_eq!(
                variants.len(),
                calculators::explanations::variants(tool.kind).len()
            );
            for &(mode, note, formula) in calculators::explanations::variants(tool.kind) {
                let exported = variants.iter().find(|item| item["mode"] == mode).unwrap();
                assert_eq!(exported["note"], note);
                assert_eq!(exported["formula"], formula);
            }
        }
    }

    #[test]
    fn every_calculator_returns_a_process_for_default_input() {
        let mut checked = 0;
        for tool in TOOLS {
            match tool.kind {
                ToolKind::PdfDownload => continue,
                ToolKind::Gauge(kind) => {
                    let page = calculators::gauges::api::page(kind, json!({}), "");
                    assert!(
                        page["error"].as_str().unwrap_or("").is_empty(),
                        "{}: {}",
                        tool.id,
                        page["error"]
                    );
                    assert!(
                        !page["report"]["process"].as_array().unwrap().is_empty(),
                        "{}",
                        tool.id
                    );
                }
                _ => {
                    let out = calculate_inner(tool.id, "{}")
                        .unwrap_or_else(|e| panic!("{}: {e}", tool.id));
                    assert!(!out.process.is_empty(), "{} 缺少计算过程", tool.id);
                }
            }
            checked += 1;
        }
        assert_eq!(checked, 49);
    }

    #[test]
    fn process_follows_selected_models_inputs_and_boundaries() {
        let process = |id: &str, input: Value| {
            calculate_inner(id, &input.to_string())
                .unwrap_or_else(|e| panic!("{id}: {e}"))
                .process
                .join("\n")
                .replace('\u{2063}', "")
        };
        for (known, value, expected) in [
            (0, 50., "输入相对湿度"),
            (1, 10., "输入露点"),
            (2, 15., "输入湿球"),
            (3, 7.5, "输入混合比"),
        ] {
            let text = process(
                "temperature_humidity_conversion",
                json!({"known":known,"value":value}),
            );
            assert!(text.contains(expected), "{known}: {text}");
            assert!(text.contains("水汽分压"));
        }
        for model in 0..2 {
            let text = process(
                "blackbody_radiation_source",
                json!({"model":model,"temperature":100.,"response":1}),
            );
            assert!(text.contains(if model == 0 { "有效波长" } else { "Planck" }));
        }
        for direction in 0..2 {
            let value = if direction == 0 {
                100.
            } else {
                4.096230218723254
            };
            let text = process(
                "working_thermocouple",
                json!({"type":3,"direction":direction,"value":value}),
            );
            assert!(text.contains("c[0]"));
            assert!(text.contains("指数项 a₀"));
        }
        let text = process("bspline_interpolation", json!({}));
        assert!(text.contains("sᵧ"));
        assert!(text.contains("归一化"));
        assert_eq!(text.matches("X(u) =").count(), 1);
        let text = process("error_analysis", json!({"value":100.02,"reference":100.}));
        assert!(text.contains("绝对误差 Δ"));
        let text = process(
            "uncertainty_calculation",
            json!({"components":[{"kind":0,"values":[10.02,10.01,10.03],"sensitivity":1}]}),
        );
        assert!(text.contains("测量点 1："));
        assert!(!text.contains("测量点 1：\n\n"));
        assert!(
            calculate_inner(
                "temperature_humidity_conversion",
                &json!({"known":0,"value":101}).to_string()
            )
            .is_err()
        );
        for (kind, state) in [
            (catalog::GaugeKind::Metric, json!({"side":"塞规（内螺纹）"})),
            (catalog::GaugeKind::Metric, json!({"side":"环规（外螺纹）"})),
            (
                catalog::GaugeKind::Unified,
                json!({"side":"环规（外螺纹）"}),
            ),
        ] {
            let page = calculators::gauges::api::page(kind, state, "");
            assert!(page["error"].as_str().unwrap_or("").is_empty(), "{page}");
            let text = page["report"]["process"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("中径"));
            assert!(text.contains("工作"));
        }
    }
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
        ToolKind::PdfDownload => "pdf",
        ToolKind::Unit(_) => "unit",
        ToolKind::Gauge(_) => "gauge",
    }
}

pub(crate) fn calculate_inner(id: &str, input: &str) -> Result<Output, String> {
    let tool = TOOLS.iter().find(|t| t.id == id).ok_or("工具不存在")?;
    let input = Input::new(input)?;
    match tool.kind {
        ToolKind::PdfDownload => Err("此工具通过服务器缓存获取 PDF，无需数值计算。".into()),
        ToolKind::Unit(d) => {
            let units = units::units_for(d);
            let source = input.index("source", 0, units.len())?;
            let value = input.number("value", 1.0)?;
            units::convert_unit(d, value, source, source)?;
            let mut out = Output::default();
            out.headings = vec!["换算结果".into(), "单位名称".into(), "单位符号".into()];
            for (i, u) in units.iter().enumerate() {
                let converted = units::convert_unit(d, value, source, i);
                if i > 0 {
                    out.process.push(String::new());
                }
                match converted {
                    Ok(v) => {
                        let c = calculators::common::format_process_computed;
                        let (sf, so) = units[source].conversion_coefficients();
                        let (tf, to) = u.conversion_coefficients();
                        let formula = if i == source {
                            format!("同单位直接返回输入 = {}", value)
                        } else if matches!(d, catalog::Dimension::Temperature) {
                            let kelvin = value.mul_add(sf, so).max(0.);
                            format!(
                                "K = max({value} × {sf} + {so}, 0) = {}；目标 = (K − {to}) / {tf} = {}",
                                c(kelvin),
                                c(v)
                            )
                        } else {
                            format!(
                                "目标 = 输入 × (源系数/目标系数) = {value} × ({sf}/{tf}) = {}",
                                c(v)
                            )
                        };
                        out.process.push(format!(
                            "{} → {}：{} {}。",
                            units[source].code, u.code, formula, u.code
                        ));
                    }
                    Err(ref e) => out
                        .process
                        .push(format!("{} → {}：{}。", units[source].code, u.code, e)),
                }
                let result = converted.map(|v| json!(v)).unwrap_or_else(|e| json!(e));
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

#[wasm_bindgen]
pub fn curve_value(id: &str, input: &str, x: f64) -> Result<f64, JsValue> {
    curves::curve_value(
        id,
        &Input::new(input).map_err(|e| JsValue::from_str(&e))?,
        x,
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
