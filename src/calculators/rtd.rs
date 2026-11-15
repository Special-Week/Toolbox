//! 工业热电阻页面；按分度号隔离 R₀ 与电阻输入，等级范围和曲线范围分别检查。
mod curves;
mod model;
mod tolerance;

use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use crate::{BLUE, INK, MUTED};
use eframe::egui::{self, RichText};
use model::{GRADUATIONS, RtdModel, TemperatureUnit, ToleranceKind};

impl ToolboxApp {
    pub(crate) fn render_industrial_rtd(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(ui.available_width().min(820.0));
        let parameters = panel(ui, |ui| {
            section_title(ui, "热电阻参数");
            let selected = self.select_index(
                ui,
                tool,
                "rtd.graduation",
                "分度号",
                &GRADUATIONS.iter().map(|g| g.name).collect::<Vec<_>>(),
                0,
            );
            let graduation = &GRADUATIONS[selected];
            let direction = self.select_index(
                ui,
                tool,
                "rtd.direction",
                "计算方式",
                &["温度 → 电阻", "电阻 → 温度"],
                0,
            );
            let unit_index = self.select_index(
                ui,
                tool,
                "rtd.unit",
                "温度单位",
                &["摄氏度 °C", "华氏度 °F", "开尔文 K"],
                0,
            );
            let unit = [
                TemperatureUnit::Celsius,
                TemperatureUnit::Fahrenheit,
                TemperatureUnit::Kelvin,
            ][unit_index];
            ui.add_space(5.0);
            ui.label(
                RichText::new(format!(
                    "{}  ·  换算范围 {}～{} {}  ·  标称 R₀ = {} Ω",
                    graduation.name,
                    format_number(unit.display_temperature(graduation.min)),
                    format_number(unit.display_temperature(graduation.max)),
                    unit.symbol(),
                    format_number(graduation.r0),
                ))
                .size(12.0)
                .color(MUTED),
            );
            if graduation.key == "cu10_427" {
                ui.label(
                    RichText::new("Cu10 的 10 Ω 为约 25 °C 时的标称值，0 °C 的 R₀ 为 9.035 Ω。")
                        .size(12.0)
                        .color(MUTED),
                );
            }
            ui.add_space(7.0);
            let r0_key = graduation.field_key("r0");
            let default_r0 = graduation.r0.to_string();
            self.input(ui, tool, &r0_key, "R₀ (Ω)", &default_r0, "0 °C 时的电阻");
            if graduation.custom() {
                self.input(
                    ui,
                    tool,
                    "rtd.cvd.a",
                    "系数 A (°C⁻¹)",
                    "0.0039083",
                    "例如 0.0039083",
                );
                self.input(
                    ui,
                    tool,
                    "rtd.cvd.b",
                    "系数 B (°C⁻²)",
                    "-0.0000005775",
                    "例如 -0.0000005775",
                );
                self.input(
                    ui,
                    tool,
                    "rtd.cvd.c",
                    "系数 C (°C⁻⁴)",
                    "-0.000000000004183",
                    "只用于负温区",
                );
            }
            let model = (|| {
                let coefficients = if graduation.custom() {
                    Some([
                        self.number(tool, "rtd.cvd.a", "0.0039083")?,
                        self.number(tool, "rtd.cvd.b", "-0.0000005775")?,
                        self.number(tool, "rtd.cvd.c", "-0.000000000004183")?,
                    ])
                } else {
                    None
                };
                RtdModel::new(
                    graduation,
                    self.number(tool, &r0_key, &default_r0)?,
                    coefficients,
                )
            })();
            // 温标、换算方向和分度号分别使用独立存储键，
            // 避免切换到 Pt1000 后沿用 Pt100 的 R0，或将温度误作电阻。
            let temperature_key = format!("rtd.temperature.{unit_index}");
            let resistance_key = graduation.field_key("resistance");
            let default_t = unit.display_temperature(100.0).to_string();
            let default_r = model
                .as_ref()
                .ok()
                .and_then(|m| m.resistance(100.0).ok())
                .unwrap_or(graduation.r0)
                .to_string();
            if direction == 0 {
                self.input(
                    ui,
                    tool,
                    &temperature_key,
                    &format!("温度 ({})", unit.symbol()),
                    &default_t,
                    "输入待换算温度",
                );
            } else {
                self.input(
                    ui,
                    tool,
                    &resistance_key,
                    "电阻 (Ω)",
                    &default_r,
                    "输入待换算电阻",
                );
            }
            let model = model?;
            let (temperature, resistance) = if direction == 0 {
                let mut t = unit.to_celsius(self.number(tool, &temperature_key, &default_t)?);
                // 单位偏移可能使量程端点产生几个末位精度单位的误差。
                for endpoint in [graduation.min, graduation.max] {
                    if (t - endpoint).abs() < 1e-10 {
                        t = endpoint;
                    }
                }
                (t, model.resistance(t)?)
            } else {
                let r = self.number(tool, &resistance_key, &default_r)?;
                (model.temperature(r)?, r)
            };
            let slope = model.slope(temperature)?;
            Ok::<_, String>((model, temperature, resistance, slope, unit, direction))
        })
        .inner;

        let mut process = None;
        ui.add_space(13.0);
        match parameters {
            Ok((model, temperature, resistance, slope, unit, direction)) => {
                render_metrics(
                    ui,
                    "换算结果",
                    &[
                        (
                            "温度",
                            format!(
                                "{} {}",
                                format_number(unit.display_temperature(temperature)),
                                unit.symbol()
                            ),
                        ),
                        ("电阻", format!("{} Ω", format_number(resistance))),
                        ("电阻比 R / R₀", format_number(resistance / model.r0)),
                        (
                            "电阻变化率 dR/dt",
                            format!(
                                "{} Ω/{}",
                                format_number(slope / unit.span_factor()),
                                unit.symbol()
                            ),
                        ),
                    ],
                );
                let mut steps = vec![
                    format!(
                        "分度号 {}；R₀ = {} Ω；曲线范围 {}～{} °C。",
                        model.graduation.name,
                        format_process_number(model.r0),
                        format_process_number(model.graduation.min),
                        format_process_number(model.graduation.max)
                    ),
                    format!(
                        "当前温度 = {} °C = {} {}；电阻 = {} Ω。",
                        if direction == 0 {
                            format_process_number(temperature)
                        } else {
                            format_process_computed(temperature)
                        },
                        if direction == 0 {
                            format_process_number(unit.display_temperature(temperature))
                        } else {
                            format_process_computed(unit.display_temperature(temperature))
                        },
                        unit.symbol(),
                        if direction == 0 {
                            format_process_computed(resistance)
                        } else {
                            format_process_number(resistance)
                        }
                    ),
                ];
                steps.push(temperature_conversion_note(unit.symbol()).to_owned());
                if let Some([a, b, c]) = model.cvd_coefficients() {
                    steps.push(format!(
                        "CVD 系数 A = {}，B = {}，C = {}。",
                        format_process_number(a),
                        format_process_number(b),
                        format_process_number(c)
                    ));
                    steps.push(
                        if temperature < 0.0 {
                            "R/R₀ = 1 + A·t + B·t² + C·(t−100)·t³，t 使用 °C。"
                        } else {
                            "R/R₀ = 1 + A·t + B·t²；非负温度不使用 C 项，t 使用 °C。"
                        }
                        .to_owned(),
                    );
                    steps.push(format!(
                        "代入 t = {} °C、R₀ = {} Ω；按上述系数计算 R₀ × f(t)。",
                        if direction == 0 {
                            format_process_number(temperature)
                        } else {
                            format_process_computed(temperature)
                        },
                        format_process_number(model.r0)
                    ));
                    if temperature < 0.0 {
                        steps.push(format!("R = {} × [1 + {}×({}) + {}×({})² + {}×({}−100)×({})³] Ω（程序按展开后的多项式求值）。", format_process_number(model.r0), format_process_number(a), if direction == 0 { format_process_number(temperature) } else { format_process_computed(temperature) }, format_process_number(b), if direction == 0 { format_process_number(temperature) } else { format_process_computed(temperature) }, format_process_number(c), if direction == 0 { format_process_number(temperature) } else { format_process_computed(temperature) }, if direction == 0 { format_process_number(temperature) } else { format_process_computed(temperature) }));
                    } else {
                        steps.push(format!(
                            "R = {} × [1 + {}×({}) + {}×({})²] Ω。",
                            format_process_number(model.r0),
                            format_process_number(a),
                            if direction == 0 {
                                format_process_number(temperature)
                            } else {
                                format_process_computed(temperature)
                            },
                            format_process_number(b),
                            if direction == 0 {
                                format_process_number(temperature)
                            } else {
                                format_process_computed(temperature)
                            }
                        ));
                    }
                } else if let Some(form) = model.polynomial_form(temperature) {
                    steps.push(format!("分段参考曲线：{}。", form.segment));
                    if let Some((low, high)) = form.bounds {
                        steps.push(format!(
                            "当前分段边界 {}～{} °C；连接点按程序先匹配的分段处理。",
                            format_process_number(low),
                            format_process_number(high)
                        ));
                    }
                    steps.push(format!(
                        "x = t + {} = {}；R/R₀ = {} × Σ c[i]·xⁱ，t 用 °C。",
                        format_process_number(form.shift),
                        format_process_computed(form.argument),
                        format_process_number(form.scale)
                    ));
                    steps.extend(
                        form.coefficients()
                            .iter()
                            .enumerate()
                            .map(|(i, c)| format!("c[{i}] = {}", format_process_number(*c))),
                    );
                    steps.push(format!(
                        "多项式按秦九韶法得到 R/R₀ = {}；R = R₀ × (R/R₀)。",
                        format_process_computed(form.value_and_slope().0)
                    ));
                }
                if let Ok(calculated) = model.resistance(temperature) {
                    steps.push(format!(
                        "曲线正算电阻 = {} Ω{}。",
                        format_process_computed(calculated),
                        if direction == 1 {
                            "（反算校验）"
                        } else {
                            ""
                        }
                    ));
                }
                steps.push(format!(
                    "电阻比 R/R₀ = {} / {} = {}。",
                    if direction == 0 {
                        format_process_computed(resistance)
                    } else {
                        format_process_number(resistance)
                    },
                    format_process_number(model.r0),
                    format_process_computed(resistance / model.r0)
                ));
                steps.push(
                    if direction == 0 {
                        "将摄氏温度代入所选曲线，再乘以 R₀ 得到电阻。"
                    } else {
                        "在所选曲线有效温区内，以输入电阻通过有界二分法反算温度。"
                    }
                    .to_owned(),
                );
                steps.push(format!(
                    "曲线解析变化率 dR/dt = {} Ω/{}。",
                    format_process_computed(slope / unit.span_factor()),
                    unit.symbol()
                ));
                process = Some(steps);
                ui.add_space(13.0);
                self.render_rtd_tolerances(ui, tool, &model, temperature, unit);
                let table_unit = crate::calculators::calibration::TemperatureUnit(match unit {
                    TemperatureUnit::Celsius => 0,
                    TemperatureUnit::Kelvin => 1,
                    TemperatureUnit::Fahrenheit => 2,
                });
                self.render_division_table(
                    ui,
                    tool,
                    model.graduation.key,
                    (model.graduation.min, model.graduation.max),
                    table_unit,
                    "Ω",
                    |t| model.resistance(t),
                );
            }
            Err(error) => {
                render_error_message(ui, &error);
            }
        }
        if let Some(lines) = &process {
            calculation_process(ui, "industrial_rtd_process", lines);
        }
        ui.add_space(12.0);
        info_note(ui, tool.kind);
    }

    fn render_rtd_tolerances(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        model: &RtdModel,
        temperature: f64,
        unit: TemperatureUnit,
    ) {
        panel(ui, |ui| {
            section_title(ui, "等级允差");
            let kind = model.graduation.tolerance;
            if kind == ToleranceKind::None {
                ui.label(RichText::new("该镍电阻分度号没有内置等级允差数据。").color(MUTED));
                return;
            }
            let film = if matches!(
                kind,
                ToleranceKind::Platinum | ToleranceKind::PlatinumReference
            ) {
                self.select_index(
                    ui,
                    tool,
                    "rtd.element",
                    "元件结构",
                    &["绕线式", "薄膜式"],
                    0,
                ) == 1
            } else {
                false
            };
            if kind == ToleranceKind::PlatinumReference {
                ui.label(RichText::new("此分度号采用 AA / A / B / C 公式给出参考允差；IEC 60751 的标准特性为 385 系列。")
                    .size(12.0).color(MUTED));
            }
            ui.label(
                RichText::new(format!(
                    "当前温度：{} {}，温度允差为 ±Δt，电阻界限按曲线换算。",
                    format_number(unit.display_temperature(temperature)),
                    unit.symbol()
                ))
                .size(12.0)
                .color(MUTED),
            );
            ui.add_space(8.0);
            let allowances = tolerance::tolerances(kind, temperature, film);
            if ui.available_width() >= 650.0 {
                for row in allowances.chunks(2) {
                    ui.columns(2, |columns| {
                        for (column, allowance) in columns.iter_mut().zip(row) {
                            tolerance_card(column, allowance, model, temperature, unit);
                        }
                    });
                    ui.add_space(7.0);
                }
            } else {
                for allowance in &allowances {
                    tolerance_card(ui, allowance, model, temperature, unit);
                    ui.add_space(7.0);
                }
            }
        });
    }
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(14.0).strong().color(INK));
    ui.add_space(6.0);
}

fn tolerance_card(
    ui: &mut egui::Ui,
    allowance: &tolerance::Tolerance,
    model: &RtdModel,
    temperature: f64,
    unit: TemperatureUnit,
) {
    egui::Frame::new()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(223, 229, 239),
        ))
        .corner_radius(6.0)
        .inner_margin(11.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(allowance.name).size(13.0).strong().color(INK));
            ui.add_space(5.0);
            if let Some(delta) = allowance.value {
                ui.label(
                    RichText::new(format!(
                        "±{} {}",
                        format_number(delta * unit.span_factor()),
                        unit.symbol()
                    ))
                    .size(18.0)
                    .color(BLUE),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new(format!(
                        "温度界限  {}～{} {}",
                        format_number(unit.display_temperature(temperature - delta)),
                        format_number(unit.display_temperature(temperature + delta)),
                        unit.symbol()
                    ))
                    .size(12.0)
                    .color(INK),
                );
                if let (Ok(low), Ok(high)) = (
                    model.resistance(temperature - delta),
                    model.resistance(temperature + delta),
                ) {
                    ui.label(
                        RichText::new(format!(
                            "电阻界限  {}～{} Ω",
                            format_number(low),
                            format_number(high)
                        ))
                        .size(12.0)
                        .color(INK),
                    );
                } else {
                    ui.label(
                        RichText::new("电阻界限超出该分度号的换算范围")
                            .size(12.0)
                            .color(MUTED),
                    );
                }
            } else {
                ui.label(RichText::new("超出适用温度范围").size(15.0).color(MUTED));
                ui.add_space(5.0);
            }
            ui.add_space(6.0);
            ui.label(RichText::new(allowance.formula).size(11.5).color(MUTED));
            ui.label(
                RichText::new(format!(
                    "适用范围  {}～{} {}",
                    format_number(unit.display_temperature(allowance.min)),
                    format_number(unit.display_temperature(allowance.max)),
                    unit.symbol()
                ))
                .size(11.5)
                .color(MUTED),
            );
        });
}
