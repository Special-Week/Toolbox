//! 温标页面适配；正反算、允差与分度表共用桌面参考曲线。
use crate::api::{Input, Output, celsius_in_bounds, division};
use crate::calculators::common::{format_process_computed as c, format_process_number as n};
use crate::calculators::{rtd, sprt, thermistor, thermocouple as tc};
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
    let kinds = tc::WORKING_KINDS;
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
    out.process = vec![format!(
        "{kind} 型参考函数；热端温度 {} °C，参考端温度 {} °C，修正量 {} mV。",
        c(t),
        c(cold),
        n(correction)
    )];
    out.process
        .extend(tc::reference_process(kind, t, "热端", direction != 0)?);
    out.process.push(String::new());
    out.process
        .extend(tc::reference_process(kind, cold, "参考端", false)?);
    let check = tc::measured_emf(kind, t, cold, correction)?;
    out.process.push(format!(
        "测量热电势 = E(热端) − E(参考端) − 修正量 = {} − {} − {} = {} mV。",
        c(tc::thermocouple_emf(kind, t)?),
        c(tc::thermocouple_emf(kind, cold)?),
        n(correction),
        c(check)
    ));
    if direction != 0 {
        out.process.push(format!(
            "先将输入电势加回参考端电势与修正量，在参考函数单调区间二分；代回残差 = {} mV。",
            c(check - measured)
        ));
    }
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
    }, None)?;
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
    out.process.push(format!(
        "分度号 {}；R₀ = {} Ω；温度 = {} °C，电阻 = {} Ω。",
        g.name,
        n(m.r0),
        c(t),
        c(r)
    ));
    if let Some([a, b, cc]) = m.cvd_coefficients() {
        out.process.push(format!(
            "CVD 系数 A = {}，B = {}，C = {}；{}",
            n(a),
            n(b),
            n(cc),
            if t < 0. {
                "R/R₀ = 1 + At + Bt² + C(t−100)t³。"
            } else {
                "R/R₀ = 1 + At + Bt²。"
            }
        ));
    } else if let Some(form) = m.polynomial_form(t) {
        out.process.push(format!(
            "{}：z = t + {} = {}；R/R₀ = {} × Σcᵢzⁱ，系数按升幂：",
            form.segment,
            n(form.shift),
            c(form.argument),
            n(form.scale)
        ));
        out.process.extend(
            form.coefficients()
                .iter()
                .enumerate()
                .map(|(j, v)| format!("c[{j}] = {}。", n(*v))),
        );
    }
    out.process.push(format!(
        "代入温度正算 R = {} Ω；电阻比 R/R₀ = {}；变化率 dR/dt = {} Ω/°C。",
        c(m.resistance(t)?),
        c(r / m.r0),
        c(m.slope(t)?)
    ));
    if dir != 0 {
        out.process
            .push("电阻反算采用同一参考曲线的有界二分。".into());
    }
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
    division(&mut out, i, (g.min, g.max), "Ω", |t| m.resistance(t), None)?;
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
            let base = zone.2.clone();
            StandardCouple::fit_zone_coefficients(&mut zone, &points)?;
            out.process.push("校准点拟合：在基准多项式上仅调整一次和二次项，按 ΔE = 实测电势 − 基准电势建立最小二乘方程。".into());
            out.process.extend(
                base.iter()
                    .enumerate()
                    .map(|(j, v)| format!("基准系数 b[{j}] = {}。", n(*v))),
            );
            for (j, (temp, measured)) in points.iter().enumerate() {
                let base_e = crate::calculators::calibration::polynomial(&base, *temp);
                let fitted_e = crate::calculators::calibration::polynomial(&zone.2, *temp);
                out.process.push(String::new());
                out.process.push(format!("校准点 {}：t = {} °C，实测 = {} mV，基准电势 = {} mV；ΔE = 实测 − 基准 = {} mV；拟合残差 = 实测 − 拟合电势 = {} mV。",j+1,n(*temp),n(*measured),c(base_e),c(*measured-base_e),c(*measured-fitted_e)));
            }
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
    let bounds = m.bounds();
    let unit = i.unit()?;
    let dir = i.index("direction", 0, 2)?;
    let v = i.number("value", if dir == 0 { unit.display(1000.) } else { 9.587 })?;
    let cold = celsius_in_bounds(unit, i.number("cold", unit.display(0.0))?, bounds);
    let cold_emf = m.reference_end_emf(cold)?;
    let corr = i.number("correction", 0.)?;
    let (t, e) = if dir == 0 {
        let t = celsius_in_bounds(unit, v, bounds);
        (t, m.emf(t)? - cold_emf - corr)
    } else {
        (m.temperature(v + corr + cold_emf)?, v)
    };
    out.metric("温度", unit.display(t), unit.symbol());
    out.metric("测得热电势", e, "mV");
    out.metric("补偿后热电势", m.emf(t)?, "mV");
    for (label, temp) in [("热端", t), ("参考端", cold)] {
        out.process.push(String::new());
        if let Some((lo, hi, coefficients)) = &m.zone {
            out.process.push(format!(
                "{label}：系数温区 {}～{} °C，t = {} °C；E(t) = Σcᵢtⁱ。参考端在 0 °C 时电势为 0 mV。",
                n(*lo),
                n(*hi),
                c(temp)
            ));
            out.process.extend(
                coefficients
                    .iter()
                    .enumerate()
                    .map(|(j, v)| format!("c[{j}] = {}。", n(*v))),
            );
        } else {
            use tc::standard::FixedMethod;
            let adjustment = m.fixed_adjustment(temp)?;
            out.process.extend(tc::reference_process(
                kind,
                temp,
                &format!("{label}参考曲线"),
                true,
            )?);
            for (j, ((point, measured), offset)) in
                m.points.iter().zip(&adjustment.offsets).enumerate()
            {
                out.process.push(format!(
                    "固定点 {}：t = {} °C，实测 {} mV，参考 {} mV，偏差 δ = 实测 − 参考 = {} mV。",
                    j + 1,
                    n(*point),
                    n(*measured),
                    c(tc::thermocouple_emf(kind, *point)?),
                    c(*offset)
                ));
            }
            match adjustment.method {
                FixedMethod::SLinear => out.process.push(format!("S 型低于 400 °C：δ = δ₁t/t₁ = {} × {} / {} = {} mV。", c(adjustment.offsets[0]), c(temp), n(m.points[0].0), c(adjustment.value))),
                FixedMethod::SQuadratic(terms) => {
                    out.process.push("S 型拉格朗日二次插值：δ = Σδᵢ∏ⱼ≠ᵢ[(t−tⱼ)/(tᵢ−tⱼ)]。".into());
                    for (index, term) in terms.iter().enumerate() {
                        let mut substitution = c(adjustment.offsets[index]);
                        for (j, &(point, _)) in m.points.iter().enumerate() {
                            if index != j {
                                substitution.push_str(&format!(" × ({}−{})/({}−{})", c(temp), n(point), n(m.points[index].0), n(point)));
                            }
                        }
                        out.process.push(String::new());
                        out.process.push(format!("固定点 {} 的插值项 = {} = {} mV。", index + 1, substitution, c(*term)));
                    }
                }
                FixedMethod::BZero => out.process.push("B 型低于 23 °C：修正量为 0。".into()),
                FixedMethod::BLinearFrom23 => out.process.push(format!("B 型低温段：δ = δ₁(t−23)/(t₁−23) = {} × ({}−23)/({}−23) = {} mV。", c(adjustment.offsets[0]), c(temp), n(m.points[0].0), c(adjustment.value))),
                FixedMethod::BConstant(index) => out.process.push(format!("B 型采用固定点 {} 的偏差 = {} mV。", index + 1, c(adjustment.value))),
                FixedMethod::BTransition { index, fraction } => out.process.push(format!("B 型固定点 {} 与 {} 之间的过渡：δ = δᵢ + (δᵢ₊₁−δᵢ)(t−tᵢ−30)/40 = {} + ({}−{}) × ({}−{}−30)/40 = {} mV；过渡比例 = {}。", index + 1, index + 2, c(adjustment.offsets[index]), c(adjustment.offsets[index + 1]), c(adjustment.offsets[index]), c(temp), n(m.points[index].0), c(adjustment.value), c(fraction))),
            }
            out.process.push(format!(
                "固定点修正量 = {} mV；校准电势 = 参考电势 + 修正量 = {} + {} = {} mV。",
                c(adjustment.value),
                c(tc::thermocouple_emf(kind, temp)?),
                c(adjustment.value),
                c(m.emf(temp)?)
            ));
        }
        let emf = if label == "参考端" { cold_emf } else { m.emf(temp)? };
        out.process.push(format!("{label}电势 = {} mV。", c(emf)));
    }
    let reconstructed = m.emf(t)? - cold_emf - corr;
    out.process.push(format!(
        "测量电势 = E(热端) − E(参考端) − 修正量 = {} − {} − {} = {} mV。",
        c(m.emf(t)?),
        c(cold_emf),
        n(corr),
        c(reconstructed)
    ));
    if dir != 0 {
        out.process.push(format!(
            "反算对补偿后电势执行{}有界二分；代回残差 = {} mV。",
            if m.zone.is_none() { "分段" } else { "" },
            c(reconstructed - e)
        ));
    }
    let bounds = m.bounds();
    let slope = m.slope(t)?;
    out.metric("参考端热电势", cold_emf, "mV");
    out.optional(
        "微分热电势 dE/dt",
        slope.map(|(_, _, slope)| slope / unit.span()),
        &format!("mV/{}", unit.symbol()),
    );
    out.process.push(slope.map_or_else(
        || "分段交界处的微分热电势未定义。".to_owned(),
        |(lo, hi, slope)| format!(
            "微分热电势在同一分段内取差分：[E({}) − E({})]/({} − {}) = {} mV/°C。",
            c(hi), c(lo), c(hi), c(lo), c(slope)
        ),
    ));
    division(&mut out, i, bounds, "mV", |t| {
        Ok(m.emf(t)? - cold_emf - corr)
    }, Some(&|t| m.slope(t).map(|value| value.map(|(_, _, slope)| slope))))?;
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
    let coefficients = solve_3x3(
        pts.iter().map(|p| cvd_basis(p.0)).collect(),
        pts.iter().map(|p| p.1 / r0 - 1.).collect(),
    )
    .ok_or("校准点无法独立确定系数")?;
    let mut o = Output::default();
    for (name, v) in ["A", "B", "C"].into_iter().zip(coefficients) {
        o.metric(name, v, "")
    }
    o.coefficients = coefficients.to_vec();
    o.process.push(format!(
        "CVD 三点拟合：R/R₀ − 1 = At + Bt² + C(t−100)t³（t ≥ 0 时 C 项为零），R₀ = {} Ω。",
        n(r0)
    ));
    o.process.push(format!(
        "缩放线性方程求得 A = {}，B = {}，C = {}。",
        n(coefficients[0]),
        n(coefficients[1]),
        n(coefficients[2])
    ));
    o.headings = vec![
        "温度 (°C)".into(),
        "实测电阻 (Ω)".into(),
        "拟合电阻 (Ω)".into(),
        "电阻残差 (Ω)".into(),
    ];
    for (t, r) in pts {
        let fitted = cvd_resistance(t, r0, coefficients[0], coefficients[1], coefficients[2]);
        o.process.push(String::new());
        o.process.push(format!(
            "校准点 t = {} °C，实测 R = {} Ω；R/R₀ − 1 = {}，拟合电阻 = {} Ω，残差 = {} Ω。",
            n(t),
            n(r),
            c(r / r0 - 1.),
            c(fitted),
            c(r - fitted)
        ));
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
        out.process.push("固定点拟合使用 W = R/Rtp 与参考电阻比 Wr；高、低温偏差分别建立线性方程并采用最小二乘求系数。".into());
        for (j, (temp, resistance)) in hp.iter().chain(&lp).enumerate() {
            let w = *resistance / m.rtp;
            out.process.push(String::new());
            out.process.push(format!(
                "固定点 {}：t = {} °C，R = {} Ω；W = R/Rtp = {}，Wr(t) = {}，偏差 W−Wr = {}。",
                j + 1,
                n(*temp),
                n(*resistance),
                c(w),
                c(sprt::curves::reference(*temp)),
                c(w - sprt::curves::reference(*temp))
            ));
        }
        if full && hr == 0 {
            m.w660 = hp.iter().find(|p| p.0 == 660.323).ok_or("缺少铝点")?.1 / m.rtp;
            m.fit_silver(961.78, i.number("silver", m.resistance(961.78)?)?)?;
        }
    }
    if hr == 6 {
        m.a = 0.;
        m.b = 0.;
        m.c = 0.;
    }
    if full {
        if hr == 0 && !fitted {
            let default_w660 = m.resistance(660.323)? / m.rtp;
            m.d = i.number("d", m.d)?;
            m.w660 = i.number("w660", default_w660)?;
            if m.w660 <= 1.0 {
                return Err("W660 须大于 1".into());
            }
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
    let curve = |t| m.resistance_branch(t, sprt::model::high_for_temperature(t, hr != 6, lr != 2));
    let (t, r) = if dir == 0 {
        let t = celsius_in_bounds(u, v, bounds);
        (t, curve(t)?)
    } else {
        (
            m.temperature_branch(
                v,
                sprt::model::high_for_resistance(v, m.rtp, hr != 6, lr != 2),
            )?,
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
    let high = if dir == 0 {
        sprt::model::high_for_temperature(t, hr != 6, lr != 2)
    } else {
        sprt::model::high_for_resistance(r, m.rtp, hr != 6, lr != 2)
    };
    let w = r / m.rtp;
    let wr = m.reference_ratio(w, high);
    let (coefficients, x, low_reference) = m.reference_function(w, wr, high);
    out.process.push(format!(
        "Rtp = {} Ω，R = {} Ω；W = R/Rtp = {}。",
        n(m.rtp),
        c(r),
        c(w)
    ));
    out.process.push(if high {format!("高温：Wr = W − a(W−1) − b(W−1)² − c(W−1)³ − d·max(W−W660,0)²；a、b、c、d、W660 = {}、{}、{}、{}、{}；Wr = {}。",n(m.a),n(m.b),n(m.c),n(m.d),n(m.w660),c(wr))} else {format!("{}；a = {}，b = {}，Wr = {}。",if m.low_quadratic {"低温：Wr = W − a₅(W−1) − b₅(W−1)²"}else{"低温：Wr = W − a₄(W−1) − b₄(W−1)ln(W)"},n(m.a4),n(m.b4),c(wr))});
    out.process.push(format!(
        "{}，归一化自变量 = {}；参考函数系数按升幂：",
        if low_reference {
            "低温：x = (Wr^(1/6)−0.65)/0.35"
        } else {
            "高温：x = (Wr−2.64)/1.64"
        },
        c(x)
    ));
    out.process.extend(
        coefficients
            .iter()
            .enumerate()
            .map(|(j, v)| format!("c[{j}] = {}。", n(*v))),
    );
    out.process.push(format!(
        "{} = {} °C；正算电阻通过该分支的有界二分求解。",
        if low_reference {
            "温度 t = 273.16Σcᵢxⁱ − 273.15"
        } else {
            "温度 t = Σcᵢxⁱ"
        },
        c(m.temperature_branch(r, high)?)
    ));
    out.metric("电阻变化率", slope / u.span(), &format!("Ω/{}", u.symbol()));
    out.details =
        json!({"coefficients":{"a":m.a,"b":m.b,"c":m.c,"a4":m.a4,"b4":m.b4,"d":m.d,"w660":m.w660}});
    division(&mut out, i, bounds, "Ω", curve, None)?;
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
            let t = celsius_in_bounds(u, v, (min, max));
            (t, m.resistance(t)?)
        } else {
            // β 模型单调递减；端点相同表示双精度下无法区分温区内的电阻。
            let upper_r = m.resistance(min).ok();
            let lower_r = m.resistance(max).ok();
            if upper_r == Some(v) && lower_r == Some(v) {
                return Err("校准温区两端电阻在双精度下无法区分，不能唯一反算温度".into());
            }
            let inverse = m.temperature(v)?;
            if upper_r.is_some_and(|r| v > r) || lower_r.is_some_and(|r| v < r) {
                return Err("温度须处于有效校准温区内".into());
            }
            // 端点溢出时，只吸收反算温度的微小舍入误差。
            let roundoff = 4.0 * f64::EPSILON * (inverse + 273.15).abs().max(1.0);
            let t = if upper_r == Some(v) || (inverse < min && min - inverse <= roundoff) {
                min
            } else if lower_r == Some(v) || (inverse > max && inverse - max <= roundoff) {
                max
            } else {
                inverse
            };
            (t, v)
        };
        if t < min || t > max {
            return Err("温度须处于有效校准温区内".into());
        }
        out.metric("温度", u.display(t), u.symbol());
        out.metric("电阻", r, "Ω");
        let r0 = i.number("r0", 10000.)?;
        let t0 = u.celsius(i.number("t0", u.display(25.))?);
        let beta = i.number("beta", 3950.)?;
        out.process = vec![
            format!(
                "β 模型：R₀ = {} Ω，T₀ = {} K，β = {} K。",
                n(r0),
                c(t0 + 273.15),
                n(beta)
            ),
            "ln(R) = ln(R₀) + β(1/T−1/T₀)；1/T = 1/T₀ + [ln(R)−ln(R₀)]/β。T 用 K、R 用 Ω。"
                .to_string(),
            format!(
                "当前温度 = {} °C = {} K，电阻 = {} Ω。",
                c(t),
                c(t + 273.15),
                c(r)
            ),
        ];
        if direction == 0 {
            out.process.push(format!(
                "ln(R) = ln({}) + {} × (1/{} − 1/{}) = {}；R = exp(ln R) = {} Ω。",
                n(r0),
                n(beta),
                c(t + 273.15),
                c(t0 + 273.15),
                c(m.log_resistance(t + 273.15)),
                c(r)
            ));
        } else {
            let inv = m.inverse_temperature(r);
            out.process.push(format!(
                "1/T = 1/{} + [ln({}) − ln({})]/{} = {} K⁻¹；t = 1/(1/T) − 273.15 = {} °C。",
                c(t0 + 273.15),
                n(r),
                n(r0),
                n(beta),
                c(inv),
                c(t)
            ));
        }
        let slope = m.slope_at_resistance(t, r)?;
        out.process.push(format!(
            "电阻变化率 dR/dt = −Rβ/T² = −{} × {} / {}² = {} Ω/°C；按显示温标换算后为 {} Ω/{}。",
            c(r),
            n(beta),
            c(t + 273.15),
            c(slope),
            c(slope / u.span()),
            u.symbol()
        ));
        out.metric("电阻变化率", slope / u.span(), &format!("Ω/{}", u.symbol()));
        division(
            &mut out,
            i,
            (min, max),
            "Ω",
            |t| m.resistance(t),
            Some(&|t| Ok(Some(m.slope(t)?))),
        )?;
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
        out.process.push("校准点拟合分别使用 x = ln(R)、y = 1/T 拟合 A 系数，以及 x = 1/T、y = ln(R) 拟合 B 系数；使用列缩放 QR。".into());
        for (j, (temp, resistance)) in pts.iter().enumerate() {
            out.process.push(String::new());
            out.process.push(format!(
                "校准点 {}：t = {} °C，R = {} Ω；ln(R) = {}，1/T = {} K⁻¹，温度残差 = {} °C。",
                j + 1,
                n(*temp),
                n(*resistance),
                c(resistance.ln()),
                c(1. / (*temp + 273.15)),
                c(*temp - model.temperature(*resistance)?)
            ));
        }
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
        let t = celsius_in_bounds(u, v, (min, max));
        (t, m.resistance(t)?)
    } else {
        (m.temperature_in_bounds(v, min, max)?, v)
    };
    if t < min || t > max {
        return Err("温度须处于有效校准温区内".into());
    }
    out.metric("温度", u.display(t), u.symbol());
    out.metric("电阻", r, "Ω");
    out.process.push(format!(
        "{} 阶{}热敏电阻模型；t = {} °C，R = {} Ω。",
        order,
        if sh {
            " Steinhart–Hart "
        } else {
            "多项式"
        },
        c(t),
        c(r)
    ));
    out.process.push("电阻 → 温度：1/T = ΣAᵢ[ln(R)]ⁱ；温度 → 电阻：ln(R) = ΣBᵢ(1/T)ⁱ。两组系数分别拟合或输入，不保证精确互逆。".into());
    let (coeffs, x) = if dir == 0 {
        (&m.b, 1. / (t + 273.15))
    } else {
        (&m.a, r.ln())
    };
    out.process
        .push(format!("本次自变量 = {}；系数按升幂：", c(x)));
    out.process.extend(
        coeffs
            .iter()
            .enumerate()
            .map(|(j, v)| format!("{}[{j}] = {}。", if dir == 0 { "B" } else { "A" }, n(*v))),
    );
    out.process.push(format!(
        "多项式值 = {}；{}。",
        c(crate::calculators::calibration::polynomial(coeffs, x)),
        if dir == 0 {
            format!("电阻 = exp(多项式值) = {} Ω", c(r))
        } else {
            format!("温度 = 1/(多项式值) − 273.15 = {} °C", c(t))
        }
    ));
    let slope = m.slope(t)?;
    let kelvin = t + 273.15;
    let b_resistance = m.resistance(t)?;
    let b_derivative = crate::calculators::calibration::polynomial_slope(&m.b, 1. / kelvin).1;
    if dir != 0 {
        out.process.push("变化率使用温度 → 电阻的 B 系数：".into());
        out.process.extend(
            m.b.iter()
                .enumerate()
                .map(|(j, v)| format!("B[{j}] = {}。", n(*v))),
        );
    }
    out.process.push(format!(
        "令 x = 1/T，B′(x) = Σ i·B[i]·x^(i−1)；T = {} K，x = {} K⁻¹，B′(x) = {}，B 曲线电阻 R_B = {} Ω。",
        c(kelvin),
        c(1. / kelvin),
        c(b_derivative),
        c(b_resistance),
    ));
    out.process.push(format!(
        "链式法则：dR/dt = −R_B × B′(1/T) / T² = −{} × {} / {}² = {} Ω/°C；按显示温标换算后 = {} Ω/{}。",
        c(b_resistance),
        c(b_derivative),
        c(kelvin),
        c(slope),
        c(slope / u.span()),
        u.symbol()
    ));
    out.metric("电阻变化率", slope / u.span(), &format!("Ω/{}", u.symbol()));
    out.details
        .as_object_mut()
        .map(|d| d.insert("coefficients".into(), json!({"A":m.a,"B":m.b})));
    if out.details.is_null() {
        out.details = json!({"coefficients":{"A":m.a,"B":m.b}})
    }
    division(
        &mut out,
        i,
        (min, max),
        "Ω",
        |t| m.resistance(t),
        Some(&|t| Ok(Some(m.slope(t)?))),
    )?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platinum_round_trips_near_triple_point_for_enabled_ranges() {
        for (high, low, temperatures) in [
            (1, 0, &[-0.005, 0.0, 0.005, 0.01, 0.011][..]),
            (1, 1, &[-0.005, 0.0, 0.005, 0.01, 0.011][..]),
            (1, 2, &[0.0, 0.005, 0.01][..]),
            (6, 0, &[-0.005, 0.0][..]),
            (6, 1, &[-0.005, 0.0][..]),
        ] {
            for &temperature in temperatures {
                let forward = platinum(&Input(json!({"highRange":high,"lowRange":low,"direction":0,"value":temperature,"a4":0.003,"b4":0.001}))).unwrap();
                let resistance = forward
                    .metrics
                    .iter()
                    .find(|m| m.label == "电阻")
                    .unwrap()
                    .value
                    .as_f64()
                    .unwrap();
                let reverse = platinum(&Input(json!({"highRange":high,"lowRange":low,"direction":1,"value":resistance,"a4":0.003,"b4":0.001}))).unwrap();
                let restored = reverse
                    .metrics
                    .iter()
                    .find(|m| m.label == "温度")
                    .unwrap()
                    .value
                    .as_f64()
                    .unwrap();
                assert!(
                    (restored - temperature).abs() < 1e-8,
                    "分区 {high}/{low}：{temperature} → {restored}"
                );
            }
        }
    }

    #[test]
    fn platinum_validates_w660_and_clears_disabled_high_coefficients() {
        let err = platinum(&Input(json!({
            "highRange": 0,
            "lowRange": 0,
            "deviation": 0,
            "source": 0,
            "w660": 1.0
        })))
        .err()
        .unwrap();
        assert!(err.contains("W660 须大于 1"), "{err}");

        let low_only = platinum(&Input(json!({
            "highRange": 6,
            "lowRange": 0,
            "direction": 0,
            "value": -20.0
        })))
        .unwrap();
        let coeffs = &low_only.details["coefficients"];
        assert_eq!(coeffs["a"].as_f64(), Some(0.0));
        assert_eq!(coeffs["b"].as_f64(), Some(0.0));
        assert_eq!(coeffs["c"].as_f64(), Some(0.0));
    }

    #[test]
    fn beta_thermistor_round_trips_both_calibration_endpoints() {
        let min = -19.9;
        let max = 100.1;
        for (endpoint, is_min) in [(min, true), (max, false)] {
            let forward = ntc(&Input(json!({"family":1,"min":min,"max":max,"value":endpoint}))).unwrap();
            let resistance = forward.metrics[1].value.as_f64().unwrap();
            let reverse = |value| {
                ntc(&Input(json!({"family":1,"direction":1,"min":min,"max":max,"value":value})))
            };
            assert_eq!(reverse(resistance).unwrap().metrics[0].value.as_f64(), Some(endpoint));
            let outside = if is_min { resistance.next_up() } else { resistance.next_down() };
            assert!(reverse(outside).is_err());
        }
        // 即便下限电阻超出可表示范围，温区内部的正常反算仍可用；超出上限对应电阻的输入必须拒绝。
        assert!(ntc(&Input(json!({"family":1,"direction":1,"min":-273.149,"max":100,"value":10000}))).is_ok());
        assert!(ntc(&Input(json!({"family":1,"direction":1,"min":-273.149,"max":100,"value":100}))).is_err());
    }

    #[test]
    fn standard_couple_and_thermistor_accept_fahrenheit_range_endpoints() {
        let s_max_f = 1768.1 * 1.8 + 32.0;
        let out = standard(&Input(json!({
            "type": 0,
            "source": 0,
            "unit": 2,
            "direction": 0,
            "value": s_max_f
        })))
        .unwrap();
        assert!((out.metrics[0].value.as_f64().unwrap() - s_max_f).abs() < 1e-9);

        let ntc_min_f = -19.9 * 1.8 + 32.0;
        let ntc_out = ntc(&Input(json!({
            "family": 0,
            "unit": 2,
            "direction": 0,
            "min": -19.9,
            "max": 100.0,
            "value": ntc_min_f
        })))
        .unwrap();
        assert!((ntc_out.metrics[0].value.as_f64().unwrap() - ntc_min_f).abs() < 1e-9);
    }
}
