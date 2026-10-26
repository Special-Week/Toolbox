//! 压力、湿空气、辐射与变送器的网页字段适配，不另写参考曲线。
use crate::api::{Input, Output};
use crate::calculators::{humidity, math, pressure, radiation, transmitter};
use crate::catalog::{Dimension, ToolKind};
use crate::units::{convert_unit, units_for};
use serde_json::json;

pub(crate) fn calculate(kind: ToolKind, i: &Input) -> Result<Output, String> {
    let mut o = Output::default();
    match kind {
        ToolKind::HumidityConversion => {
            let u = i.unit()?;
            let known = i.index("known", 0, 4)?;
            let t = u.celsius(i.number("temperature", u.display(20.))?);
            let mut v = i.number("value", [50., u.display(10.), u.display(15.), 7.5][known])?;
            if known == 1 || known == 2 {
                v = u.celsius(v)
            }
            let r =
                humidity::website::calculate(t, i.number("pressure", 101.325)? * 1000., known, v)?;
            o.metric("相对湿度", r.rh, "%RH");
            o.optional("露点", r.dew.map(|t| u.display(t)), u.symbol());
            o.optional(
                "霜点",
                r.frost.filter(|t| *t <= 0.).map(|t| u.display(t)),
                u.symbol(),
            );
            for (label, value, unit) in [
                ("饱和水气压", r.saturated, "Pa"),
                ("水气分压", r.vapor, "Pa"),
                ("含湿量", r.volume_fraction, "L/L"),
                ("体积比（干空气基准）", r.ppm, "µL/L"),
                ("绝对湿度", r.absolute, "g/m³"),
                ("质量比", r.mass, "µg/g"),
                ("比湿", r.specific, "g/kg"),
                ("混合比", r.mixing, "g/kg"),
                ("标准状态含水量", r.normal, "g/Nm³"),
            ] {
                o.metric(label, value, unit)
            }
            o.metric("湿球温度", u.display(r.wet), u.symbol());
        }
        ToolKind::EarBlackbody => {
            let t = i.number("target", 35.)?;
            let calc = if i.index("model", 0, 2)? == 0 {
                radiation::model::ear_blackbody
            } else {
                radiation::model::ear_single_reflection
            };
            let light = calc(
                t,
                i.number("emissivity", 0.9995)?,
                i.number("probeTemperature", 18.)?,
                i.number("probeEmissivity", 0.8)?,
            )?;
            o.metric("黑体亮温", light, "°C");
            o.metric("亮温修正值（亮温−真实温度）", light - t, "°C");
        }
        ToolKind::BlackbodyError | ToolKind::RadiationCorrection => {
            let correction = matches!(kind, ToolKind::RadiationCorrection);
            let t = i.number("temperature", 1000.)?;
            let bg = i.number("ambient", 20.)?;
            let em = i.number("emissivity", 0.99)?;
            let instrument = if correction {
                i.number("instrumentEmissivity", 0.95)?
            } else {
                1.
            };
            let response = i.index("response", 0, 3)?;
            let lambda = if response == 0 {
                i.number("wavelength", 0.65)?
            } else {
                0.65
            };
            if response == 0 && !(0.1..=100.).contains(&lambda) {
                return Err("有效波长须在 0.1～100 µm 内".into());
            }
            if i.index("model", 0, 2)? == 0 {
                let w = radiation::website::wavelength(response, lambda, t);
                let delta = if correction {
                    radiation::website::correction(instrument, em, w, t, bg)?
                } else {
                    radiation::website::error(em, w, t, bg)?
                };
                let delta = (delta * 100.).round() / 100.;
                o.metric(
                    if correction {
                        "示值修正值"
                    } else {
                        "发射率误差"
                    },
                    delta,
                    "°C",
                );
                if correction {
                    o.metric("修正后温度", t + delta, "°C")
                };
                o.metric("有效波长", w, "µm");
            } else {
                use radiation::model::{Response, Spectrum};
                let spectrum = Spectrum::new(match response {
                    1 => Response::Band(8., 14.),
                    2 => Response::Band(7., 18.),
                    _ => Response::Single(lambda),
                })?;
                if correction {
                    let corrected = spectrum.corrected(t, bg, instrument, em)?;
                    o.metric("修正后温度", corrected, "°C");
                    o.metric("示值修正值", corrected - t, "°C");
                } else {
                    let light = spectrum.apparent(t, bg, em)?;
                    o.metric("表观亮温", light, "°C");
                    o.metric("示值误差（亮温−真温）", light - t, "°C");
                    o.metric("示值修正值", t - light, "°C");
                }
            }
        }
        ToolKind::HeightCorrection => {
            let height = i.number("height", 1.)?;
            let g = i.number("gravity", 9.80665)?;
            let medium = i.index("medium", 0, 2)?;
            let local_density = medium == 0 || i.index("gasState", 1, 2)? == 0;
            let delta = if local_density {
                pressure::model::hydrostatic_difference(
                    height,
                    i.number("density", if medium == 0 { 1000. } else { 1.204 })?,
                    g,
                )?
            } else {
                pressure::model::gas_difference(
                    height,
                    i.number("normalDensity", 1.293)?,
                    g,
                    i.number("temperature", 20.)?,
                    i.number("pressure", 101325.)?,
                    i.number("ambient", 101325.)?,
                    i.index("reference", 0, 2)? == 0,
                )?
            };
            o.metric("静压差 p标准−p被检", delta, "Pa");
            o.metric("移至被检高度的压力修正量", -delta, "Pa");
            o.metric("移至被检高度的压力修正量", -delta / 1000., "kPa");
            o.metric("等效水柱（4 °C）", delta / 9.806_38, "mmH₂O");
        }
        ToolKind::AltitudePressure => {
            let (h, p) = if i.index("direction", 0, 2)? == 0 {
                let h = i.number("value", 1000.)?;
                let p = math::isa_pressure(h).ok_or("高度须在 −1000～86000 m 内")?;
                (h, p)
            } else {
                let p = i.number("value", 89874.56)?;
                let h = math::isa_altitude(p).ok_or("绝对气压超出标准大气模型范围")?;
                (h, p)
            };
            o.metric("位势高度", h, "m");
            o.metric("绝对气压", p, "Pa");
            o.metric("绝对气压", p / 1000., "kPa");
            o.metric("绝对气压", p / 100., "hPa");
            let temperature = math::isa_temperature(h);
            o.metric("标准大气温度", temperature - 273.15, "°C");
            o.metric("空气密度", p / (287.052_87 * temperature), "kg/m³");
        }
        ToolKind::Transmitter => {
            use pressure::model::Range;
            let family = i.index("family", 0, 5)?;
            let symbol = if family == 0 {
                let options = units_for(Dimension::Pressure);
                options[i.index("pressureUnit", 0, options.len())?].code
            } else if family == 1 || family == 3 {
                i.unit()?.symbol()
            } else {
                ["Pa", "°C", "%RH", "°C", "g/m³"][family]
            };
            let p = Range::new(i.number("low", 0.)?, i.number("high", 100.)?)?;
            let signal = Range::new(i.number("signalLow", 4.)?, i.number("signalHigh", 20.)?)?;
            let m = transmitter::model::Transmitter::new(
                p,
                signal,
                i.index("characteristic", 0, 2)? == 1,
            );
            let inverse = i.index("direction", 0, 2)? == 1;
            let v = i.number("value", if inverse { 12. } else { 50. })?;
            let (result, f) = if inverse {
                m.inverse(v)?
            } else {
                m.forward(v)?
            };
            let s = ["mA", "V", "mV"][i.index("signalUnit", 0, 3)?];
            o.metric(
                if inverse { "过程量" } else { "输出信号" },
                result,
                if inverse { symbol } else { s },
            );
            o.metric("过程量量程比例", f * 100., "%");
        }
        ToolKind::PressureReference | ToolKind::TransmitterReference => return selection(kind, i),
        _ => return Err("工程工具不存在".into()),
    }
    Ok(o)
}

fn selection(kind: ToolKind, i: &Input) -> Result<Output, String> {
    use pressure::{model::Range, selection_model as m};
    let transmitter = matches!(kind, ToolKind::TransmitterReference);
    let units = units_for(Dimension::Pressure);
    let mpa = units
        .iter()
        .position(|u| u.code == "MPa")
        .ok_or("缺少 MPa 单位")?;
    let idx = i.index("pressureUnit", mpa, units.len())?;
    let symbol = units[idx].code;
    let dut = Range::new(i.number("low", 0.)?, i.number("high", 1.)?)?;
    let mode = i.index("mode", 0, 3)?;
    let da = i.number("accuracy", if transmitter { 0.1 } else { 0.25 })?;
    // 推荐量程时会遍历候选准确度，不能让隐藏的旧核验字段阻断计算。
    let ra = if mode == 1 {
        0.05
    } else {
        i.number("refAccuracy", if transmitter { 0.01 } else { 0.05 })?
    };
    let required = i.number("ratio", 3.)?;
    let piston = !transmitter && i.index("referenceType", 0, 2)? == 1;
    let signal = Range::new(i.number("signalLow", 4.)?, i.number("signalHigh", 20.)?)?;
    let reading = i.number("reading", 0.02)?;
    let legacy = transmitter && i.index("assessment", 0, 2)? == 1;
    let fixed = if transmitter && !legacy {
        i.number("fixedError", 0.)?
    } else {
        0.
    };
    let electrical = if legacy {
        Some(pressure::model::select_standard(
            signal,
            Range::new(
                i.number("refSignalLow", 0.)?,
                i.number("refSignalHigh", 24.)?,
            )?,
            i.number("signalAccuracy", 0.1)?,
            reading,
            required,
        )?)
    } else {
        None
    };
    let mut o = Output::default();
    if mode == 0 {
        let reference = Range::new(i.number("refLow", 0.)?, i.number("refHigh", 1.6)?)?;
        let p = m::verify(dut, reference, da, ra, required, piston)?;
        o.metric("被检压力允差", p.dut_tolerance, symbol);
        o.metric("标准器压力允差", p.reference_tolerance, symbol);
        o.metric("量传比", p.ratio, "");
        o.metric("量程覆盖", if p.coverage { "符合" } else { "不符合" }, "");
        let meets = if transmitter {
            if let Some(es) = &electrical {
                o.metric("被检输出允差", es.dut_tolerance, "mA");
                o.metric("标准电测允差", es.reference_tolerance, "mA");
                o.metric("电测通道量传比", es.ratio, "");
                o.metric(
                    "电测量程覆盖",
                    if es.coverage { "符合" } else { "不符合" },
                    "",
                );
                p.meets && es.meets
            } else {
                let s = m::transmitter(dut, reference, signal, da, ra, reading, fixed, required)?;
                o.metric("合成扩展不确定度 (k=2)", s.expanded, "mA");
                o.metric("电信号允许限值", s.allowance, "mA");
                s.meets
            }
        } else {
            p.meets
        };
        o.metric("综合判断", if meets { "符合" } else { "不符合" }, "");
        return Ok(o);
    }
    let ranges = if i.index("series", 0, 2)? == 0 {
        m::METRIC_RANGES
    } else {
        pressure::ranges::EXTENDED_RANGES
    };
    let ranges = ranges
        .iter()
        .map(|&(l, h)| {
            Range::new(
                convert_unit(Dimension::Pressure, l, mpa, idx)?,
                convert_unit(Dimension::Pressure, h, mpa, idx)?,
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let duties = if mode == 1 {
        vec![dut]
    } else {
        ranges.iter().copied().filter(|r| dut.covers(*r)).collect()
    };
    o.headings = vec![
        format!("被检下限 ({symbol})"),
        format!("被检上限 ({symbol})"),
        format!("标准器下限 ({symbol})"),
        format!("标准器上限 ({symbol})"),
        "准确度 (%)".into(),
    ];
    for duty in duties {
        let accuracies = if mode == 2 {
            vec![ra]
        } else if piston {
            vec![0.003, 0.005, 0.01, 0.05]
        } else {
            vec![0.01, 0.02, 0.05]
        };
        let mut matches = vec![];
        for range in &ranges {
            for &accuracy in &accuracies {
                let meets = if let Some(es) = &electrical {
                    m::verify(duty, *range, da, accuracy, required, false)?.meets && es.meets
                } else if transmitter {
                    m::transmitter(duty, *range, signal, da, accuracy, reading, fixed, required)?
                        .meets
                } else {
                    m::verify(duty, *range, da, accuracy, required, piston)?.meets
                };
                if meets {
                    matches.push((*range, accuracy))
                }
            }
        }
        matches.sort_by(|a, b| {
            b.1.total_cmp(&a.1)
                .then_with(|| a.0.span.total_cmp(&b.0.span))
        });
        for (r, a) in matches {
            o.rows.push(vec![
                json!(duty.low),
                json!(duty.high),
                json!(r.low),
                json!(r.high),
                json!(a),
            ]);
        }
    }
    o.metric("满足条件的组合", o.rows.len(), "组");
    if o.rows.is_empty() {
        o.warnings.push("内置目录没有满足条件的标准器组合。".into())
    }
    Ok(o)
}
