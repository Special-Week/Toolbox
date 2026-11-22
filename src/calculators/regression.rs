use super::{
    calibration::TemperatureUnit,
    curve_fit::model::{CurveFit, FitKind},
    division_table::sample_points,
    humidity::website,
    numerics::statistics,
    pressure::model::{Range, select_standard},
    radiation::model::{Response, Spectrum},
    rtd::model::{GRADUATIONS, RtdModel},
    spline::model::{Point, UniformCubicSpline, planned_sample_count},
    thermistor::polynomial::PolynomialThermistor,
    thermocouple::{
        WORKING_KINDS, bounds, corrected_temperature, measured_emf,
        standard::{StandardCouple, zones},
    },
    transmitter::model::Transmitter,
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
            let model = &GRADUATIONS[input["graduation"].as_u64().unwrap() as usize];
            json!(RtdModel::new(model, model.r0, None)?.temperature(n("value"))?)
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
                b: numbers(&input["bCoefficients"])
            };
            if index("direction", 0) == 1 {
                let (min, max) = (n("min"), n("max"));
                let t = model.temperature_in_bounds(n("value"), min, max)?;
                if t < min || t > max {
                    return Err("温度须处于有效校准温区内".into());
                }
                json!(unit.display(t))
            } else {
                json!(model.resistance(n("value"))?)
            }
        }
        "repeatability_calculation" => json!(statistics(&numbers(&input["values"]))?.sd),
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
