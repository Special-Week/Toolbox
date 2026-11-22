//! 湿度页面：宽屏左输右显双栏布局，四种已知参数共用同一组输出。
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef, theme};
use eframe::egui::{self, RichText};
mod process;
pub(crate) mod website;

impl ToolboxApp {
    pub(crate) fn render_humidity(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        let mut process_lines = None;

        split_workspace(
            ui,
            0.40,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("湿空气参数").size(14.5).strong().color(p.ink));
                    ui.add_space(10.0);
                    let unit = TemperatureUnit(app.select_segmented(
                        ui,
                        tool,
                        "unit",
                        "温度单位",
                        &["°C", "K", "°F"],
                        0,
                    ));
                    let tk = format!("temperature.{}", unit.0);
                    let td = unit.display(20.0).to_string();
                    app.input(
                        ui,
                        tool,
                        &tk,
                        &format!("环境温度 ({})", unit.symbol()),
                        &td,
                        "−100～100 °C",
                    );
                    app.input(
                        ui,
                        tool,
                        "pressure",
                        "大气压 (kPa)",
                        "101.325",
                        "当地绝对压力",
                    );
                    let known = app.select_index(
                        ui,
                        tool,
                        "known",
                        "已知参数",
                        &["相对湿度", "露点 / 霜点", "湿球温度值", "混合比"],
                        0,
                    );
                    let key = format!(
                        "known.{known}.{}",
                        if known == 1 || known == 2 { unit.0 } else { 0 }
                    );
                    let label = match known {
                        1 => format!("露点 / 霜点 ({})", unit.symbol()),
                        2 => format!("湿球温度 ({})", unit.symbol()),
                        3 => "混合比 (g/kg)".into(),
                        _ => "相对湿度 (%RH)".into(),
                    };
                    let default = match known {
                        1 => unit.display(10.0).to_string(),
                        2 => unit.display(15.0).to_string(),
                        3 => "7.5".into(),
                        _ => "50".into(),
                    };
                    app.input(
                        ui,
                        tool,
                        &key,
                        &label,
                        &default,
                        if known == 1 {
                            "≤0 °C 按霜点输入，>0 °C 按露点输入"
                        } else {
                            "输入已知参数值"
                        },
                    );
                    let t = unit.celsius(app.number(tool, &tk, &td)?);
                    let mut v = app.number(tool, &key, &default)?;
                    if known == 1 || known == 2 {
                        v = unit.celsius(v);
                    }
                    let pressure = app.number(tool, "pressure", "101.325")? * 1000.0;
                    let result = website::calculate(t, pressure, known, v)?;
                    let steps = process::steps(t, pressure, known, v, unit, &result)?;
                    Ok::<_, String>((result, unit, steps))
                })
                .inner
            },
            |ui, _app, result| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match result {
                        Err(e) => render_error_message(ui, &e),
                        Ok((r, u, steps)) => {
                            let temp = |t: Option<f64>| {
                                t.map(|t| format!("{} {}", format_number(u.display(t)), u.symbol()))
                                    .unwrap_or("无有限值（干燥空气）".into())
                            };
                            let mut metrics = vec![
                                ("相对湿度", format!("{} %RH", format_number(r.rh))),
                                ("露点（水面）", temp(r.dew)),
                            ];
                            if r.frost.is_some_and(|t| t <= 0.0) {
                                metrics.push(("霜点（冰面）", temp(r.frost)));
                            }
                            metrics.extend([
                                ("饱和水气压", format!("{} Pa", format_number(r.saturated))),
                                ("水气压", format!("{} Pa", format_number(r.vapor))),
                                (
                                    "含湿量（体积分数）",
                                    format!("{} L/L", format_number(r.volume_fraction)),
                                ),
                                (
                                    "体积比（以干空气计）",
                                    format!("{} μL/L", format_number(r.ppm)),
                                ),
                                ("绝对湿度", format!("{} g/m³", format_number(r.absolute))),
                                ("质量比", format!("{} μg/g", format_number(r.mass))),
                                ("比湿", format!("{} g/kg", format_number(r.specific))),
                                ("混合比", format!("{} g/kg", format_number(r.mixing))),
                                ("含水量", format!("{} g/Nm³", format_number(r.normal))),
                                ("湿球温度", if r.wet.is_some() {
                                    temp(r.wet)
                                } else {
                                    "低于 −100 °C，超出模型范围".into()
                                }),
                            ]);
                            render_metrics(ui, "换算结果", &metrics);
                            process_lines = Some(steps);
                        }
                    }
                });
            },
        );

        if let Some(steps) = &process_lines {
            calculation_process(ui, "humidity_process", steps);
        }
        info_note(ui, tool.kind);
    }
}
