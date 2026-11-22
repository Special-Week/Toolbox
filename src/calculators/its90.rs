//! 工业热电偶、三点 CVD 拟合及 β 热敏电阻页面。
//! 输入温标在页面边界转为 °C，底层曲线和等级公式不直接使用显示温标。
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use crate::theme;
use eframe::egui::{self, CornerRadius, FontId, RichText, Stroke};

use crate::calculators::math::{cvd_basis, cvd_resistance, solve_3x3};
use crate::calculators::thermistor::BetaThermistor;
use crate::calculators::thermocouple::{
    WORKING_KINDS, corrected_temperature, measured_emf, thermocouple_emf,
};

impl ToolboxApp {
    pub(crate) fn render_thermocouple(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        standard: bool,
    ) {
        if standard {
            self.render_standard_thermocouple(ui, tool);
            return;
        }
        let p = theme::palette(ui.ctx());
        let mut process = None;
        let types = [
            "B 型", "E 型", "J 型", "K 型", "N 型", "R 型", "S 型", "T 型", "C 型", "D 型",
            "G 型", "L 型", "LR 型", "U 型", "A 型",
        ];
        struct TcParams {
            type_code: &'static str,
            unit: crate::calculators::calibration::TemperatureUnit,
            mode: usize,
            input_key: String,
            input_default: String,
            cold_key: String,
            cold_default: String,
        }

        let tc_params = split_workspace(
            ui,
            0.40,
            self,
            |ui, app| {
                let mut params = TcParams {
                    type_code: "K",
                    unit: crate::calculators::calibration::TemperatureUnit(0),
                    mode: 0,
                    input_key: String::new(),
                    input_default: String::new(),
                    cold_key: String::new(),
                    cold_default: String::new(),
                };
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("热电偶参数").size(14.5).strong().color(p.ink));
                    ui.add_space(10.0);
                    let type_index = app.select_index(ui, tool, "type", "分度号", &types, 3);
                    params.type_code = WORKING_KINDS[type_index];
                    params.unit = crate::calculators::calibration::TemperatureUnit(
                        app.select_segmented(
                            ui,
                            tool,
                            "tc.unit",
                            "温度单位",
                            &["°C", "K", "°F"],
                            0,
                        ),
                    );
                    params.mode = app.select_segmented(
                        ui,
                        tool,
                        "mode",
                        "计算方式",
                        &["温度 → 热电势", "热电势 → 温度"],
                        0,
                    );
                    params.input_key = if params.mode == 0 {
                        if params.unit.0 == 0 {
                            "temperature".to_owned()
                        } else {
                            format!("temperature.{}", params.unit.0)
                        }
                    } else {
                        format!("emf.{}", params.type_code)
                    };
                    params.input_default = if params.mode == 0 {
                        params.unit.display(100.0).to_string()
                    } else {
                        thermocouple_emf(params.type_code, 100.0)
                            .unwrap_or(0.0)
                            .to_string()
                    };
                    if params.mode == 0 {
                        app.input(
                            ui,
                            tool,
                            &params.input_key,
                            &format!("热端温度 ({})", params.unit.symbol()),
                            &params.input_default,
                            "所选分度号的有效范围内",
                        );
                    } else {
                        app.input(
                            ui,
                            tool,
                            &params.input_key,
                            "测得热电势 (mV)",
                            &params.input_default,
                            "参考端温度见下方设置",
                        );
                    }
                    params.cold_key = if params.unit.0 == 0 {
                        "cold".into()
                    } else {
                        format!("cold.{}", params.unit.0)
                    };
                    params.cold_default = params.unit.display(0.0).to_string();
                    app.input(
                        ui,
                        tool,
                        &params.cold_key,
                        &format!("参考端温度 ({})", params.unit.symbol()),
                        &params.cold_default,
                        "通常为 0 °C",
                    );
                });
                params
            },
            |ui, app, params| {
                let type_code = params.type_code;
                let unit = params.unit;
                let mode = params.mode;
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let mut calculated_temperature = None;
                    let result = (|| {
                        let bounds = crate::calculators::thermocouple::bounds(type_code)?;
                        let input = app.number(tool, &params.input_key, &params.input_default)?;
                        let input = if mode == 0 {
                            unit.celsius_in_bounds(input, bounds)
                        } else {
                            input
                        };
                        let cold = unit.celsius_in_bounds(
                            app.number(tool, &params.cold_key, &params.cold_default)?,
                            bounds,
                        );
                        if mode == 0 {
                            let cold_emf = thermocouple_emf(type_code, cold)?;
                            let emf = measured_emf(type_code, input, cold, 0.0)?;
                            let hot_emf = thermocouple_emf(type_code, input)?;
                            calculated_temperature = Some(input);
                            let mut steps = vec![
                                format!(
                                    "{} 型参考函数；热端温度 {} °C，参考端温度 {} °C。",
                                    type_code,
                                    format_process_number(input),
                                    format_process_number(cold)
                                ),
                                format!(
                                    "参考函数 E(热端) = {} mV，E(参考端) = {} mV。",
                                    format_process_computed(hot_emf),
                                    format_process_computed(cold_emf)
                                ),
                                format!(
                                    "测得热电势 = E(热端) − E(参考端) = {} − {} = {} mV。",
                                    format_process_computed(hot_emf),
                                    format_process_computed(cold_emf),
                                    format_process_computed(emf)
                                ),
                                format!(
                                    "热端温度显示为 {} {}。",
                                    format_process_number(unit.display(input)),
                                    unit.symbol()
                                ),
                            ];
                            steps.push(temperature_conversion_note(unit.symbol()).to_owned());
                            steps.push(String::new());
                            steps.extend(crate::calculators::thermocouple::reference_process(
                                type_code,
                                input,
                                "热端参考函数",
                                false,
                            )?);
                            steps.push(String::new());
                            steps.extend(crate::calculators::thermocouple::reference_process(
                                type_code,
                                cold,
                                "参考端参考函数",
                                false,
                            )?);
                            process = Some(steps);
                            Ok(vec![
                                ("测得热电势", format!("{} mV", format_number(emf))),
                                (
                                    "热端温度",
                                    format!(
                                        "{} {}",
                                        format_number(unit.display(input)),
                                        unit.symbol()
                                    ),
                                ),
                                ("参考端电势", format!("{} mV", format_number(cold_emf))),
                            ])
                        } else {
                            let (temperature, hot_emf) =
                                corrected_temperature(type_code, input, cold, 0.0)?;
                            let cold_emf = thermocouple_emf(type_code, cold)?;
                            calculated_temperature = Some(temperature);
                            let mut steps = vec![
                                format!(
                                    "{} 型参考函数；测得热电势 {} mV，参考端温度 {} °C。",
                                    type_code,
                                    format_process_number(input),
                                    format_process_number(cold)
                                ),
                                format!(
                                    "参考端电势 E(参考端) = {} mV。",
                                    format_process_computed(cold_emf)
                                ),
                                format!(
                                    "补偿后热电势 = 测得热电势 + E(参考端) = {} + {} = {} mV。",
                                    format_process_number(input),
                                    format_process_computed(cold_emf),
                                    format_process_computed(hot_emf)
                                ),
                                format!(
                                    "在所选分度号的有效范围内反算热端温度 = {} °C = {} {}。",
                                    format_process_computed(temperature),
                                    format_process_computed(unit.display(temperature)),
                                    unit.symbol()
                                ),
                            ];
                            steps.push(temperature_conversion_note(unit.symbol()).to_owned());
                            steps.push(String::new());
                            steps.extend(crate::calculators::thermocouple::reference_process(
                                type_code,
                                cold,
                                "参考端参考函数",
                                false,
                            )?);
                            steps.push(String::new());
                            steps.extend(crate::calculators::thermocouple::reference_process(
                                type_code,
                                temperature,
                                "反算热端的参考函数校验",
                                true,
                            )?);
                            process = Some(steps);
                            Ok(vec![
                                (
                                    "热端温度",
                                    format!(
                                        "{} {}",
                                        format_number(unit.display(temperature)),
                                        unit.symbol()
                                    ),
                                ),
                                ("补偿后热电势", format!("{} mV", format_number(hot_emf))),
                                (
                                    "参考端温度",
                                    format!(
                                        "{} {}",
                                        format_number(unit.display(cold)),
                                        unit.symbol()
                                    ),
                                ),
                            ])
                        }
                    })();
                    show_result_or_error(ui, "计算结果", result);
                    if let Some(t) = calculated_temperature {
                        if let Ok(s) = crate::calculators::thermocouple::slope(type_code, t) {
                            render_metrics(
                                ui,
                                "微分热电势",
                                &[(
                                    "dE/dt",
                                    format!(
                                        "{} mV/{}",
                                        format_number(s / unit.span()),
                                        unit.symbol()
                                    ),
                                )],
                            );
                        }
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("等级允差 · IEC 60584-1")
                                .size(13.5)
                                .strong()
                                .color(p.ink),
                        );
                        ui.add_space(6.0);
                        let tolerances =
                            crate::calculators::thermocouple::tolerance::calculate(type_code, t);
                        if tolerances.is_empty() {
                            ui.label(
                                RichText::new("此分度号未内置等级允差数据。")
                                    .size(12.5)
                                    .color(p.muted),
                            );
                        } else {
                            let cols = if ui.available_width() >= 480.0 { 2 } else { 1 };
                            for row in tolerances.chunks(cols) {
                                ui.columns(cols, |columns| {
                                    for (idx, tolerance) in row.iter().enumerate() {
                                        egui::Frame::new()
                                            .fill(p.surface_subtle)
                                            .stroke(Stroke::new(1.0_f32, p.border))
                                            .corner_radius(CornerRadius::same(10))
                                            .inner_margin(12)
                                            .show(&mut columns[idx], |ui| {
                                                ui.set_min_width(ui.available_width());
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(tolerance.name)
                                                            .size(13.0)
                                                            .strong()
                                                            .color(p.ink),
                                                    );
                                                    if let Some(d) = tolerance.delta {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "±{} {}",
                                                                format_number(d * unit.span()),
                                                                unit.symbol()
                                                            ))
                                                            .font(FontId::monospace(13.0))
                                                            .strong()
                                                            .color(p.accent),
                                                        );
                                                    }
                                                });
                                                ui.add_space(4.0);
                                                if let Some(d) = tolerance.delta {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "温度界限：{} ～ {} {}",
                                                            format_number(unit.display(t - d)),
                                                            format_number(unit.display(t + d)),
                                                            unit.symbol()
                                                        ))
                                                        .size(12.0)
                                                        .color(p.ink_secondary),
                                                    );
                                                    if let (Ok(a), Ok(b)) = (
                                                        thermocouple_emf(type_code, t - d),
                                                        thermocouple_emf(type_code, t + d),
                                                    ) {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "热电势界限（参考端 0 °C）：{} ～ {} mV",
                                                                format_number(a),
                                                                format_number(b)
                                                            ))
                                                            .size(11.5)
                                                            .color(p.muted),
                                                        );
                                                    }
                                                } else {
                                                    ui.label(
                                                        RichText::new("当前温度不在此等级适用范围内")
                                                            .size(12.0)
                                                            .color(p.muted),
                                                    );
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} · 范围 {}～{} °C",
                                                        tolerance.formula,
                                                        format_number(tolerance.min),
                                                        format_number(tolerance.max)
                                                    ))
                                                    .size(11.0)
                                                    .color(p.muted),
                                                );
                                            });
                                    }
                                });
                                ui.add_space(6.0);
                            }
                        }
                    }
                });
                params
            },
        );

        let type_code = tc_params.type_code;
        let unit = tc_params.unit;
        if let (Ok(bounds), Ok(cold)) = (
            crate::calculators::thermocouple::bounds(type_code),
            self.number(tool, &tc_params.cold_key, &tc_params.cold_default),
        ) {
            let cold = unit.celsius_in_bounds(cold, bounds);
            self.render_division_table(
                ui, tool, type_code, bounds, unit, "mV",
                |t| measured_emf(type_code, t, cold, 0.0), None,
            );
        }
        if let Some(lines) = &process {
            calculation_process(ui, "industrial_thermocouple_process", lines);
        }
        self.calculation_note(ui, tool);
    }

    pub(crate) fn render_cvd_fit(&mut self, ui: &mut egui::Ui, tool: &ToolDef) -> bool {
        let p = theme::palette(ui.ctx());
        let mut process = None;
        let valid = split_workspace(
            ui,
            0.44,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("标准铂电阻系数拟合")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "输入 R₀ 与三个校准温度点的实测电阻，拟合 CVD 系数。至少一个点需低于 0 °C。",
                        )
                        .size(11.5)
                        .color(p.muted),
                    );
                    ui.add_space(8.0);
                    app.input(ui, tool, "r0", "R₀ (Ω)", "100", "0 °C 时的电阻");
                    app.input(ui, tool, "t1", "温度点 T₁ (°C)", "-100", "建议包含负温点");
                    app.input(ui, tool, "r1", "实测电阻 R₁ (Ω)", "60.25584", "与 T₁ 对应");
                    app.input(ui, tool, "t2", "温度点 T₂ (°C)", "100", "与 R₂ 对应");
                    app.input(ui, tool, "r2", "实测电阻 R₂ (Ω)", "138.5055", "与 T₂ 对应");
                    app.input(ui, tool, "t3", "温度点 T₃ (°C)", "200", "与 R₃ 对应");
                    app.input(ui, tool, "r3", "实测电阻 R₃ (Ω)", "175.8560", "与 T₃ 对应");
                });
            },
            |ui, app, _| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let r0 = app.number(tool, "r0", "100")?;
                        if r0 <= 0.0 {
                            return Err("R₀ 必须大于 0".to_owned());
                        }
                        let points = [
                            (
                                app.number(tool, "t1", "-100")?,
                                app.number(tool, "r1", "60.25584")?,
                            ),
                            (
                                app.number(tool, "t2", "100")?,
                                app.number(tool, "r2", "138.5055")?,
                            ),
                            (
                                app.number(tool, "t3", "200")?,
                                app.number(tool, "r3", "175.8560")?,
                            ),
                        ];
                        if !points.iter().any(|(t, _)| *t < 0.0) {
                            return Err("三个温度点中至少要有一个低于 0 °C".to_owned());
                        }
                        if points
                            .iter()
                            .any(|(t, r)| !(-200.0..=850.0).contains(t) || *r <= 0.0)
                        {
                            return Err(
                                "校准点温度须在 −200～850 °C 内，电阻必须大于 0".to_owned()
                            );
                        }
                        let rows: Vec<[f64; 3]> = points.iter().map(|(t, _)| cvd_basis(*t)).collect();
                        let rhs: Vec<f64> = points.iter().map(|(_, r)| r / r0 - 1.0).collect();
                        let equations: Vec<_> = points.iter().zip(&rows).zip(&rhs).map(|(((t, r), row), y)| format!("校准点：t = {} °C，R = {} Ω；{}·A + {}·B + {}·C = R/R₀ − 1 = {}。", format_process_number(*t), format_process_number(*r), format_process_computed(row[0]), format_process_computed(row[1]), format_process_computed(row[2]), format_process_computed(*y))).collect();
                        let [a, b, c] = solve_3x3(rows, rhs)
                            .ok_or_else(|| "温度点组合无法确定三个系数".to_owned())?;
                        let check = points
                            .iter()
                            .map(|(t, r)| (cvd_resistance(*t, r0, a, b, c) - r).abs())
                            .fold(0.0_f64, f64::max);
                        crate::calculators::numerics::finite(&[a, b, c, check])?;
                        let mut steps = vec![format!("0 °C 电阻 R₀ = {} Ω。", format_process_number(r0)), "对每个校准点建立 R/R₀ − 1 = A·t + B·t² + C·(t−100)·t³；非负温度点的 C 项为 0。".to_owned()];
                        for equation in equations {
                            steps.push(String::new());
                            steps.push(equation);
                        }
                        steps.push("对以上三元方程逐列缩放，并使用选主元消元求解系数。".to_owned());
                        steps.push(format!("求解得到 A = {}，B = {}，C = {}；校准点最大电阻残差绝对值 = {} Ω。", format_process_number(a), format_process_number(b), format_process_number(c), format_process_computed(check)));
                        process = Some(steps);
                        Ok(vec![
                            ("系数 A", format_process_number(a)),
                            ("系数 B", format_process_number(b)),
                            ("系数 C", format_process_number(c)),
                            ("最大残差", format!("{} Ω", format_number(check))),
                        ])
                    })();
                    let valid = result.is_ok();
                    show_result_or_error(ui, "拟合结果", result);
                    valid
                })
                .inner
            },
        );
        if let Some(lines) = &process {
            calculation_process(ui, "cvd_fit_process", lines);
        }
        self.calculation_note(ui, tool);
        valid
    }

    pub(crate) fn render_beta_thermistor(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        let mut process = None;
        split_workspace(
            ui,
            0.42,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("热敏电阻参数").size(14.5).strong().color(p.ink));
                    ui.add_space(10.0);
                    let mode = app.select_segmented(
                        ui,
                        tool,
                        "mode",
                        "计算方式",
                        &["电阻 → 温度", "温度 → 电阻"],
                        0,
                    );
                    app.input(
                        ui,
                        tool,
                        "r0",
                        "标称电阻 R₀ (Ω)",
                        "10000",
                        "参考温度下的电阻",
                    );
                    app.input(ui, tool, "t0", "参考温度 T₀ (°C)", "25", "常用值为 25 °C");
                    app.input(ui, tool, "beta", "β 值 (K)", "3950", "请查元件规格书");
                    let input_key = if mode == 0 {
                        "resistance"
                    } else {
                        "temperature"
                    };
                    let input_default = if mode == 0 { "10000" } else { "25" };
                    if mode == 0 {
                        app.input(
                            ui,
                            tool,
                            input_key,
                            "实测电阻 (Ω)",
                            "10000",
                            "请输入大于 0 的电阻",
                        );
                    } else {
                        app.input(
                            ui,
                            tool,
                            input_key,
                            "目标温度 (°C)",
                            "25",
                            "绝对温度须大于 0 K",
                        );
                    }
                    (mode, input_key, input_default)
                })
                .inner
            },
            |ui, app, (mode, input_key, input_default)| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let r0 = app.number(tool, "r0", "10000")?;
                        let t0 = app.number(tool, "t0", "25")?;
                        let beta = app.number(tool, "beta", "3950")?;
                        let model = BetaThermistor::new(r0, t0, beta)?;
                        let input = app.number(tool, input_key, input_default)?;
                        let (temperature, resistance) = if mode == 0 {
                            (model.temperature(input)?, input)
                        } else {
                            (input, model.resistance(input)?)
                        };
                        let mut lines = vec![format!(
                            "R₀ = {} Ω；参考温度 T₀ = {} °C = {} K；β = {} K。",
                            format_process_number(r0),
                            format_process_number(t0),
                            format_process_computed(t0 + 273.15),
                            format_process_number(beta)
                        )];
                        if mode == 0 {
                            lines.push(format!(
                                "输入电阻 R = {} Ω；1/T = 1/T₀ + [ln(R) − ln(R₀)]/β。",
                                format_process_number(resistance)
                            ));
                            lines.push(format!(
                                "1/T = 1/{} + [ln({}) − ln({})]/{} = {} K⁻¹。",
                                format_process_computed(t0 + 273.15),
                                format_process_number(resistance),
                                format_process_number(r0),
                                format_process_number(beta),
                                format_process_computed(model.inverse_temperature(resistance))
                            ));
                            lines.push(format!(
                                "换算得到 T = {} K，温度 = {} °C。",
                                format_process_computed(temperature + 273.15),
                                format_process_computed(temperature)
                            ));
                        } else {
                            lines.push(format!(
                                "输入温度 = {} °C，T = {} K；ln(R) = ln(R₀) + β(1/T − 1/T₀)。",
                                format_process_number(temperature),
                                format_process_computed(temperature + 273.15)
                            ));
                            lines.push(format!(
                                "ln(R) = ln({}) + {} × (1/{} − 1/{}) = {}；R = exp[ln(R)]。",
                                format_process_number(r0),
                                format_process_number(beta),
                                format_process_computed(temperature + 273.15),
                                format_process_computed(t0 + 273.15),
                                format_process_computed(model.log_resistance(temperature + 273.15))
                            ));
                            lines.push(format!(
                                "按 β 模型计算电阻 R = {} Ω。",
                                format_process_computed(resistance)
                            ));
                        }
                        process = Some(lines);
                        Ok(vec![
                            ("温度", format!("{} °C", format_number(temperature))),
                            ("电阻", format!("{} Ω", format_number(resistance))),
                        ])
                    })();
                    show_result_or_error(ui, "计算结果", result);
                });
            },
        );
        if let Some(lines) = &process {
            calculation_process(ui, "thermistor_beta_process", lines);
        }
        self.calculation_note(ui, tool);
    }
}
