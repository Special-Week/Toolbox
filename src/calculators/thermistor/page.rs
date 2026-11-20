//! 热敏电阻页面：手动系数或校准点拟合，以及独立的 β 模型入口。
//! 系数编辑器折叠时仍读取已保存输入；显示温标不改变校准多项式的单位。
use super::{BetaThermistor, polynomial::PolynomialThermistor};
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef, theme};
use eframe::egui::{self, RichText};

impl ToolboxApp {
    pub(crate) fn render_thermistor(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        let family = panel(ui, |ui| {
            ui.set_width(ui.available_width());
            self.select_index(
                ui,
                tool,
                "ntc.model",
                "计算模型",
                &["多项式 / Steinhart–Hart", "β 模型"],
                0,
            )
        })
        .inner;
        ui.add_space(10.0);
        if family == 1 {
            self.render_beta_thermistor(ui, tool);
            return;
        }
        let mut table = None;
        let mut process = None;
        split_workspace(
            ui,
            0.44,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("多项式热敏电阻参数")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(10.0);
                    let sh = app.select_index(
                        ui,
                        tool,
                        "ntc.sh",
                        "多项式形式",
                        &["完整多项式", "Steinhart–Hart（去除二次项）"],
                        0,
                    ) == 1;
                    let order = app.select_segmented(
                        ui,
                        tool,
                        "ntc.order",
                        "多项式阶数",
                        &["3 阶", "4 阶", "5 阶", "6 阶"],
                        0,
                    ) + 3;
                    let source = app.select_index(
                        ui,
                        tool,
                        "ntc.source",
                        "系数设置",
                        &["手动输入系数", "校准点最小二乘拟合"],
                        0,
                    );
                    let mut residuals = Vec::new();
                    let mut fitted_points = Vec::new();
                    let model = if source == 0 {
                        app.read_thermistor_coefficients(ui, tool, order, sh)?
                    } else {
                        let count = app.dynamic_count(
                            ui,
                            tool,
                            "ntc.points",
                            "校准点",
                            7,
                            if sh { order } else { order + 1 },
                            30,
                        );
                        let beta = BetaThermistor::new(10000.0, 25.0, 3950.0)?;
                        let mut points = Vec::new();
                        for i in 0..count {
                            let t = -20.0 + i as f64 * 20.0;
                            let tk = format!("ntc.point.{i}.t");
                            let rk = format!("ntc.point.{i}.r");
                            let default_r = beta.resistance(t)?.to_string();
                            app.input(
                                ui,
                                tool,
                                &tk,
                                &format!("校准点 {} · 温度 (°C)", i + 1),
                                &t.to_string(),
                                "证书温度",
                            );
                            app.input(
                                ui,
                                tool,
                                &rk,
                                &format!("校准点 {} · 电阻 (Ω)", i + 1),
                                &default_r,
                                "证书电阻",
                            );
                            points.push((
                                app.number(tool, &tk, &t.to_string())?,
                                app.number(tool, &rk, &default_r)?,
                            ));
                        }
                        let model = PolynomialThermistor::fit(&points, order, sh)?;
                        for (i, (t, r)) in points.iter().enumerate() {
                            residuals.push((
                                format!("点 {} 温度残差", i + 1),
                                format!("{} °C", format_number(t - model.temperature(*r)?)),
                            ));
                        }
                        fitted_points = points;
                        model
                    };
                    let unit = TemperatureUnit(app.select_segmented(
                        ui,
                        tool,
                        "ntc.unit",
                        "温度单位",
                        &["°C", "K", "°F"],
                        0,
                    ));
                    let direction = app.select_segmented(
                        ui,
                        tool,
                        "mode",
                        "计算方式",
                        &["电阻 → 温度", "温度 → 电阻"],
                        0,
                    );
                    let key = if direction == 0 {
                        "resistance".into()
                    } else {
                        format!("ntc.temperature.{}", unit.0)
                    };
                    let default = if direction == 0 {
                        "10000".into()
                    } else {
                        unit.display(25.0).to_string()
                    };
                    app.input(
                        ui,
                        tool,
                        &key,
                        if direction == 0 {
                            "电阻 (Ω)"
                        } else {
                            unit.symbol()
                        },
                        &default,
                        "输入待换算值",
                    );
                    let input = app.number(tool, &key, &default)?;
                    let (t, r) = if direction == 0 {
                        (model.temperature(input)?, input)
                    } else {
                        let t = unit.celsius(input);
                        (t, model.resistance(t)?)
                    };
                    let upper_resistance = model.resistance(t + 1.0)?;
                    let lower_resistance = model.resistance(t - 1.0)?;
                    let slope = (upper_resistance - lower_resistance) / 2.0;
                    app.input(
                        ui,
                        tool,
                        "ntc.range.min",
                        "校准温区下限 (°C)",
                        "-20",
                        "按证书填写",
                    );
                    app.input(
                        ui,
                        tool,
                        "ntc.range.max",
                        "校准温区上限 (°C)",
                        "100",
                        "按证书填写",
                    );
                    let min = app.number(tool, "ntc.range.min", "-20")?;
                    let max = app.number(tool, "ntc.range.max", "100")?;
                    if min >= max || t < min || t > max {
                        return Err("温度须处于所填校准温区内，温区下限须小于上限".into());
                    }
                    let mut steps = vec![
                        format!(
                            "{} 阶{}；{}；校准温区 {}～{} °C。",
                            order,
                            if sh {
                                " Steinhart–Hart 形式（二次项固定为零）"
                            } else {
                                "完整多项式"
                            },
                            if source == 0 {
                                "手动输入系数"
                            } else {
                                "校准点最小二乘拟合系数"
                            },
                            format_process_number(min),
                            format_process_number(max)
                        ),
                        format!(
                            "温度 = {} °C = {} K；电阻 R = {} Ω。",
                            if direction == 0 {
                                format_process_computed(t)
                            } else {
                                format_process_number(t)
                            },
                            format_process_computed(t + 273.15),
                            if direction == 0 {
                                format_process_number(r)
                            } else {
                                format_process_computed(r)
                            }
                        ),
                    ];
                    steps.push(temperature_conversion_note(unit.symbol()).to_owned());
                    if source == 1 {
                        steps.push("拟合 A 时使用 x=ln(R)、y=1/T；拟合 B 时使用 x=1/T、y=ln(R)。分别最小化各自的平方残差，T 用 K、R 用 Ω；采用列缩放与 QR 求解。".to_owned());
                        if sh {
                            steps.push("拟合时二次项系数固定为 0，不作为未知量。".to_owned());
                        }
                        for (i, (point_t, point_r)) in fitted_points.iter().enumerate() {
                            steps.push(String::new());
                            steps.push(format!("校准点 {}：t = {} °C，R = {} Ω，ln(R) = {}，1/T = {} K⁻¹。", i + 1, format_process_number(*point_t), format_process_number(*point_r), format_process_computed(point_r.ln()), format_process_computed(1.0 / (*point_t + 273.15))));
                        }
                    }
                    let (prefix, coefficients, x) = if direction == 0 {
                        steps.push("电阻 → 温度：1/T = Σ A[i]·ln(R)ⁱ；T 用 K，R 用 Ω。".to_owned());
                        ("A", &model.a, r.ln())
                    } else {
                        steps.push(
                            "温度 → 电阻：ln(R) = Σ B[i]·(1/T)ⁱ；T 用 K，R 用 Ω。".to_owned(),
                        );
                        ("B", &model.b, 1.0 / (t + 273.15))
                    };
                    steps.extend(coefficients.iter().enumerate().map(|(i, value)| {
                        format!("{prefix}[{i}] = {}", format_process_number(*value))
                    }));
                    let polynomial = crate::calculators::calibration::polynomial(coefficients, x);
                    if direction == 0 {
                        steps.push(format!(
                            "ln(R) = {}；多项式得到 1/T = {} K⁻¹，反算温度 = {} {}。",
                            format_process_computed(x),
                            format_process_computed(polynomial),
                            format_process_computed(unit.display(t)),
                            unit.symbol()
                        ));
                    } else {
                        steps.push(format!(
                            "1/T = {} K⁻¹；多项式得到 ln(R) = {}，取指数得到 R = {} Ω。",
                            format_process_computed(x),
                            format_process_computed(polynomial),
                            format_process_computed(r)
                        ));
                    }
                    steps.push("电阻变化率使用温度 → 电阻的 B 系数曲线；A、B 独立拟合或输入，可能不是精确的互逆函数。".to_owned());
                    if direction == 0 {
                        steps.extend(model.b.iter().enumerate().map(|(i, value)| {
                            format!("变化率使用 B[{i}] = {}", format_process_number(*value))
                        }));
                    }
                    if let Ok(at_temperature) = model.resistance(t) {
                        steps.push(format!(
                            "B 曲线在当前温度的电阻 = {} Ω。",
                            format_process_computed(at_temperature)
                        ));
                    }
                    steps.push(format!(
                        "B 曲线采用以 t 为中心、间隔 1 °C 的差分：dR/dt ≈ [R(t+1)−R(t−1)]/2 = ({} − {}) / 2 = {} Ω/°C；按显示温标换算后 = {} Ω/{}。",
                        format_process_computed(upper_resistance),
                        format_process_computed(lower_resistance),
                        format_process_computed(slope),
                        format_process_computed(slope / unit.span()),
                        unit.symbol()
                    ));
                    if t - 1.0 < min || t + 1.0 > max {
                        steps.push("差分点超出所填校准温区；此变化率包含温区外的曲线外推。".to_owned());
                    }
                    let fit_coeff = if source == 1 {
                        let mut c = Vec::new();
                        for (name, values) in [("A", &model.a), ("B", &model.b)] {
                            for (i, coefficient) in values.iter().enumerate() {
                                c.push((format!("{name}[{i}]"), format_process_number(*coefficient)));
                            }
                        }
                        Some((c, residuals))
                    } else {
                        None
                    };
                    let metrics = vec![
                        (
                            "温度",
                            format!("{} {}", format_number(unit.display(t)), unit.symbol()),
                        ),
                        ("电阻", format!("{} Ω", format_number(r))),
                        (
                            "电阻变化率",
                            format!("{} Ω/{}", format_number(slope / unit.span()), unit.symbol()),
                        ),
                    ];
                    Ok::<_, String>((metrics, steps, (model, unit, min, max), fit_coeff))
                })
                .inner
            },
            |ui, _app, result| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match result {
                        Ok((metrics, steps, tbl, fit_coeff)) => {
                            render_metrics(ui, "换算结果", &metrics);
                            if let Some((coefficients, residuals)) = fit_coeff {
                                ui.add_space(10.0);
                                ui.collapsing("拟合系数与校准点残差", |ui| {
                                    render_metrics(
                                        ui,
                                        "拟合系数",
                                        &coefficients
                                            .iter()
                                            .map(|(a, b)| (a.as_str(), b.clone()))
                                            .collect::<Vec<_>>(),
                                    );
                                    render_metrics(
                                        ui,
                                        "残差 = 校准温度 − 电阻反算温度",
                                        &residuals
                                            .iter()
                                            .map(|(a, b)| (a.as_str(), b.clone()))
                                            .collect::<Vec<_>>(),
                                    );
                                });
                            }
                            process = Some(steps);
                            table = Some(tbl);
                        }
                        Err(error) => render_error_message(ui, &error),
                    }
                });
            },
        );
        if let Some((m, u, min, max)) = table {
            self.render_division_table(ui, tool, "ntc", (min, max), u, "Ω", |t| m.resistance(t));
        }
        if let Some(lines) = &process {
            calculation_process(ui, "thermistor_polynomial_process", lines);
        }
        self.calculation_note(ui, tool);
    }

    /// 编辑器是否展开只影响绘制，所有生效系数始终从状态中读取。
    fn read_thermistor_coefficients(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        order: usize,
        sh: bool,
    ) -> Result<PolynomialThermistor, String> {
        ui.label("请填写实际证书系数；温度用 K，电阻用 Ω。");
        collapsible_note(
            ui,
            (tool.id, "coefficient_info"),
            "系数与公式说明",
            |ui| {
                ui.add(
                    egui::Label::new("A：1/T = Σ A[i]·ln(R)^i；B：ln(R) = Σ B[i]·(1/T)^i").wrap(),
                );
                ui.add(egui::Label::new("示例系数对应 10 kΩ、25 °C、β=3950 K；实际使用请填写证书系数。T 用 K，R 用 Ω。").wrap());
            },
        );
        let a0 = 1.0 / 298.15 - 10000.0_f64.ln() / 3950.0;
        let b0 = 10000.0_f64.ln() - 3950.0 / 298.15;
        let mut a = vec![0.0; order + 1];
        let mut b = a.clone();
        for (prefix, coefficients, defaults) in [
            ("a", &mut a, [a0, 1.0 / 3950.0]),
            ("b", &mut b, [b0, 3950.0]),
        ] {
            ui.collapsing(
                if prefix == "a" {
                    "电阻 → 温度系数 A"
                } else {
                    "温度 → 电阻系数 B"
                },
                |ui| {
                    for (i, item) in coefficients.iter_mut().enumerate() {
                        if sh && i == 2 {
                            continue;
                        }
                        let key = format!("ntc.{prefix}.{order}.{sh}.{i}");
                        let default = defaults.get(i).copied().unwrap_or(0.0).to_string();
                        self.input(
                            ui,
                            tool,
                            &key,
                            &format!("{}[{i}]", prefix.to_uppercase()),
                            &default,
                            "证书系数，按升幂排列",
                        );
                        *item = self.number(tool, &key, &default)?;
                    }
                    Ok::<_, String>(())
                },
            )
            .body_returned
            .unwrap_or(Ok(()))?;
            for (i, item) in coefficients.iter_mut().enumerate() {
                if !sh || i != 2 {
                    *item = self.number(
                        tool,
                        &format!("ntc.{prefix}.{order}.{sh}.{i}"),
                        &defaults.get(i).copied().unwrap_or(0.0).to_string(),
                    )?;
                }
            }
        }
        Ok(PolynomialThermistor { a, b })
    }
}
