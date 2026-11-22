use super::{
    calibration::TemperatureUnit,
    curve_fit::model::{CurveFit, FitKind},
    division_table::sample_points,
    humidity::website,
    math::isa_kinetic_temperature,
    numerics::statistics,
    pressure::model::{Range, select_standard},
    pressure::selection_model,
    radiation::model::{Response, Spectrum},
    rtd::model::{GRADUATIONS, RtdModel},
    spline::model::{Point, UniformCubicSpline, planned_sample_count},
    sprt::model::{Sprt, high_for_resistance, high_for_temperature},
    thermistor::polynomial::PolynomialThermistor,
    thermocouple::{
        WORKING_KINDS, bounds, corrected_temperature, measured_emf,
        standard::{StandardCouple, zones},
    },
    transmitter::model::Transmitter,
    uncertainty::model as unc,
};
use serde_json::{Value, json};

fn number(value: &Value) -> f64 {
    value.as_f64().expect("fixture number")
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("fixture array")
        .iter()
        .map(number)
        .collect()
}

fn evaluate(case: &Value) -> Result<Value, String> {
    let input = &case["input"];
    let n = |key: &str| number(&input[key]);
    let or = |key: &str, default: f64| input.get(key).map_or(default, number);
    let index = |key: &str, default: usize| {
        input
            .get(key)
            .map_or(default, |value| value.as_u64().unwrap() as usize)
    };
    let unit = TemperatureUnit(index("unit", 0));
    Ok(match case["tool"].as_str().unwrap() {
        "pressure_reference_selection" => {
            let result = select_standard(
                Range::new(or("low", 0.0), n("high"))?,
                Range::new(or("refLow", 0.0), n("refHigh"))?,
                n("accuracy"),
                n("refAccuracy"),
                n("ratio"),
            )?;
            json!(if result.meets { "符合" } else { "不符合" })
        }
        "pressure_transmitter_reference_selection" => {
            let result = selection_model::transmitter(
                Range::new(n("low"), n("high"))?, Range::new(n("refLow"), n("refHigh"))?,
                Range::new(n("signalLow"), n("signalHigh"))?, n("accuracy"), n("refAccuracy"),
                n("reading"), or("fixedError", 0.0), n("ratio"),
            )?;
            json!(if case["metric"] == "电信号允许限值" { result.allowance } else { result.expanded })
        }
        "standard_thermocouple" => {
            let kind = ["S", "B"][index("type", 0)];
            let mut model = StandardCouple {
                kind,
                points: Vec::new(),
                zone: None,
            };
            if input["source"] == 1 {
                let mut zone = zones(kind).remove(index("zone", 0));
                if let Some(coefficients) = input.get("coefficients") {
                    zone.2 = numbers(coefficients);
                }
                model.zone = Some(zone);
                let bounds = model.bounds();
                let cold = unit.celsius_in_bounds(or("cold", unit.display(0.0)), bounds);
                json!(model.emf(unit.celsius_in_bounds(n("value"), bounds))? - model.reference_end_emf(cold)?)
            } else {
                let temperatures: &[f64] = if kind == "S" {
                    &[419.527, 660.323, 1084.62]
                } else {
                    &[1100., 1200., 1300., 1400., 1500.]
                };
                let measured = numbers(&input["fixed"]);
                assert_eq!(temperatures.len(), measured.len());
                model.points = temperatures.iter().copied().zip(measured).collect();
                if input["direction"] == 1 {
                    json!(unit.display(model.temperature(n("value"))?))
                } else {
                    let t = unit.celsius_in_bounds(n("value"), model.bounds());
                    json!(model.slope(t)?.map(|(_, _, slope)| slope / unit.span()))
                }
            }
        }
        "industrial_thermal_resistance" => {
            let def = &GRADUATIONS[index("graduation", 0)];
            let model = RtdModel::new(def, or("r0", def.r0), None)?;
            if index("direction", 0) == 1 {
                json!(unit.display(model.temperature(n("value"))?))
            } else {
                let t = unit.celsius_in_bounds(n("value"), (def.min, def.max));
                json!(model.resistance(t)?)
            }
        }
        "temperature_humidity_conversion" => {
            let known = index("known", 0);
            let value = n("value");
            let value = if matches!(known, 1 | 2) {
                unit.celsius(value)
            } else {
                value
            };
            let result = website::calculate(
                unit.celsius(n("temperature")),
                or("pressure", 101.325) * 1000.,
                known,
                value,
            )?;
            match case["metric"].as_str().unwrap() {
                "露点" => json!(result.dew.map(|t| unit.display(t))),
                "相对湿度" => json!(result.rh),
                "湿球温度" => json!(result.wet.map(|t| unit.display(t))),
                "水气分压" => json!(result.vapor),
                metric => panic!("unknown humidity metric {metric}"),
            }
        }
        "altitude_pressure_converter" => json!(isa_kinetic_temperature(n("value")) - 273.15),
        "transmitter" => {
            let model = Transmitter::new(
                Range::new(n("low"), n("high"))?,
                Range::new(n("signalLow"), n("signalHigh"))?,
                index("characteristic", 0) == 1,
            );
            let (result, _) = if index("direction", 0) == 1 {
                model.inverse(n("value"))?
            } else {
                model.forward(n("value"))?
            };
            json!(result)
        }
        "calibration_curve_fit" => {
            let points: Vec<_> = input["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| (number(&p[0]), number(&p[1])))
                .collect();
            json!(
                CurveFit::calculate(&points, FitKind::from_index(n("model") as usize))?
                    .coefficients[1]
            )
        }
        "bspline_interpolation" => {
            let points: Vec<_> = input["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Point {
                    x: number(&p[0]),
                    y: number(&p[1]),
                })
                .collect();
            UniformCubicSpline::interpolate(&points)?;
            json!(planned_sample_count(n("start"), n("end"), n("step"))?)
        }
        "blackbody_radiation_source" => {
            json!(Spectrum::new(Response::Single(n("wavelength")))?.apparent(
                n("temperature"),
                n("ambient"),
                n("emissivity")
            )?)
        }
        "radiation_thermometer" => {
            let response = match index("response", 0) {
                1 => Response::Band(8.0, 14.0),
                2 => Response::Band(7.0, 18.0),
                _ => Response::Single(n("wavelength")),
            };
            json!(Spectrum::new(response)?.corrected(
                n("temperature"), n("ambient"), n("instrumentEmissivity"), n("emissivity"),
            )?)
        }
        "working_thermocouple" => {
            let kind = WORKING_KINDS[index("type", 3)];
            let bounds = bounds(kind)?;
            let cold = unit.celsius_in_bounds(or("cold", unit.display(0.0)), bounds);
            let correction = or("correction", 0.0);
            let inverse = index("direction", 0) == 1;
            let value = or(
                "value",
                if inverse {
                    4.096230218723254
                } else {
                    unit.display(100.0)
                },
            );
            let signal = if inverse {
                unit.display(corrected_temperature(kind, value, cold, correction)?.0)
            } else {
                measured_emf(
                    kind,
                    unit.celsius_in_bounds(value, bounds),
                    cold,
                    correction,
                )?
            };
            if input["division"] == true {
                let (start, end) = (n("tableStart"), n("tableEnd"));
                if start < unit.display(bounds.0) || end > unit.display(bounds.1) {
                    return Err("分度表超出所选温区".into());
                }
                json!(sample_points(start, end, n("tableStep"))?.len())
            } else {
                json!(signal)
            }
        }
        "thermistor" => {
            let model = PolynomialThermistor {
                a: input.get("aCoefficients").map_or_else(Vec::new, numbers),
                b: numbers(&input["bCoefficients"]),
            };
            let (min, max) = (or("min", -20.0), or("max", 100.0));
            let (t, r) = if index("direction", 0) == 1 {
                let t = model.temperature_in_bounds(n("value"), min, max)?;
                if t < min || t > max {
                    return Err("温度须处于有效校准温区内".into());
                }
                (t, n("value"))
            } else {
                let t = unit.celsius_in_bounds(n("value"), (min, max));
                if t < min || t > max {
                    return Err("温度须处于有效校准温区内".into());
                }
                (t, model.resistance(t)?)
            };
            match case["metric"].as_str().unwrap_or("") {
                "电阻变化率" => json!(model.slope(t)? / unit.span()),
                "温度" => json!(unit.display(t)),
                _ => json!(r),
            }
        }
        "standard_platinum_resistor_sprt" => {
            let high_range = index("highRange", 1);
            let low_range = index("lowRange", 0);
            let high_enabled = high_range != 6;
            let low_enabled = low_range != 2;
            let model = Sprt {
                rtp: or("rtp", 25.12461),
                a: if high_enabled { or("a", -0.000272072) } else { 0.0 },
                b: if high_enabled && high_range <= 3 { or("b", 0.0000178924) } else { 0.0 },
                c: if high_enabled && high_range <= 1 { or("c", -0.0000112269) } else { 0.0 },
                a4: if low_enabled { or("a4", 0.0) } else { 0.0 },
                b4: if low_enabled { or("b4", 0.0) } else { 0.0 },
                low_quadratic: low_range == 1,
                d: if high_range == 0 { or("d", 0.0) } else { 0.0 },
                w660: or("w660", 3.376008599409339),
            };
            if high_range == 0 && model.w660 <= 1.0 {
                return Err("W660 须大于 1".into());
            }
            if !high_enabled && !low_enabled {
                return Err("至少启用一个温区".into());
            }
            if index("direction", 0) == 1 {
                let r = n("value");
                let high = high_for_resistance(r, model.rtp, high_enabled, low_enabled);
                json!(unit.display(model.temperature_branch(r, high)?))
            } else {
                let t = unit.celsius(n("value"));
                let high = high_for_temperature(t, high_enabled, low_enabled);
                json!(model.resistance_branch(t, high)?)
            }
        }
        "uncertainty_calculation" => {
            let nodes = input["components"].as_array().unwrap();
            let mut contributions = Vec::new();
            let mut dof_terms = Vec::new();
            for node in nodes {
                let kind = node.get("kind").and_then(Value::as_u64).unwrap_or(1);
                let sensitivity = node.get("sensitivity").and_then(Value::as_f64).unwrap_or(1.0);
                let item = unc::Component {
                    name: "分量".into(),
                    sensitivity,
                    rounding: None,
                    kind: if kind == 0 {
                        unc::Kind::TypeA {
                            values: numbers(&node["values"]),
                            range_method: node.get("method").and_then(Value::as_u64).unwrap_or(0) == 1,
                            average_count: None,
                        }
                    } else {
                        let dof = match node.get("dof") {
                            None | Some(Value::Null) => None,
                            Some(Value::Number(v)) => unc::parse_dof(&v.to_string())?,
                            Some(Value::String(s)) => unc::parse_dof(s)?,
                            _ => return Err("自由度须为大于 0 的数".into()),
                        };
                        unc::Kind::TypeB {
                            half_width: number(&node["halfWidth"]),
                            distribution: node.get("distribution").and_then(Value::as_u64).unwrap_or(0) as usize,
                            dof,
                        }
                    },
                };
                let d = item.calculate()?;
                contributions.push(d.contribution);
                dof_terms.push((d.contribution, d.dof));
            }
            let maximum = index("combine", 0) == 1;
            let uc = unc::combine(&contributions, maximum)?;
            let eff = unc::combine_dof(&dof_terms, maximum)?;
            let k = unc::coverage_factor(index("coverageMode", 0), or("coverage", 2.0), eff)?;
            match case["metric"].as_str().unwrap_or("") {
                "有效自由度 ν_eff" => match eff {
                    unc::DegreesOfFreedom::Finite(v) => json!(v),
                    unc::DegreesOfFreedom::Infinite => json!("∞"),
                    unc::DegreesOfFreedom::NotApplicable => json!("不适用"),
                },
                "包含因子 k" => json!(k),
                "修约后 U" => json!(unc::rounded_combined(
                    uc, k, index("digits", 1) + 1, index("roundingV2", 1) == 1,
                )?.1),
                _ => json!(uc),
            }
        }
        "repeatability_calculation" => {
            let stats = statistics(&numbers(&input["values"]))?;
            json!(if case["metric"] == "平均值" { stats.mean } else { stats.sd })
        }
        tool => panic!("unknown fixture tool {tool}"),
    })
}

#[test]
fn shared_boundary_cases() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/boundaries.json")).unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let id = case["id"].as_str().unwrap();
        assert_ne!(
            case.get("error").is_some(),
            case.get("expected").is_some(),
            "{id}"
        );
        let result = evaluate(&case);
        if let Some(expected) = case["error"].as_str() {
            assert!(!expected.is_empty(), "{id}");
            assert!(
                matches!(&result, Err(error) if error.contains(expected)),
                "{id}: {result:?}"
            );
        } else {
            let actual = result.unwrap_or_else(|error| panic!("{id}: {error}"));
            let expected = case
                .get("expected")
                .unwrap_or_else(|| panic!("{id}: missing expected value"));
            if let (Some(a), Some(b)) = (actual.as_f64(), expected.as_f64()) {
                assert!(
                    (a - b).abs() <= case["tolerance"].as_f64().unwrap_or(0.),
                    "{id}: {a} != {b}"
                );
            } else {
                assert_eq!(&actual, expected, "{id}");
            }
        }
    }
}
