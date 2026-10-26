//! 温标页面适配；正反算、允差与分度表共用桌面参考曲线。
use crate::api::{Input, Output, celsius_in_bounds, division};
use crate::calculators::{calibration::TemperatureUnit, rtd, sprt, thermistor, thermocouple as tc};
use crate::catalog::ToolKind;
use serde_json::{Value, json};

pub(crate) fn rtd_catalog() -> Value {
    json!(
        rtd::model::GRADUATIONS
            .iter()
            .map(|g| json!({"name":g.name,"key":g.key,"r0":g.r0,"min":g.min,"max":g.max}))
            .collect::<Vec<_>>()
    )
}
pub(crate) fn zone_catalog() -> Value {
    json!(["S","B"].iter().map(|k|tc::standard::zones(k).iter().map(|(a,b,c)|json!({"min":a,"max":b,"coefficients":c,"points":(1..=3).map(|j|{let t=a+(b-a)*j as f64/4.;vec![t.to_string(),crate::calculators::calibration::polynomial(c,t).to_string()]}).collect::<Vec<_>>()})).collect::<Vec<_>>()).collect::<Vec<_>>())
}
pub(crate) fn low_catalog() -> Value {
    json!(
        sprt::ranges::LOW_RANGES
            .iter()
            .map(|r| json!({"label":r.label,"min":r.minimum,"fixed":r.fixed_points}))
            .collect::<Vec<_>>()
    )
}

pub(crate) fn defaults() -> Value {
    let beta = thermistor::BetaThermistor::new(10000., 25., 3950.).expect("内置 β 示例有效");
    let points = (0..7)
        .map(|j| {
            let t = -20. + j as f64 * 20.;
            json!([
                t.to_string(),
                beta.resistance(t).expect("内置示例温度有效").to_string()
            ])
        })
        .collect::<Vec<_>>();
    let model = sprt::model::Sprt::default();
    let mut fixed = serde_json::Map::new();
    for t in [
        -189.344, -38.834, 29.765, 156.599, 231.928, 419.527, 660.323, 961.78,
    ] {
        fixed.insert(
            t.to_string(),
            json!(
                model
                    .resistance(t)
                    .expect("内置 SPRT 固定点有效")
                    .to_string()
            ),
        );
    }
    json!({"ntcA":[(1./298.15-10000.0_f64.ln()/3950.).to_string(),(1./3950.).to_string()],"ntcB":[(10000.0_f64.ln()-3950./298.15).to_string(),"3950"],"ntcPoints":points,"sprtFixed":fixed})
}

pub(crate) fn calculate(kind: ToolKind, i: &Input) -> Result<Output, String> {
    match kind {
        ToolKind::WorkingThermocouple => thermocouple(i),
        ToolKind::IndustrialResistance => industrial(i),
        ToolKind::StandardThermocouple => standard(i),
        ToolKind::StandardPlatinumResistor => platinum(i),
        ToolKind::Thermistor => ntc(i),
        _ => Err("温标工具不存在".into()),
    }
}

fn thermocouple(i: &Input) -> Result<Output, String> {
    let kinds = [
        "B", "E", "J", "K", "N", "R", "S", "T", "C", "D", "G", "L", "LR", "U", "A",
    ];
    let kind = kinds[i.index("type", 3, kinds.len())?];
    let unit = i.unit()?;
    let direction = i.index("direction", 0, 2)?;
    let cold = celsius_in_bounds(
        unit,
        i.number("cold", unit.display(0.0))?,
        tc::bounds(kind)?,
    );
    let correction = i.number("correction", 0.0)?;
    let input = i.number(
        "value",
        if direction == 0 {
            unit.display(100.0)
        } else {
            4.096230218723254
        },
    )?;
    let (t, measured) = if direction == 0 {
        let t = celsius_in_bounds(unit, input, tc::bounds(kind)?);
        (t, tc::measured_emf(kind, t, cold, correction)?)
    } else {
        (
            tc::corrected_temperature(kind, input, cold, correction)?.0,
            input,
        )
    };
    let mut out = Output::default();
    out.metric("热端温度", unit.display(t), unit.symbol());
    out.metric("测得热电势", measured, "mV");
    out.metric(
        "补偿后热电势（参考端 0 °C）",
        tc::thermocouple_emf(kind, t)?,
        "mV",
    );
    out.metric("参考端热电势", tc::thermocouple_emf(kind, cold)?, "mV");
    out.metric(
        "微分热电势 dE/dt",
        tc::slope(kind, t)? / unit.span(),
        &format!("mV/{}", unit.symbol()),
    );
    for a in tc::tolerance::calculate(kind, t) {
        let bounds = a.delta.and_then(|d| {
            Some([
                tc::thermocouple_emf(kind, t - d).ok()?,
                tc::thermocouple_emf(kind, t + d).ok()?,
            ])
        });
        out.tolerances.push(json!({"name":a.name,"formula":a.formula,"min":a.min,"max":a.max,"delta":a.delta.map(|d|d*unit.span()),"unit":unit.symbol(),"temperature":a.delta.map(|d|[unit.display(t-d),unit.display(t+d)]),"signal":bounds,"signalUnit":"mV"}));
    }
    if out.tolerances.is_empty() {
        out.warnings.push("此分度号未内置等级允差数据。".into())
    }
    division(&mut out, i, tc::bounds(kind)?, "mV", |t| {
        tc::measured_emf(kind, t, cold, correction)
    })?;
    Ok(out)
}

fn industrial(i: &Input) -> Result<Output, String> {
    let g = &rtd::model::GRADUATIONS[i.index("graduation", 0, rtd::model::GRADUATIONS.len())?];
    // 已切回内置分度号时，隐藏的自定义系数不应继续参与校验。
    let custom = if g.custom() {
        Some([
            i.number("a", 0.0039083)?,
            i.number("b", -5.775e-7)?,
            i.number("c", -4.183e-12)?,
        ])
    } else {
        None
    };
    let m = rtd::model::RtdModel::new(g, i.number("r0", g.r0)?, custom)?;
    let unit = i.unit()?;
    let dir = i.index("direction", 0, 2)?;
    let value = i.number(
        "value",
        if dir == 0 {
            unit.display(100.0)
        } else {
            m.resistance(100.0_f64.min(g.max))?
        },
    )?;
    let (t, r) = if dir == 0 {
        let t = celsius_in_bounds(unit, value, (g.min, g.max));
        (t, m.resistance(t)?)
    } else {
        (m.temperature(value)?, value)
    };
    let mut out = Output::default();
    out.metric("温度", unit.display(t), unit.symbol());
    out.metric("电阻", r, "Ω");
    out.metric("电阻比 R/R₀", r / m.r0, "");
    out.metric(
        "电阻变化率 dR/dt",
        m.slope(t)? / unit.span(),
        &format!("Ω/{}", unit.symbol()),
    );
    for a in rtd::tolerance::tolerances(g.tolerance, t, i.index("structure", 0, 2)? == 1) {
        let rb = a
            .value
            .and_then(|d| Some([m.resistance(t - d).ok()?, m.resistance(t + d).ok()?]));
        out.tolerances.push(json!({"name":a.name,"formula":a.formula,"min":a.min,"max":a.max,"delta":a.value.map(|d|d*unit.span()),"unit":unit.symbol(),"temperature":a.value.map(|d|[unit.display(t-d),unit.display(t+d)]),"signal":rb,"signalUnit":"Ω"}));
    }
    if g.tolerance == rtd::model::ToleranceKind::PlatinumReference {
        out.warnings
            .push("当前分度号的铂电阻等级结果为公式参考值，请按实际适用标准评定。".into())
    }
    if out.tolerances.is_empty() {
        out.warnings.push("此分度号未内置等级允差数据。".into())
    }
    division(&mut out, i, (g.min, g.max), "Ω", |t| m.resistance(t))?;
    Ok(out)
}

fn standard(i: &Input) -> Result<Output, String> {
    use tc::standard::{StandardCouple, zones};
    let kind = ["S", "B"][i.index("type", 0, 2)?];
    let mode = i.index("source", 0, 2)?;
    let defaults: &[(f64, f64)] = if kind == "S" {
        &[(419.527, 3.4472), (660.323, 5.8605), (1084.62, 10.5822)]
    } else {
        &[
            (1100., 5.8029),
            (1200., 6.805),
            (1300., 7.8809),
            (1400., 8.9926),
            (1500., 10.1205),
        ]
    };
    let mut m = StandardCouple {
        kind,
        points: vec![],
        zone: None,
    };
    let mut out = Output::default();
    if mode == 0 {
        let measured = i.values(
            "fixed",
            &defaults.iter().map(|p| p.1).collect::<Vec<_>>(),
            defaults.len(),
            defaults.len(),
        )?;
        m.points = defaults
            .iter()
            .zip(measured)
            .map(|(p, e)| (p.0, e))
            .collect();
    } else {
        let zones = zones(kind);
        let mut zone = zones[i.index("zone", 0, zones.len())?].clone();
        if i.index("coefficientSource", 0, 2)? == 1 {
            let points = i.points(&[], 2, 30)?;
            StandardCouple::fit_zone_coefficients(&mut zone, &points)?;
            out.details = json!({"calibrationResiduals":points.iter().map(|(t,e)|json!([t,e-crate::calculators::calibration::polynomial(&zone.2,*t)])).collect::<Vec<_>>()});
        } else {
            zone.2 = i.values("coefficients", &zone.2, zone.2.len(), zone.2.len())?;
        }
        out.coefficients = zone.2.clone();
        // 手动与拟合模式均返回实际使用的系数，供页面与完整结果导出。
        if out.details.is_null() {
            out.details = json!({});
        }
        out.details["coefficients"] = json!({"C":zone.2});
        m.zone = Some(zone);
    }
    let unit = i.unit()?;
    let dir = i.index("direction", 0, 2)?;
    let v = i.number("value", if dir == 0 { unit.display(1000.) } else { 9.587 })?;
    let cold = unit.celsius(i.number("cold", unit.display(0.0))?);
    let corr = i.number("correction", 0.)?;
    let (t, e) = if dir == 0 {
        let t = unit.celsius(v);
        (t, m.emf(t)? - m.emf(cold)? - corr)
    } else {
        (m.temperature(v + corr + m.emf(cold)?)?, v)
    };
    out.metric("温度", unit.display(t), unit.symbol());
    out.metric("测得热电势", e, "mV");
    out.metric("补偿后热电势", m.emf(t)?, "mV");
    let bounds = m
        .zone
        .as_ref()
        .map(|z| (z.0, z.1))
        .unwrap_or(if kind == "S" {
            (-50., 1768.1)
        } else {
            (0., 1820.)
        });
    let lo = (t - 0.01).max(bounds.0);
    let hi = (t + 0.01).min(bounds.1);
    out.metric("参考端热电势", m.emf(cold)?, "mV");
    out.metric(
        "微分热电势 dE/dt",
        (m.emf(hi)? - m.emf(lo)?) / (hi - lo) / unit.span(),
        &format!("mV/{}", unit.symbol()),
    );
    division(&mut out, i, bounds, "mV", |t| {
        Ok(m.emf(t)? - m.emf(cold)? - corr)
    })?;
    Ok(out)
}

fn cvd_fit(i: &Input) -> Result<Output, String> {
    use crate::calculators::math::{cvd_basis, cvd_resistance, solve_3x3};
    let r0 = i.number("r0", 100.)?;
    if r0 <= 0.0 {
        return Err("R₀ 必须大于 0".into());
    }
    let pts = i.points(
        &[(-100., 60.25584), (100., 138.5055), (200., 175.856)],
        3,
        3,
    )?;
    if !pts.iter().any(|p| p.0 < 0.) {
        return Err("至少一个校准点须低于 0 °C".into());
    }
    if pts
        .iter()
        .any(|p| !(-200.0..=850.0).contains(&p.0) || p.1 <= 0.)
    {
        return Err("校准点须在 −200～850 °C 内，电阻大于 0".into());
    }
    let c = solve_3x3(
        pts.iter().map(|p| cvd_basis(p.0)).collect(),
        pts.iter().map(|p| p.1 / r0 - 1.).collect(),
    )
    .ok_or("校准点无法独立确定系数")?;
    let mut o = Output::default();
    for (name, v) in ["A", "B", "C"].into_iter().zip(c) {
        o.metric(name, v, "")
    }
    o.coefficients = c.to_vec();
    o.headings = vec![
        "温度 (°C)".into(),
        "实测电阻 (Ω)".into(),
        "拟合电阻 (Ω)".into(),
        "电阻残差 (Ω)".into(),
    ];
    for (t, r) in pts {
        let fitted = cvd_resistance(t, r0, c[0], c[1], c[2]);
        o.rows
            .push(vec![json!(t), json!(r), json!(fitted), json!(r - fitted)])
    }
    Ok(o)
}

fn platinum(i: &Input) -> Result<Output, String> {
    if i.index("model", 0, 2)? == 1 {
        return cvd_fit(i);
    }
    let hr = i.index("highRange", 1, 7)?;
    let lr = i.index("lowRange", 0, 3)?;
    if hr == 6 && lr == 2 {
        return Err("高低温分区不能同时停用".into());
    }
    let full = i.index("deviation", 0, 2)? == 0;
    let fitted = i.index("source", 0, 2)? == 1;
    let low = &sprt::ranges::LOW_RANGES[lr];
    // 拟合模式与停用分区不读取隐藏的证书系数，防止旧输入影响新模式。
    let coefficient = |key, default, enabled| {
        if !fitted && enabled {
            i.number(key, default)
        } else {
            Ok(default)
        }
    };
    let mut m = sprt::model::Sprt {
        rtp: i.number("rtp", 25.12461)?,
        a: coefficient("a", -0.000272072, hr != 6)?,
        b: coefficient("b", 0.0000178924, hr != 6 && (hr <= 3 || !full))?,
        c: coefficient("c", -0.0000112269, hr != 6 && (hr <= 1 || !full))?,
        a4: coefficient("a4", 0., lr != 2)?,
        b4: coefficient("b4", 0., lr != 2)?,
        low_quadratic: full && lr == 1,
        ..Default::default()
    };
    let mut out = Output::default();
    let highs: Vec<f64> = match hr {
        0 | 1 => vec![231.928, 419.527, 660.323],
        2 => vec![231.928, 419.527],
        3 => vec![156.599, 231.928],
        4 => vec![156.599],
        5 => vec![29.765],
        _ => vec![],
    };
    if fitted {
        let mut hp = vec![];
        let mut lp = vec![];
        for (temps, list, prefix) in [
            (&highs[..], &mut hp, "highFixed"),
            (low.fixed_points, &mut lp, "lowFixed"),
        ] {
            let defaults = temps
                .iter()
                .map(|t| m.resistance(*t))
                .collect::<Result<Vec<_>, _>>()?;
            let values = i.values(prefix, &defaults, temps.len(), temps.len())?;
            list.extend(temps.iter().copied().zip(values));
        }
        m.fit(
            &hp,
            &lp,
            if hr <= 1 {
                3
            } else if hr <= 3 {
                2
            } else {
                1
            },
        )?;
        if full && hr == 0 {
            m.w660 = hp.iter().find(|p| p.0 == 660.323).ok_or("缺少铝点")?.1 / m.rtp;
            m.fit_silver(961.78, i.number("silver", m.resistance(961.78)?)?)?;
        }
    }
    if full {
        if hr == 0 && !fitted {
            m.d = i.number("d", m.d)?;
            m.w660 = i.number("w660", m.resistance(660.323)? / m.rtp)?;
        }
        if hr >= 2 {
            m.c = 0.
        }
        if hr >= 4 {
            m.b = 0.
        }
    }
    let bounds = (
        low.minimum,
        [961.78, 660.323, 419.527, 231.928, 156.599, 29.765, 0.][hr],
    );
    let u = i.unit()?;
    let dir = i.index("direction", 0, 2)?;
    let v = i.number(
        "value",
        if dir == 0 {
            u.display(if hr == 6 { -20. } else { 20. })
        } else {
            25.
        },
    )?;
    let curve = |t| m.resistance_branch(t, hr != 6 && t >= 0.);
    let (t, r) = if dir == 0 {
        let t = celsius_in_bounds(u, v, bounds);
        (t, curve(t)?)
    } else {
        (
            if hr == 6 {
                m.temperature_branch(v, false)?
            } else {
                m.temperature(v)?
            },
            v,
        )
    };
    if t < bounds.0 - 1e-8 || t > bounds.1 + 1e-8 {
        return Err(format!("温度须在 {}～{} °C 内", bounds.0, bounds.1));
    }
    let lo = (t - 1.).max(bounds.0);
    let hi = (t + 1.).min(bounds.1);
    let slope = (curve(hi)? - curve(lo)?) / (hi - lo);
    out.metric("温度", u.display(t), u.symbol());
    out.metric("电阻", r, "Ω");
    out.metric("电阻比 W", r / m.rtp, "");
    out.metric("电阻变化率", slope / u.span(), &format!("Ω/{}", u.symbol()));
    out.details =
        json!({"coefficients":{"a":m.a,"b":m.b,"c":m.c,"a4":m.a4,"b4":m.b4,"d":m.d,"w660":m.w660}});
    division(&mut out, i, bounds, "Ω", curve)?;
    Ok(out)
}

fn ntc(i: &Input) -> Result<Output, String> {
    use thermistor::{BetaThermistor, polynomial::PolynomialThermistor};
    let family = i.index("family", 0, 2)?;
    let u = i.unit()?;
    let direction = i.index("direction", 0, 2)?;
    let mut out = Output::default();
    let min = i.number("min", -20.)?;
    let max = i.number("max", 100.)?;
    if min <= -273.15 || min >= max {
        return Err("校准温区须高于绝对零度，且下限小于上限".into());
    }
    if family == 1 {
        let m = BetaThermistor::new(
            i.number("r0", 10000.)?,
            u.celsius(i.number("t0", u.display(25.))?),
            i.number("beta", 3950.)?,
        )?;
        let v = i.number(
            "value",
            if direction == 0 {
                u.display(25.)
            } else {
                10000.
            },
        )?;
        let (t, r) = if direction == 0 {
            let t = u.celsius(v);
            (t, m.resistance(t)?)
        } else {
            (m.temperature(v)?, v)
        };
        if t < min || t > max {
            return Err("温度须处于有效校准温区内".into());
        }
        out.metric("温度", u.display(t), u.symbol());
        out.metric("电阻", r, "Ω");
        let slope = -r * i.number("beta", 3950.)? / (t + 273.15).powi(2);
        crate::calculators::numerics::finite(&[slope])?;
        out.metric("电阻变化率", slope / u.span(), &format!("Ω/{}", u.symbol()));
        division(&mut out, i, (min, max), "Ω", |t| m.resistance(t))?;
        return Ok(out);
    }
    let order = i.index("order", 0, 4)? + 3;
    let sh = i.index("form", 0, 2)? == 1;
    let a0 = 1.0 / 298.15 - 10000.0_f64.ln() / 3950.;
    let b0 = 10000.0_f64.ln() - 3950. / 298.15;
    let mut a = vec![0.; order + 1];
    let mut b = a.clone();
    a[0] = a0;
    a[1] = 1. / 3950.;
    b[0] = b0;
    b[1] = 3950.;
    let m = if i.index("source", 0, 2)? == 1 {
        let beta = BetaThermistor::new(10000., 25., 3950.)?;
        let defaults = (0..7)
            .map(|j| {
                let t = -20. + j as f64 * 20.;
                Ok((t, beta.resistance(t)?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let pts = i.points(&defaults, if sh { order } else { order + 1 }, 30)?;
        let model = PolynomialThermistor::fit(&pts, order, sh)?;
        out.details = json!({"calibrationResiduals":pts.iter().map(|(t,r)|Ok(json!([t,t-model.temperature(*r)?]))).collect::<Result<Vec<_>,String>>()?});
        model
    } else {
        a = i.values("aCoefficients", &a, order + 1, order + 1)?;
        b = i.values("bCoefficients", &b, order + 1, order + 1)?;
        if sh {
            a[2] = 0.;
            b[2] = 0.;
        }
        PolynomialThermistor { a, b }
    };
    let dir = i.index("direction", 0, 2)?;
    let v = i.number("value", if dir == 0 { u.display(25.) } else { 10000. })?;
    let (t, r) = if dir == 0 {
        let t = u.celsius(v);
        (t, m.resistance(t)?)
    } else {
        (m.temperature(v)?, v)
    };
    if t < min || t > max {
        return Err("温度须处于有效校准温区内".into());
    }
    out.metric("温度", u.display(t), u.symbol());
    out.metric("电阻", r, "Ω");
    let lo = (t - 1.).max(min);
    let hi = (t + 1.).min(max);
    let slope = (m.resistance(hi)? - m.resistance(lo)?) / (hi - lo);
    out.metric("电阻变化率", slope / u.span(), &format!("Ω/{}", u.symbol()));
    out.details
        .as_object_mut()
        .map(|d| d.insert("coefficients".into(), json!({"A":m.a,"B":m.b})));
    if out.details.is_null() {
        out.details = json!({"coefficients":{"A":m.a,"B":m.b}})
    }
    division(&mut out, i, (min, max), "Ω", |t| m.resistance(t))?;
    Ok(out)
}
