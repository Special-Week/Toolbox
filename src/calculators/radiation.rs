//! 三个辐射页面共用输入与结果布局；mode=0/1/2 对应耳温计、源误差、示值修正。
//! 有效波长法与 Planck 法的误差方向不同，结果标签和计算说明分别明确。
pub(crate) mod model;
pub(crate) mod website;
use crate::INK;
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use eframe::egui::{self, RichText};
use model::{Response, Spectrum, ear_blackbody};

impl ToolboxApp {
    pub(crate) fn render_radiation(&mut self, ui: &mut egui::Ui, tool: &ToolDef, mode: u8) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
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
            let result = (|| {
                if mode == 0 {
                    let target = self.number(tool, "target", "35")?;
                    let calc = if self.value(tool, "ear.model", "0") == "1" {
                        model::ear_single_reflection
                    } else {
                        ear_blackbody
                    };
                    let light = calc(
                        target,
                        self.number(tool, "emissivity", "0.9995")?,
                        self.number(tool, "probe_temperature", "18")?,
                        self.number(tool, "probe_emissivity", "0.8")?,
                    )?;
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
                    let spectrum = Spectrum::new(match response {
                        1 => Response::Band(8.0, 14.0),
                        2 => Response::Band(7.0, 18.0),
                        _ => Response::Single(self.number(tool, "wavelength", "0.65")?),
                    })?;
                    let source_emissivity = self.number(tool, "emissivity", "0.99")?;
                    let background = self.number(tool, "ambient", "20")?;
                    if self.value(tool, "model", "0") == "0" {
                        let t = if mode == 1 {
                            self.number(tool, "target", "1000")?
                        } else {
                            self.number(tool, "indicated", "1000")?
                        };
                        let lambda = website::wavelength(
                            response,
                            if response == 0 {
                                self.number(tool, "wavelength", "0.65")?
                            } else {
                                0.0
                            },
                            t,
                        );
                        let delta = if mode == 1 {
                            website::error(source_emissivity, lambda, t, background)?
                        } else {
                            website::correction(
                                self.number(tool, "instrument_emissivity", "0.95")?,
                                source_emissivity,
                                lambda,
                                t,
                                background,
                            )?
                        };
                        // The page rounds the temperature difference to 0.01 °C
                        // before adding it to the indicated temperature.
                        let delta = (delta * 100.0).round() / 100.0;
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
                        let corrected = spectrum.corrected(
                            indicated,
                            background,
                            self.number(tool, "instrument_emissivity", "0.95")?,
                            source_emissivity,
                        )?;
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
        });
        info_note(ui, tool.kind);
    }
}
