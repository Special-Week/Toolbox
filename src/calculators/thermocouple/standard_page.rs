//! 标准热电偶参数、校准拟合和参考端补偿界面；热电势一律使用 mV。
#![allow(clippy::items_after_test_module)]
use super::standard::{FixedMethod, StandardCouple, zones};
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
fn fixed_endpoint_process(
    model: &StandardCouple,
    t: f64,
    label: &str,
    calculated_temperature: bool,
) -> Result<Vec<String>, String> {
    let adjustment = model.fixed_adjustment(t)?;
    let shown_t = if calculated_temperature {
        format_process_computed(t)
    } else {
        format_process_number(t)
    };
    let mut steps = vec![format!("{label}固定点修正： t = {} °C。", shown_t.clone())];
    match adjustment.method {
        FixedMethod::SLinear => steps.push(format!("S 型低于 400 °C：修正量 = δ₁ × t / t₁ = {} × {} / {} = {} mV。", format_process_computed(adjustment.offsets[0]), shown_t.clone(), format_process_number(model.points[0].0), format_process_computed(adjustment.value))),
        FixedMethod::SQuadratic(terms) => {
            steps.push("S 型不低于 400 °C：修正量 = Σ δᵢ × ∏ⱼ≠ᵢ[(t−tⱼ)/(tᵢ−tⱼ)]；固定点和偏差如上。".to_owned());
            for (i, term) in terms.iter().enumerate() {
                steps.push(String::new());
                let mut substitution = format_process_computed(adjustment.offsets[i]);
                for (j, &(point, _)) in model.points.iter().enumerate() {
                    if i != j {
                        substitution.push_str(&format!(" × ({}−{})/({}−{})", shown_t, format_process_number(point), format_process_number(model.points[i].0), format_process_number(point)));
                    }
                }
                steps.push(format!("固定点 {} 的插值项 = {} = {} mV。", i + 1, substitution, format_process_computed(*term)));
            }
            steps.push(format!("各插值项相加，修正量 = {} mV。", format_process_computed(adjustment.value)));
        }
        FixedMethod::BZero => steps.push("B 型低于 23 °C：固定点修正量取 0 mV。".to_owned()),
        FixedMethod::BLinearFrom23 => steps.push(format!("B 型低温过渡：修正量 = δ₁ × (t−23) / (t₁−23) = {} × ({}−23) / ({}−23) = {} mV。", format_process_computed(adjustment.offsets[0]), shown_t.clone(), format_process_number(model.points[0].0), format_process_computed(adjustment.value))),
        FixedMethod::BConstant(index) => steps.push(format!("B 型当前温区使用固定点 {} 的偏差，修正量 = {} mV。", index + 1, format_process_computed(adjustment.value))),
        FixedMethod::BTransition { index, fraction } => steps.push(format!("B 型在固定点 {} 与 {} 间过渡：修正量 = δᵢ + (δᵢ₊₁−δᵢ) × (t−tᵢ−30)/40；δᵢ = {} mV，δᵢ₊₁ = {} mV，tᵢ = {} °C，t = {} °C，过渡比例 = {}，修正量 = {} mV。", index + 1, index + 2, format_process_computed(adjustment.offsets[index]), format_process_computed(adjustment.offsets[index + 1]), format_process_number(model.points[index].0), shown_t.clone(), format_process_computed(fraction), format_process_computed(adjustment.value))),
    }
    let reference = super::thermocouple_emf(model.kind, t)?;
    steps.extend(super::reference_process(
        model.kind,
        t,
        &format!("{label}参考函数"),
        calculated_temperature,
    )?);
    steps.push(format!(
        "{label}参考电势 {} mV + 固定点修正 {} mV = 模型电势 {} mV。",
        format_process_computed(reference),
        format_process_computed(adjustment.value),
        format_process_computed(model.emf(t)?)
    ));
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_endpoint_details_follow_each_actual_branch() {
        for (kind, points, cases) in [
            (
                "S",
                vec![419.527, 660.323, 1084.62],
                vec![
                    (-50.0, "S 型低于 400"),
                    (399.999, "S 型低于 400"),
                    (400.0, "S 型不低于 400"),
                    (1084.62, "S 型不低于 400"),
                ],
            ),
            (
                "B",
                vec![1100.0, 1200.0, 1300.0, 1400.0, 1500.0],
                vec![
                    (0.0, "B 型低于 23"),
                    (23.0, "B 型低温过渡"),
                    (799.999, "B 型低温过渡"),
                    (800.0, "固定点 1 的偏差"),
                    (1130.0, "固定点 1 的偏差"),
                    (1150.0, "固定点 1 与 2 间过渡"),
                    (1170.0, "固定点 2 的偏差"),
                    (1250.0, "固定点 2 与 3 间过渡"),
                    (1350.0, "固定点 3 与 4 间过渡"),
                    (1450.0, "固定点 4 与 5 间过渡"),
                    (1820.0, "固定点 5 的偏差"),
                ],
            ),
        ] {
            let model = StandardCouple {
                kind,
                points: points
                    .into_iter()
                    .enumerate()
                    .map(|(i, t)| {
                        (
                            t,
                            super::super::thermocouple_emf(kind, t).unwrap()
                                + (i + 1) as f64 * 0.01,
                        )
                    })
                    .collect(),
                zone: None,
            };
            for (temperature, branch) in cases {
                let lines = fixed_endpoint_process(&model, temperature, "参考端", false)
                    .unwrap()
                    .join("\n")
                    .replace('\u{2063}', "");
                let adjustment = model.fixed_adjustment(temperature).unwrap();
                let expected = format!(
                    "参考端参考电势 {} mV + 固定点修正 {} mV = 模型电势 {} mV。",
                    format_process_number(
                        super::super::thermocouple_emf(kind, temperature).unwrap()
                    ),
                    format_process_number(adjustment.value),
                    format_process_number(model.emf(temperature).unwrap())
                );
                assert!(lines.contains(branch), "{kind} {temperature}: {lines}");
                assert!(lines.contains(&expected), "{kind} {temperature}: {lines}");
                if let FixedMethod::SQuadratic(terms) = adjustment.method {
                    for (index, term) in terms.iter().enumerate() {
                        let line = lines
                            .lines()
                            .find(|line| {
                                line.starts_with(&format!("固定点 {} 的插值项", index + 1))
                            })
                            .unwrap();
                        assert_eq!(line.matches(" × (").count(), 2);
                        assert!(line.contains(&format!("= {} mV", format_process_number(*term))));
                    }
                }
            }
        }
    }
}

impl ToolboxApp {
    pub(crate) fn render_standard_thermocouple(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = crate::theme::palette(ui.ctx());
        let mut table = None;
        let mut process = None;

        let (archive_save, archive_required) = split_workspace(
            ui,
            0.46,
            self,
            |ui, app| {
                let archive_save = app.calibration_archive_controls(ui, tool);
                let mut archive_required = None;
                type CoupleFitSummary = (Vec<f64>, Vec<(f64, f64)>);
                let mut fit_summary: Option<CoupleFitSummary> = None;
                let result = panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new("标准热电偶参数")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(8.0);
                    let k = app.select_index(ui, tool, "stc.type", "分度号", &["S 型", "B 型"], 0);
                    let kind = ["S", "B"][k];
                    let mut zone_id = 0;
                    let mut fitted = false;
                    let mode = app.select_index(
                        ui,
                        tool,
                        "stc.calibration",
                        "校准方式",
                        &["固定点实测热电势", "温区多项式系数"],
                        0,
                    );
                    let mut required = vec!["stc.type".to_owned(), "stc.calibration".to_owned()];
                    let mut model = StandardCouple {
                        kind,
                        points: Vec::new(),
                        zone: None,
                    };
                    let mut fit_details = None;
                    if mode == 0 {
                        let defaults: &[(f64, f64)] = if k == 0 {
                            &[(419.527, 3.4472), (660.323, 5.8605), (1084.62, 10.5822)]
                        } else {
                            &[
                                (1100., 5.8029),
                                (1200., 6.8050),
                                (1300., 7.8809),
                                (1400., 8.9926),
                                (1500., 10.1205),
                            ]
                        };
                        for (i, &(t, e)) in defaults.iter().enumerate() {
                            let key = format!("stc.{kind}.cal.{i}");
                            required.push(key.clone());
                            let d = e.to_string();
                            app.input(
                                ui,
                                tool,
                                &key,
                                &format!("{} °C · 实测电势 (mV)", format_number(t)),
                                &d,
                                "证书中的实际电势，不是修正量",
                            );
                            model.points.push((t, app.number(tool, &key, &d)?));
                        }
                    } else {
                        let ranges = zones(kind);
                        let labels: Vec<_> = ranges
                            .iter()
                            .map(|(a, b, _)| {
                                format!("{} ～ {} °C", format_number(*a), format_number(*b))
                            })
                            .collect();
                        let z = app.select_index(
                            ui,
                            tool,
                            &format!("stc.{kind}.zone"),
                            "系数温区",
                            &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                            0,
                        );
                        zone_id = z;
                        required.push(format!("stc.{kind}.zone"));
                        let mut zone = ranges[z].clone();
                        let fit = app.select_index(
                            ui,
                            tool,
                            "stc.coefficient_source",
                            "系数设置",
                            &["手动输入系数", "校准点拟合"],
                            0,
                        ) == 1;
                        fitted = fit;
                        required.push("stc.coefficient_source".to_owned());
                        if fit {
                            let count =
                                app.dynamic_count(ui, tool, "stc.fit.points", "校准点", 3, 2, 30);
                            required.push("stc.fit.points.count".to_owned());
                            let mut points = Vec::new();
                            for i in 0..count {
                                let t = zone.0
                                    + (zone.1 - zone.0) * (i + 1) as f64 / (count + 1) as f64;
                                let e = crate::calculators::calibration::polynomial(&zone.2, t);
                                let key = format!("stc.fit.{kind}.{z}.{i}");
                                required.push(format!("{key}.t"));
                                required.push(format!("{key}.e"));
                                app.input(
                                    ui,
                                    tool,
                                    &format!("{key}.t"),
                                    &format!("点 {} 温度 (°C)", i + 1),
                                    &t.to_string(),
                                    "校准温度",
                                );
                                app.input(
                                    ui,
                                    tool,
                                    &format!("{key}.e"),
                                    &format!("点 {} 电势 (mV)", i + 1),
                                    &e.to_string(),
                                    "实际热电势",
                                );
                                points.push((
                                    app.number(tool, &format!("{key}.t"), &t.to_string())?,
                                    app.number(tool, &format!("{key}.e"), &e.to_string())?,
                                ));
                            }
                            let base_coefficients = zone.2.clone();
                            let fitted_delta =
                                StandardCouple::fit_zone_coefficients(&mut zone, &points)?;
                            fit_summary = Some((zone.2.clone(), points.clone()));
                            fit_details = Some((points, base_coefficients, fitted_delta));
                        } else {
                            for (i, c) in zone.2.iter_mut().enumerate() {
                                let key = format!("stc.{kind}.{z}.c.{i}");
                                required.push(key.clone());
                                let d = c.to_string();
                                app.input(
                                    ui,
                                    tool,
                                    &key,
                                    &format!("系数 c[{i}]"),
                                    &d,
                                    "E(t) = Σ c[i]·t^i；t 为 °C，E 为 mV",
                                );
                                *c = app.number(tool, &key, &d)?;
                            }
                        }
                        model.zone = Some(zone);
                    }
                    archive_required = Some(required);
                    let u = TemperatureUnit(app.select_segmented(
                        ui,
                        tool,
                        "stc.unit",
                        "温度单位",
                        &["°C", "K", "°F"],
                        0,
                    ));
                    let direction = app.select_segmented(
                        ui,
                        tool,
                        "stc.direction",
                        "计算方式",
                        &["温度 → 热电势", "热电势 → 温度"],
                        0,
                    );
                    let def_t = model
                        .zone
                        .as_ref()
                        .map(|(a, b, _)| (a + b) / 2.0)
                        .unwrap_or(if k == 0 { 1000.0 } else { 1200.0 });
                    let key = if direction == 0 {
                        format!("stc.{kind}.{mode}.{zone_id}.t.{}", u.0)
                    } else {
                        format!("stc.{kind}.{mode}.{zone_id}.emf")
                    };
                    let default = if direction == 0 {
                        u.display(def_t).to_string()
                    } else {
                        model.emf(def_t)?.to_string()
                    };
                    app.input(
                        ui,
                        tool,
                        &key,
                        if direction == 0 {
                            u.symbol()
                        } else {
                            "测得热电势 (mV)"
                        },
                        &default,
                        "输入待换算值",
                    );
                    let ck = format!("stc.cold.{}", u.0);
                    let cd = u.display(0.0).to_string();
                    app.input(
                        ui,
                        tool,
                        &ck,
                        &format!("参考端温度 ({})", u.symbol()),
                        &cd,
                        "多项式温区以外的参考端请使用 0 °C",
                    );
                    let (lo, hi) = model.bounds();
                    let cold = u.celsius_in_bounds(app.number(tool, &ck, &cd)?, (lo, hi));
                    let cold_e = model.reference_end_emf(cold)?;
                    let v = app.number(tool, &key, &default)?;
                    let (t, e, hot_e) = if direction == 0 {
                        let t = u.celsius_in_bounds(v, (lo, hi));
                        let hot_e = model.emf(t)?;
                        (t, hot_e - cold_e, hot_e)
                    } else {
                        let hot_e = v + cold_e;
                        (model.temperature(hot_e)?, v, hot_e)
                    };
                    let slope = model.slope(t)?;
                    let mut steps = vec![format!(
                        "{} 型标准热电偶；{}。",
                        kind,
                        if mode == 0 {
                            "固定点实测热电势"
                        } else if fitted {
                            "温区系数由校准点拟合"
                        } else {
                            "使用输入的温区系数"
                        }
                    )];
                    if let Some((min, max, coefficients)) = &model.zone {
                        steps.push(format!("系数温区 {}～{} °C；E(t) = Σ c[i]·tⁱ，t 用 °C、E 用 mV。参考端为 0 °C 时电势为 0 mV。", format_process_number(*min), format_process_number(*max)));
                        if let Some((points, base, delta)) = &fit_details {
                            steps.push("拟合使用基准多项式 E₀(t)=Σ b[i]·tⁱ。对各校准点，以 ΔE=实测电势−E₀(t) 建立 ΔE≈Δc₁·t+Δc₂·t²，并用最小二乘法求解。".to_owned());
                            steps.extend(base.iter().enumerate().map(|(i, c)| {
                                format!("基准 b[{i}] = {}", format_process_number(*c))
                            }));
                            for (i, (point_t, measured)) in points.iter().enumerate() {
                                steps.push(String::new());
                                let baseline =
                                    crate::calculators::calibration::polynomial(base, *point_t);
                                steps.push(format!("点 {}：t = {} °C，实测 = {} mV，E₀ = {} mV，ΔE = {} mV；方程系数 t = {}，t² = {}。", i + 1, format_process_number(*point_t), format_process_number(*measured), format_process_computed(baseline), format_process_computed(*measured - baseline), format_process_number(*point_t), format_process_computed(*point_t * *point_t)));
                            }
                            steps.push(format!("拟合得到 Δc₁ = {}、Δc₂ = {}；分别加到基准 c[1]、c[2]，其余系数保持基准值。", format_process_number(delta[0]), format_process_number(delta[1])));
                        }
                        steps.extend(coefficients.iter().enumerate().map(|(i, c)| {
                            format!("c[{i}] = {}", format_process_number(*c))
                        }));
                    } else {
                        steps.push("在参考曲线上叠加各固定点实测电势与参考电势的偏差。".to_owned());
                        steps.push(
                            if kind == "S" {
                                "固定点修正于 400 °C 从锌点线性延伸切换为三点二次插值；两段取决于校准值，不保证在切换点连续。"
                            } else {
                                "固定点修正：低于 23 °C 取零，23 °C 至低于 800 °C 按首点线性计算，800 °C 起使用首点偏差；两段在 800 °C 不保证连续。"
                            }
                            .to_owned(),
                        );
                        let adjustment = model.fixed_adjustment(t)?;
                        for ((point_t, measured), offset) in
                            model.points.iter().zip(&adjustment.offsets)
                        {
                            steps.push(String::new());
                            let reference = super::thermocouple_emf(kind, *point_t)?;
                            steps.push(format!(
                                "固定点 {} °C：实测 {} mV − 参考 {} mV = 偏差 {} mV。",
                                format_process_number(*point_t),
                                format_process_number(*measured),
                                format_process_computed(reference),
                                format_process_computed(*offset)
                            ));
                            steps.extend(super::reference_process(
                                kind,
                                *point_t,
                                "固定点参考函数",
                                false,
                            )?);
                        }
                        steps.push(String::new());
                        steps.extend(fixed_endpoint_process(&model, t, "热端", direction == 1)?);
                        steps.push(String::new());
                        steps.extend(fixed_endpoint_process(&model, cold, "参考端", false)?);
                    }
                    steps.push(temperature_conversion_note(u.symbol()).to_owned());
                    steps.push(format!(
                        "参考端温度 {} °C，参考端电势 E(参考端) = {} mV。",
                        format_process_number(cold),
                        format_process_computed(cold_e)
                    ));
                    if direction == 0 {
                        steps.push(format!(
                            "热端温度 {} °C；模型 E(热端) = {} mV。",
                            format_process_number(t),
                            format_process_computed(hot_e)
                        ));
                        steps.push(format!(
                            "测得热电势 = E(热端) − E(参考端) = {} − {} = {} mV。",
                            format_process_computed(hot_e),
                            format_process_computed(cold_e),
                            format_process_computed(e)
                        ));
                    } else {
                        steps.push(format!(
                            "补偿后电势 = 测得电势 + 参考端电势 = {} + {} = {} mV；在所选分支上反算温度。",
                            format_process_number(e),
                            format_process_computed(cold_e),
                            format_process_computed(hot_e)
                        ));
                        steps.push(format!(
                            "热端温度 = {} °C = {} {}。",
                            format_process_computed(t),
                            format_process_computed(u.display(t)),
                            u.symbol()
                        ));
                    }
                    steps.push(slope.map_or_else(
                        || "分段交界处的微分热电势未定义。".to_owned(),
                        |(a, b, value)| format!(
                            "在同一分段的 {}～{} °C 内作差分，得到微分热电势 {} mV/{}。",
                            format_process_number(a), format_process_number(b),
                            format_process_computed(value / u.span()), u.symbol()
                        ),
                    ));
                    Ok::<_, String>((
                        vec![
                            (
                                "热端温度",
                                format!("{} {}", format_number(u.display(t)), u.symbol()),
                            ),
                            ("测得热电势", format!("{} mV", format_number(e))),
                            (
                                "微分热电势 dE/dt",
                                slope.map_or_else(|| "未定义".to_owned(), |(_, _, value)| {
                                    format!("{} mV/{}", format_number(value / u.span()), u.symbol())
                                }),
                            ),
                            ("参考端电势", format!("{} mV", format_number(cold_e))),
                        ],
                        steps,
                        (model, u, lo, hi, cold_e, zone_id),
                    ))
                })
                .inner;
                (archive_save, archive_required, fit_summary, result)
            },
            |ui, _app, (archive_save, archive_required, fit_summary, result)| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match result {
                        Ok((metrics, steps, tbl)) => {
                            render_metrics(ui, "换算结果", &metrics);
                            if let Some((coeffs, points)) = fit_summary {
                                ui.add_space(10.0);
                                ui.collapsing("拟合系数与残差", |ui| {
                                    for (i, c) in coeffs.iter().enumerate() {
                                        ui.label(format!("c[{i}] = {}", format_process_number(*c)));
                                    }
                                    for (i, &(t, e)) in points.iter().enumerate() {
                                        ui.label(format!(
                                            "点 {} 电势残差：{} mV",
                                            i + 1,
                                            format_number(
                                                e - crate::calculators::calibration::polynomial(
                                                    &coeffs, t
                                                )
                                            )
                                        ));
                                    }
                                });
                            }
                            process = Some(steps);
                            table = Some(tbl);
                        }
                        Err(err) => render_error_message(ui, &err),
                    }
                });
                (archive_save, archive_required)
            },
        );

        self.finish_calibration_save(tool, archive_save, archive_required);
        if let Some((model, u, lo, hi, cold, z)) = table {
            self.render_division_table(
                ui,
                tool,
                &format!("standard.{}.{z}", model.kind),
                (lo, hi),
                u,
                "mV",
                |t| model.emf(t).map(|e| e - cold),
                Some(&|t| model.slope(t).map(|value| value.map(|(_, _, slope)| slope))),
            );
        }
        if let Some(lines) = &process {
            calculation_process(ui, "standard_thermocouple_process", lines);
        }
        self.calculation_note(ui, tool);
    }
}
