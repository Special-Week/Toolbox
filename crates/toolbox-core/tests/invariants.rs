//! 跨模型往返不变量与数值属性测试（纯计算层全量扫测）。
use toolbox_core::{
    calculators::{
        calibration::TemperatureUnit,
        humidity::website as humidity,
        math::{isa_altitude, isa_pressure},
        pressure::model::Range,
        rtd::model::{GRADUATIONS, RtdModel},
        sprt::{
            model::{Sprt, high_for_resistance, high_for_temperature},
            ranges::LOW_RANGES,
        },
        thermistor::{BetaThermistor, polynomial::PolynomialThermistor},
        thermocouple::{
            WORKING_KINDS, bounds, corrected_temperature, measured_emf,
            standard::{StandardCouple, zones},
        },
        transmitter::model::Transmitter,
        uncertainty::model::{
            DegreesOfFreedom, combine_dof, student_t_quantile,
        },
    },
    catalog::{Dimension, TOOLS, ToolKind},
    units::{convert_unit, units_for},
};

#[test]
fn all_15_working_thermocouples_round_trip_across_scales_and_reference_junctions() {
    for &kind in &WORKING_KINDS {
        let (min, max) = bounds(kind).unwrap();
        let low = if kind == "B" { 50.0 } else { min };
        let colds = if kind == "B" {
            &[0.0, 25.0, 100.0][..]
        } else {
            &[min, 0.0, (min + max) * 0.25][..]
        };
        for &cold_c in colds {
            for unit_idx in 0..3 {
                let unit = TemperatureUnit(unit_idx);
                let cold = unit.celsius_in_bounds(unit.display(cold_c), (min, max));
                for step in 0..=24 {
                    let t_c = low + (max - low) * (step as f64 / 24.0);
                    let t_disp = unit.display(t_c);
                    let t_in = unit.celsius_in_bounds(t_disp, (min, max));
                    let emf = measured_emf(kind, t_in, cold, 0.0).unwrap();
                    let (restored, _) = corrected_temperature(kind, emf, cold, 0.0).unwrap();
                    assert!(
                        (restored - t_in).abs() < 1e-6,
                        "热电偶 {kind} 温标 {} 冷端 {cold_c} °C：{t_in} → {emf} mV → {restored}",
                        unit.symbol()
                    );
                }
            }
        }
    }
}

#[test]
fn all_22_industrial_rtds_round_trip_across_scales() {
    let mut overlap_hits = 0usize;
    for def in &GRADUATIONS {
        let model = RtdModel::new(def, def.r0, None).unwrap();
        for unit_idx in 0..3 {
            let unit = TemperatureUnit(unit_idx);
            for step in 0..=20 {
                let t_c = def.min + (def.max - def.min) * (step as f64 / 20.0);
                let t_in = unit.celsius_in_bounds(unit.display(t_c), (def.min, def.max));
                let r = model.resistance(t_in).unwrap();
                match model.temperature(r) {
                    Ok(restored) => assert!(
                        (restored - t_in).abs() < 1e-6,
                        "热电阻 {} 温标 {}：{t_in} → {r} Ω → {restored}",
                        def.key,
                        unit.symbol()
                    ),
                    Err(err) if err.contains("多个温度") => {
                        overlap_hits += 1;
                    }
                    Err(err) => panic!(
                        "热电阻 {} 温标 {} 在 {t_in} °C 异常失败：{err}",
                        def.key,
                        unit.symbol()
                    ),
                }
            }
        }
    }
    // 仅有少数已知分段衔接点存在非单调重叠，绝大部分网格点均唯一往返
    assert!(overlap_hits <= 12, "过多分段重叠点：{overlap_hits}");
}

#[test]
fn sprt_all_enabled_subrange_combinations_round_trip() {
    let high_maxes: [Option<f64>; 7] = [
        Some(961.78),
        Some(660.323),
        Some(419.527),
        Some(231.928),
        Some(156.599),
        Some(29.765),
        None,
    ];
    for (high_idx, high_max) in high_maxes.into_iter().enumerate() {
        for (low_idx, low_def) in LOW_RANGES.iter().enumerate() {
            let high_enabled = high_max.is_some();
            let low_enabled = low_def.coefficients.is_some();
            if !high_enabled && !low_enabled {
                continue;
            }
            let min = if low_enabled { low_def.minimum } else { 0.0 };
            let max = high_max.unwrap_or(0.0);
            let mut model = Sprt {
                low_quadratic: low_idx == 1,
                ..Default::default()
            };
            if !high_enabled {
                model.a = 0.0;
                model.b = 0.0;
                model.c = 0.0;
                model.d = 0.0;
            }
            for step in 0..=16 {
                let t = min + (max - min) * (step as f64 / 16.0);
                let high = high_for_temperature(t, high_enabled, low_enabled);
                let r = model.resistance_branch(t, high).unwrap();
                let inv_high = high_for_resistance(r, model.rtp, high_enabled, low_enabled);
                let restored = model.temperature_branch(r, inv_high).unwrap();
                assert!(
                    (restored - t).abs() < 1e-7,
                    "SPRT 分区 {high_idx}/{low_idx}：{t} °C → {r} Ω → {restored} °C"
                );
            }
        }
    }
}

#[test]
fn standard_thermocouple_zones_round_trip() {
    for kind in ["S", "B"] {
        for zone in zones(kind) {
            let (min, max) = (zone.0, zone.1);
            let couple = StandardCouple {
                kind,
                points: Vec::new(),
                zone: Some(zone),
            };
            let low = if kind == "B" { min.max(50.0) } else { min };
            if low >= max {
                continue;
            }
            for step in 0..=16 {
                let t = low + (max - low) * (step as f64 / 16.0);
                let e = couple.emf(t).unwrap();
                let restored = couple.temperature(e).unwrap();
                assert!(
                    (restored - t).abs() < 1e-6,
                    "标准热电偶 {kind} 温区 [{min},{max}]：{t} → {e} → {restored}"
                );
            }
        }
    }
}

#[test]
fn thermistor_models_round_trip_and_analytic_slope_matches_central_difference() {
    let beta = BetaThermistor::new(10000.0, 25.0, 3950.0).unwrap();
    let mut points = Vec::new();
    for i in 0..9 {
        let t = -40.0 + i as f64 * 20.0;
        let r = beta.resistance(t).unwrap();
        assert!((beta.temperature(r).unwrap() - t).abs() < 1e-10);
        points.push((t, r));
    }
    let poly = PolynomialThermistor::fit(&points, 3, false).unwrap();
    for step in 0..=20 {
        let t = -40.0 + step as f64 * 8.0;
        let r = poly.resistance(t).unwrap();
        assert!((poly.temperature(r).unwrap() - t).abs() < 1e-8);
        let analytic = poly.slope(t).unwrap();
        let h = 1e-5;
        let numeric = (poly.resistance(t + h).unwrap() - poly.resistance(t - h).unwrap()) / (2.0 * h);
        let rel = (analytic - numeric).abs() / analytic.abs().max(1.0);
        assert!(
            rel < 1e-7,
            "热敏电阻解析导数与中心差分不符 at {t} °C: analytic={analytic}, numeric={numeric}"
        );
    }
}

#[test]
fn humidity_all_four_known_inputs_round_trip() {
    for &t in &[-30.0, -5.0, 0.5, 20.0, 60.0] {
        for &p_pa in &[80_000.0, 101_325.0] {
            for &rh in &[10.0, 50.0, 90.0] {
                let base = humidity::calculate(t, p_pa, 0, rh).unwrap();
                // 非正温度的露/霜点输入按冰面处理：霜点 <= 0 °C 时用霜点，否则用露点
                let point = match (base.frost, base.dew) {
                    (Some(f), _) if f <= 0.0 => Some(f),
                    (_, Some(d)) => Some(d),
                    _ => None,
                };
                if let Some(point) = point {
                    let from_point = humidity::calculate(t, p_pa, 1, point).unwrap();
                    assert!(
                        (from_point.rh - rh).abs() < 1e-6,
                        "露/霜点往返 t={t}, p={p_pa}, rh={rh}: got {}",
                        from_point.rh
                    );
                }
                if let Some(wet) = base.wet {
                    let from_wet = humidity::calculate(t, p_pa, 2, wet).unwrap();
                    assert!(
                        (from_wet.rh - rh).abs() < 1e-6,
                        "湿球往返 t={t}, p={p_pa}, rh={rh}: got {}",
                        from_wet.rh
                    );
                }
                let from_mix = humidity::calculate(t, p_pa, 3, base.mixing).unwrap();
                assert!(
                    (from_mix.rh - rh).abs() < 1e-6,
                    "混合比往返 t={t}, p={p_pa}, rh={rh}: got {}",
                    from_mix.rh
                );
            }
        }
    }
}

#[test]
fn isa_atmosphere_and_transmitter_round_trip() {
    for h in [-1000.0, 0.0, 5500.0, 11000.0, 15000.0, 20000.0, 32000.0, 47000.0, 51000.0, 71000.0, 84852.0] {
        let p = isa_pressure(h).unwrap();
        let restored = isa_altitude(p).unwrap();
        assert!((restored - h).abs() < 1e-5, "ISA {h} m → {p} Pa → {restored} m");
    }
    for root in [false, true] {
        let tx = Transmitter::new(Range::new(-50.0, 150.0).unwrap(), Range::new(4.0, 20.0).unwrap(), root);
        for step in 0..=20 {
            let x = -50.0 + step as f64 * 10.0;
            let (s, _) = tx.forward(x).unwrap();
            let (restored, _) = tx.inverse(s).unwrap();
            assert!((restored - x).abs() < 1e-10, "变送器 root={root}: {x} → {s} → {restored}");
        }
    }
}

#[test]
fn all_22_unit_dimensions_round_trip_for_every_pair() {
    let mut count = 0;
    for tool in TOOLS {
        let ToolKind::Unit(dimension) = tool.kind else {
            continue;
        };
        count += 1;
        let units = units_for(dimension);
        let probe = if matches!(dimension, Dimension::Temperature) {
            25.0
        } else {
            12.345
        };
        for i in 0..units.len() {
            for j in 0..units.len() {
                let forward = convert_unit(dimension, probe, i, j).unwrap();
                let back = convert_unit(dimension, forward, j, i).unwrap();
                assert!(
                    (back - probe).abs() <= 1e-9 * probe.abs().max(1.0),
                    "{} → {}: {probe} → {forward} → {back}",
                    units[i].code,
                    units[j].code
                );
            }
        }
    }
    assert_eq!(count, 22);
}

#[test]
fn uncertainty_dof_scale_invariance_and_student_t_monotonicity() {
    let base = combine_dof(
        &[
            (0.3, DegreesOfFreedom::Finite(4.0)),
            (0.4, DegreesOfFreedom::Finite(15.0)),
            (0.5, DegreesOfFreedom::Infinite),
        ],
        false,
    )
    .unwrap();
    let DegreesOfFreedom::Finite(base_v) = base else {
        panic!("expected finite dof");
    };
    for scale in [1e-120_f64, 1e-20, 1e20, 1e120] {
        let scaled = combine_dof(
            &[
                (0.3 * scale, DegreesOfFreedom::Finite(4.0)),
                (0.4 * scale, DegreesOfFreedom::Finite(15.0)),
                (0.5 * scale, DegreesOfFreedom::Infinite),
            ],
            false,
        )
        .unwrap();
        let DegreesOfFreedom::Finite(v) = scaled else {
            panic!("expected finite dof for scale {scale}");
        };
        assert!((v - base_v).abs() < 1e-11 * base_v, "scale={scale}: {v} != {base_v}");
    }
    let dofs = [0.9, 1.0, 2.0, 3.0, 5.0, 10.0, 30.0, 100.0, 1000.0, 1e6];
    let mut prev95 = f64::INFINITY;
    let mut prev99 = f64::INFINITY;
    for &v in &dofs {
        let q95 = student_t_quantile(DegreesOfFreedom::Finite(v), 0.95).unwrap();
        let q99 = student_t_quantile(DegreesOfFreedom::Finite(v), 0.99).unwrap();
        assert!(q99 > q95, "ν={v}: t99={q99} should exceed t95={q95}");
        assert!(q95 < prev95 && q99 < prev99, "t_p should decrease strictly with ν={v}");
        prev95 = q95;
        prev99 = q99;
    }
    assert!(prev95 > student_t_quantile(DegreesOfFreedom::Infinite, 0.95).unwrap());
    assert!(prev99 > student_t_quantile(DegreesOfFreedom::Infinite, 0.99).unwrap());
}
