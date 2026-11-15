//! 三个辐射页面共用输入与结果布局；mode=0/1/2 对应耳温计、源误差、示值修正。
//! 有效波长法与 Planck 法的误差方向不同，结果标签和计算说明分别明确。
pub(crate) mod model;
mod process;
pub(crate) mod website;
use crate::INK;
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use eframe::egui::{self, RichText};
use model::{Response, Spectrum, ear_blackbody};

impl ToolboxApp {
    pub(crate) fn render_radiation(&mut self, ui: &mut egui::Ui, tool: &ToolDef, mode: u8) {
        panel(ui, |ui| {
            ui.set_max_width(ui.available_width().min(820.0));
            ui.label(
                RichText::new(if mode == 0 {
                    "黑体与探头参数"
                } else {
                    "辐射温度参数"
                })
                .size(14.0)
                .strong()
                .color(INK),
            );
            ui.add_space(6.0);
            if mode == 0 {
                self.select_index(
                    ui,
                    tool,
                    "ear.model",
                    "反射模型",
                    &["多次反射平衡", "一次反射近似"],
                    0,
                );
                self.input(ui, tool, "target", "黑体温度 (°C)", "35", "黑体真实温度");
                self.input(ui, tool, "emissivity", "黑体发射率", "0.9995", "0 < ε ≤ 1");
                self.input(
                    ui,
                    tool,
                    "probe_temperature",
                    "探头温度 (°C)",
                    "18",
                    "探头表面温度",
                );
                self.input(
                    ui,
                    tool,
                    "probe_emissivity",
                    "探头发射率",
                    "0.8",
                    "0 < ε ≤ 1",
                );
            } else {
                self.select_index(
                    ui,
                    tool,
                    "model",
                    "计算模型",
                    &["有效波长法", "Planck 辐射平衡"],
                    0,
                );
                let response = self.select_index(
                    ui,
                    tool,
                    "spectral_response",
                    "光谱响应",
                    &["单一有效波长", "8～14 µm 波段", "7～18 µm 波段"],
                    0,
                );
                if response == 0 {
                    self.input(
                        ui,
                        tool,
                        "wavelength",
                        "有效波长 (µm)",
                        "0.65",
                        "0.1～100 µm",
                    );
                }
                if mode == 2 {
                    self.input(
                        ui,
                        tool,
                        "indicated",
                        "温度计示值 (°C)",
                        "1000",
                        "仪表当前显示值",
                    );
                    self.input(
                        ui,
                        tool,
                        "instrument_emissivity",
                        "温度计设定发射率",
                        "0.95",
                        "仪表中设置的发射率",
                    );
                } else {
                    self.input(
                        ui,
                        tool,
                        "target",
                        "辐射源真实温度 (°C)",
                        "1000",
                        "辐射源温度",
                    );
                }
                self.input(ui, tool, "emissivity", "辐射源发射率", "0.99", "0 < ε ≤ 1");
                self.input(ui, tool, "ambient", "反射背景温度 (°C)", "20", "背景温度");
            }
            let mut process_lines = None;
            let result = (|| {
                if mode == 0 {
                    let target = self.number(tool, "target", "35")?;
                    let single = self.value(tool, "ear.model", "0") == "1";
                    let calc = if single {
                        model::ear_single_reflection
                    } else {
                        ear_blackbody
                    };
                    let emissivity = self.number(tool, "emissivity", "0.9995")?;
                    let probe = self.number(tool, "probe_temperature", "18")?;
                    let probe_emissivity = self.number(tool, "probe_emissivity", "0.8")?;
                    let light = calc(target, emissivity, probe, probe_emissivity)?;
                    process_lines = Some(process::ear(
                        target,
                        emissivity,
                        probe,
                        probe_emissivity,
                        single,
                        light,
                    ));
                    Ok(vec![
                        ("黑体亮温", format!("{} °C", format_number(light))),
                        (
                            "亮温修正值（亮温 − 黑体温度）",
                            format!("{} °C", format_number(light - target)),
                        ),
                    ])
                } else {
                    let response = self
                        .value(tool, "spectral_response", "0")
                        .parse::<usize>()
                        .unwrap_or(0);
                    let selected_lambda = if response == 0 {
                        self.number(tool, "wavelength", "0.65")?
                    } else {
                        0.0
                    };
                    let spectrum = Spectrum::new(match response {
                        1 => Response::Band(8.0, 14.0),
                        2 => Response::Band(7.0, 18.0),
                        _ => Response::Single(selected_lambda),
                    })?;
                    let source_emissivity = self.number(tool, "emissivity", "0.99")?;
                    let background = self.number(tool, "ambient", "20")?;
                    if self.value(tool, "model", "0") == "0" {
                        let t = if mode == 1 {
                            self.number(tool, "target", "1000")?
                        } else {
                            self.number(tool, "indicated", "1000")?
                        };
                        let lambda = website::wavelength(response, selected_lambda, t);
                        let mut steps = process::wavelength(response, lambda, t);
                        let delta = if mode == 1 {
                            let error = website::error(source_emissivity, lambda, t, background)?;
                            steps.extend(process::effective_error(
                                "辐射源",
                                source_emissivity,
                                lambda,
                                t,
                                background,
                                error,
                            ));
                            error
                        } else {
                            let instrument = self.number(tool, "instrument_emissivity", "0.95")?;
                            let error = website::correction(
                                instrument,
                                source_emissivity,
                                lambda,
                                t,
                                background,
                            )?;
                            let instrument_error =
                                website::error(instrument, lambda, t, background)?;
                            let source_error =
                                website::error(source_emissivity, lambda, t, background)?;
                            steps.extend(process::effective_error(
                                "仪表",
                                instrument,
                                lambda,
                                t,
                                background,
                                instrument_error,
                            ));
                            steps.push(String::new());
                            steps.extend(process::effective_error(
                                "辐射源",
                                source_emissivity,
                                lambda,
                                t,
                                background,
                                source_error,
                            ));
                            steps.push(format!(
                                "修正值（修约前）= 仪表误差 − 辐射源误差 = {} − {} = {} °C。",
                                format_process_number(instrument_error),
                                format_process_number(source_error),
                                format_process_number(error)
                            ));
                            error
                        };
                        // 页面先将温差修约至 0.01 °C，
                        // 再加到示值温度上。
                        let delta = (delta * 100.0).round() / 100.0;
                        steps.push(format!(
                            "温差乘以 100 后取最近整数（恰好一半时远离零），再除以 100；显示温差 = {} °C。",
                            format_process_number(delta)
                        ));
                        if mode == 2 {
                            steps.push(format!(
                                "修正后温度 = 示值 + 修约后修正值 = {} + {} = {} °C。",
                                format_process_number(t),
                                format_process_number(delta),
                                format_process_number(t + delta)
                            ));
                        }
                        process_lines = Some(steps);
                        return Ok(if mode == 1 {
                            vec![
                                ("发射率误差", format!("{} °C", format_number(delta))),
                                ("有效波长", format!("{} µm", format_number(lambda))),
                            ]
                        } else {
                            vec![
                                ("示值修正值", format!("{} °C", format_number(delta))),
                                ("修正后温度", format!("{} °C", format_number(t + delta))),
                                ("有效波长", format!("{} µm", format_number(lambda))),
                            ]
                        });
                    }
                    if mode == 1 {
                        let target = self.number(tool, "target", "1000")?;
                        let apparent = spectrum.apparent(target, background, source_emissivity)?;
                        process_lines = Some(process::planck(
                            &spectrum,
                            response,
                            selected_lambda,
                            target,
                            background,
                            source_emissivity,
                            1.0,
                            apparent,
                            false,
                        ));
                        Ok(vec![
                            ("表观亮温", format!("{} °C", format_number(apparent))),
                            (
                                "示值误差（亮温 − 真温）",
                                format!("{} °C", format_number(apparent - target)),
                            ),
                            (
                                "示值修正值",
                                format!("{} °C", format_number(target - apparent)),
                            ),
                        ])
                    } else {
                        let indicated = self.number(tool, "indicated", "1000")?;
                        let instrument = self.number(tool, "instrument_emissivity", "0.95")?;
                        let corrected = spectrum.corrected(
                            indicated,
                            background,
                            instrument,
                            source_emissivity,
                        )?;
                        process_lines = Some(process::planck(
                            &spectrum,
                            response,
                            selected_lambda,
                            indicated,
                            background,
                            source_emissivity,
                            instrument,
                            corrected,
                            true,
                        ));
                        Ok(vec![
                            ("修正后温度", format!("{} °C", format_number(corrected))),
                            (
                                "示值修正值",
                                format!("{} °C", format_number(corrected - indicated)),
                            ),
                        ])
                    }
                }
            })();
            show_result_or_error(ui, "计算结果", result);
            if let Some(lines) = &process_lines {
                calculation_process(ui, "radiation_process", lines);
            }
        });
        self.calculation_note(ui, tool);
    }
}
