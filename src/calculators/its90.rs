//! 工业热电偶、三点 CVD 拟合及 β 热敏电阻页面。
//! 输入温标在页面边界转为 °C，底层曲线和等级公式不直接使用显示温标。
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use crate::{INK, MUTED};
use eframe::egui::{self, RichText};

use crate::calculators::math::{cvd_basis, cvd_resistance, solve_3x3};
use crate::calculators::thermistor::BetaThermistor;
use crate::calculators::thermocouple::{corrected_temperature, measured_emf, thermocouple_emf};

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
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(RichText::new("热电偶参数").size(14.0).strong().color(INK));
            ui.add_space(7.0);
            let types = [
                "B 型", "E 型", "J 型", "K 型", "N 型", "R 型", "S 型", "T 型", "C 型", "D 型",
                "G 型", "L 型", "LR 型", "U 型", "A 型",
            ];
            let type_index = self.select_index(ui, tool, "type", "分度号", &types, 3);
            let type_code = [
                "B", "E", "J", "K", "N", "R", "S", "T", "C", "D", "G", "L", "LR", "U", "A",
            ][type_index];
            let unit = crate::calculators::calibration::TemperatureUnit(self.select_index(
                ui,
                tool,
                "tc.unit",
                "温度单位",
                &["°C", "K", "°F"],
                0,
            ));
            let mode = self.select_index(
                ui,
                tool,
                "mode",
                "计算方式",
                &["温度 → 热电势", "热电势 → 温度"],
                0,
            );
            let input_key = if mode == 0 {
                if unit.0 == 0 {
                    "temperature".to_owned()
                } else {
                    format!("temperature.{}", unit.0)
                }
            } else {
                format!("emf.{type_code}")
            };
            let input_default = if mode == 0 {
                unit.display(100.0).to_string()
            } else {
                thermocouple_emf(type_code, 100.0)
                    .unwrap_or(0.0)
                    .to_string()
            };
            if mode == 0 {
                self.input(
                    ui,
                    tool,
                    &input_key,
                    &format!("热端温度 ({})", unit.symbol()),
                    &input_default,
                    "所选分度号的有效范围内",
                );
            } else {
                self.input(
                    ui,
                    tool,
                    &input_key,
                    "测得热电势 (mV)",
                    &input_default,
                    "参考端温度见下方设置",
                );
            }
            let cold_key = if unit.0 == 0 {
                "cold".into()
            } else {
                format!("cold.{}", unit.0)
            };
            let cold_default = unit.display(0.0).to_string();
            self.input(
                ui,
                tool,
                &cold_key,
                &format!("参考端温度 ({})", unit.symbol()),
                &cold_default,
                "通常为 0 °C",
            );
            let mut calculated_temperature = None;
            let result = (|| {
                let input = self.number(tool, &input_key, &input_default)?;
                let input = if mode == 0 {
                    unit.celsius(input)
                } else {
                    input
                };
                let cold = unit.celsius(self.number(tool, &cold_key, &cold_default)?);
                if mode == 0 {
                    let cold_emf = thermocouple_emf(type_code, cold)?;
                    let emf = measured_emf(type_code, input, cold, 0.0)?;
                    calculated_temperature = Some(input);
                    Ok(vec![
                        ("测得热电势", format!("{} mV", format_number(emf))),
                        (
                            "热端温度",
                            format!("{} {}", format_number(unit.display(input)), unit.symbol()),
                        ),
                        ("参考端电势", format!("{} mV", format_number(cold_emf))),
                    ])
                } else {
                    let (temperature, hot_emf) =
                        corrected_temperature(type_code, input, cold, 0.0)?;
                    calculated_temperature = Some(temperature);
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
                            format!("{} {}", format_number(unit.display(cold)), unit.symbol()),
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
                            format!("{} mV/{}", format_number(s / unit.span()), unit.symbol()),
                        )],
                    );
                }
                ui.add_space(14.0);
                ui.label(RichText::new("等级允差 · IEC 60584-1").size(14.0).strong());
                if crate::calculators::thermocouple::tolerance::calculate(type_code, t).is_empty() {
                    ui.label("此分度号未内置等级允差数据。");
                }
                for tolerance in
                    crate::calculators::thermocouple::tolerance::calculate(type_code, t)
                {
                    ui.group(|ui| {
                        ui.label(RichText::new(tolerance.name).strong());
                        if let Some(d) = tolerance.delta {
                            ui.label(format!(
                                "±{} {}",
                                format_number(d * unit.span()),
                                unit.symbol()
                            ));
                            ui.label(format!(
                                "温度界限：{} ～ {} {}",
                                format_number(unit.display(t - d)),
                                format_number(unit.display(t + d)),
                                unit.symbol()
                            ));
                            if let (Ok(a), Ok(b)) = (
                                thermocouple_emf(type_code, t - d),
                                thermocouple_emf(type_code, t + d),
                            ) {
                                ui.label(format!(
                                    "热电势界限（参考端 0 °C）：{} ～ {} mV",
                                    format_number(a),
                                    format_number(b)
                                ));
                            }
                        } else {
                            ui.label("当前温度不在此等级适用范围内");
                        }
                        ui.label(tolerance.formula);
                        ui.label(format!(
                            "适用范围：{} ～ {} °C",
                            format_number(tolerance.min),
                            format_number(tolerance.max)
                        ));
                    });
                    ui.add_space(6.0);
                }
            }
            if let (Ok(bounds), Ok(cold)) = (
                crate::calculators::thermocouple::bounds(type_code),
                self.number(tool, &cold_key, &cold_default),
            ) {
                let cold = unit.celsius(cold);
                self.render_division_table(ui, tool, type_code, bounds, unit, "mV", |t| {
                    measured_emf(type_code, t, cold, 0.0)
                });
            }
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_cvd_fit(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(
                RichText::new("标准铂电阻系数拟合")
                    .size(14.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "输入 R₀ 与三个校准温度点的实测电阻，拟合 CVD 系数。至少一个点需低于 0 °C。",
                )
                .size(11.5)
                .color(MUTED),
            );
            self.input(ui, tool, "r0", "R₀ (Ω)", "100", "0 °C 时的电阻");
            self.input(ui, tool, "t1", "温度点 T₁ (°C)", "-100", "建议包含负温点");
            self.input(ui, tool, "r1", "实测电阻 R₁ (Ω)", "60.25584", "与 T₁ 对应");
            self.input(ui, tool, "t2", "温度点 T₂ (°C)", "100", "与 R₂ 对应");
            self.input(ui, tool, "r2", "实测电阻 R₂ (Ω)", "138.5055", "与 T₂ 对应");
            self.input(ui, tool, "t3", "温度点 T₃ (°C)", "200", "与 R₃ 对应");
            self.input(ui, tool, "r3", "实测电阻 R₃ (Ω)", "175.8560", "与 T₃ 对应");
            let result = (|| {
                let r0 = self.number(tool, "r0", "100")?;
                if r0 <= 0.0 {
                    return Err("R₀ 必须大于 0".to_owned());
                }
                let points = [
                    (
                        self.number(tool, "t1", "-100")?,
                        self.number(tool, "r1", "60.25584")?,
                    ),
                    (
                        self.number(tool, "t2", "100")?,
                        self.number(tool, "r2", "138.5055")?,
                    ),
                    (
                        self.number(tool, "t3", "200")?,
                        self.number(tool, "r3", "175.8560")?,
                    ),
                ];
                if !points.iter().any(|(t, _)| *t < 0.0) {
                    return Err("三个温度点中至少要有一个低于 0 °C".to_owned());
                }
                if points
                    .iter()
                    .any(|(t, r)| !(-200.0..=850.0).contains(t) || *r <= 0.0)
                {
                    return Err("校准点温度须在 −200～850 °C 内，电阻必须大于 0".to_owned());
                }
                let rows: Vec<[f64; 3]> = points.iter().map(|(t, _)| cvd_basis(*t)).collect();
                let rhs: Vec<f64> = points.iter().map(|(_, r)| r / r0 - 1.0).collect();
                let [a, b, c] =
                    solve_3x3(rows, rhs).ok_or_else(|| "温度点组合无法确定三个系数".to_owned())?;
                let check = points
                    .iter()
                    .map(|(t, r)| (cvd_resistance(*t, r0, a, b, c) - r).abs())
                    .fold(0.0_f64, f64::max);
                crate::calculators::numerics::finite(&[a, b, c, check])?;
                Ok(vec![
                    ("系数 A", format_number(a)),
                    ("系数 B", format_number(b)),
                    ("系数 C", format_number(c)),
                    ("最大残差", format!("{} Ω", format_number(check))),
                ])
            })();
            show_result_or_error(ui, "拟合结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_beta_thermistor(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(RichText::new("热敏电阻参数").size(14.0).strong().color(INK));
            ui.add_space(7.0);
            let mode = self.select_index(
                ui,
                tool,
                "mode",
                "计算方式",
                &["电阻 → 温度", "温度 → 电阻"],
                0,
            );
            self.input(
                ui,
                tool,
                "r0",
                "标称电阻 R₀ (Ω)",
                "10000",
                "参考温度下的电阻",
            );
            self.input(ui, tool, "t0", "参考温度 T₀ (°C)", "25", "常用值为 25 °C");
            self.input(ui, tool, "beta", "β 值 (K)", "3950", "请查元件规格书");
            let input_key = if mode == 0 {
                "resistance"
            } else {
                "temperature"
            };
            let input_default = if mode == 0 { "10000" } else { "25" };
            if mode == 0 {
                self.input(
                    ui,
                    tool,
                    input_key,
                    "实测电阻 (Ω)",
                    "10000",
                    "请输入大于 0 的电阻",
                );
            } else {
                self.input(
                    ui,
                    tool,
                    input_key,
                    "目标温度 (°C)",
                    "25",
                    "绝对温度须大于 0 K",
                );
            }
            let result = (|| {
                let model = BetaThermistor::new(
                    self.number(tool, "r0", "10000")?,
                    self.number(tool, "t0", "25")?,
                    self.number(tool, "beta", "3950")?,
                )?;
                let input = self.number(tool, input_key, input_default)?;
                let (temperature, resistance) = if mode == 0 {
                    (model.temperature(input)?, input)
                } else {
                    (input, model.resistance(input)?)
                };
                Ok(vec![
                    ("温度", format!("{} °C", format_number(temperature))),
                    ("电阻", format!("{} Ω", format_number(resistance))),
                ])
            })();
            show_result_or_error(ui, "计算结果", result);
        });
        info_note(ui, tool.kind);
    }
}
