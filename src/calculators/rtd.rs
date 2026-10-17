//! 工业热电阻页面；按分度号隔离 R₀ 与电阻输入，等级范围和曲线范围分别检查。
mod curves;
mod model;
mod tolerance;

use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use crate::{BLUE, INK, MUTED, RED};
use eframe::egui::{self, RichText};
use model::{GRADUATIONS, RtdModel, TemperatureUnit, ToleranceKind};

impl ToolboxApp {
    pub(crate) fn render_industrial_rtd(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(820.0);
        let parameters = ui
            .group(|ui| {
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
                        RichText::new(
                            "Cu10 的 10 Ω 为约 25 °C 时的标称值，0 °C 的 R₀ 为 9.035 Ω。",
                        )
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
                // Units, directions and graduations have separate storage keys:
                // selecting Pt1000 cannot retain Pt100's R0 or a temperature as ohms.
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
                    // Unit offsets can introduce a few ULPs at range endpoints.
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
                Ok::<_, String>((model, temperature, resistance, slope, unit))
            })
            .inner;

        ui.add_space(13.0);
        match parameters {
            Ok((model, temperature, resistance, slope, unit)) => {
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
                ui.colored_label(RED, error);
            }
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
        ui.group(|ui| {
            section_title(ui, "等级允差");
            let kind = model.graduation.tolerance;
            if kind == ToleranceKind::None {
                ui.label(RichText::new("该镍电阻分度号没有内置等级允差数据。")
                    .color(MUTED));
                return;
            }
            let film = if matches!(kind, ToleranceKind::Platinum | ToleranceKind::PlatinumReference) {
                self.select_index(ui, tool, "rtd.element", "元件结构", &["绕线式", "薄膜式"], 0) == 1
            } else { false };
            if kind == ToleranceKind::PlatinumReference {
                ui.label(RichText::new("此分度号采用 AA / A / B / C 公式给出参考允差；IEC 60751 的标准特性为 385 系列。")
                    .size(12.0).color(MUTED));
            }
            ui.label(RichText::new(format!("当前温度：{} {}，温度允差为 ±Δt，电阻界限按曲线换算。",
                format_number(unit.display_temperature(temperature)), unit.symbol()))
                .size(12.0).color(MUTED));
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
