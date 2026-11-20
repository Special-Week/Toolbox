//! 压力误差预算：统一左输右显双栏布局，相对项先乘量程跨度再统一为单位合成。
use crate::calculators::{common::*, numerics::rss};
use crate::{
    app::ToolboxApp,
    catalog::{Dimension, ToolDef, convert_unit, units_for},
    theme,
};
use eframe::egui::{self, CornerRadius, RichText, Stroke};

type CompositeResult = Result<(Vec<(&'static str, String)>, Vec<String>), String>;

impl ToolboxApp {
    pub(crate) fn render_pressure_composite(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        let mut process = None;
        split_workspace(
            ui,
            0.48,
            self,
            |ui, app| {
                panel(ui, |ui| -> CompositeResult {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("压力误差分量").size(14.5).strong().color(p.ink));
                    ui.add_space(8.0);
                    let units = units_for(Dimension::Pressure);
                    let names: Vec<_> = units.iter().map(|u| u.code).collect();
                    let target = app.select_index(
                        ui,
                        tool,
                        "pressure_error.target",
                        "结果单位",
                        &names,
                        0,
                    );
                    let n = app.dynamic_count(
                        ui,
                        tool,
                        "pressure_error.components",
                        "压力误差分量",
                        3,
                        1,
                        50,
                    );
                    ui.add_space(6.0);
                    let mut errors = Vec::new();
                    let mut steps = Vec::new();
                    for i in 0..n {
                        egui::Frame::new()
                            .fill(p.surface_subtle)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(12)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    RichText::new(format!("分量 {}", i + 1))
                                        .size(13.0)
                                        .strong()
                                        .color(p.ink),
                                );
                                ui.add_space(4.0);
                                let key = format!("pressure_error.{i}");
                                let source = app.select_index(
                                    ui,
                                    tool,
                                    &format!("{key}.unit"),
                                    "分量单位",
                                    &names,
                                    0,
                                );
                                let relative = app.select_index(
                                    ui,
                                    tool,
                                    &format!("{key}.type"),
                                    "误差类型",
                                    &["相对误差（量程百分比）", "绝对误差"],
                                    0,
                                ) == 0;
                                let mut range_bounds = None;
                                let span = if relative {
                                    app.input(
                                        ui,
                                        tool,
                                        &format!("{key}.low"),
                                        "量程下限",
                                        "0",
                                        "使用分量单位",
                                    );
                                    app.input(
                                        ui,
                                        tool,
                                        &format!("{key}.high"),
                                        "量程上限",
                                        "100",
                                        "使用分量单位",
                                    );
                                    let range = super::model::Range::new(
                                        app.number(tool, &format!("{key}.low"), "0")?,
                                        app.number(tool, &format!("{key}.high"), "100")?,
                                    )?;
                                    range_bounds = Some((range.low, range.high));
                                    range.span
                                } else {
                                    1.0
                                };
                                app.input(
                                    ui,
                                    tool,
                                    &format!("{key}.error"),
                                    if relative {
                                        "误差 (%)"
                                    } else {
                                        "绝对误差"
                                    },
                                    "0.02",
                                    "允许带正负号",
                                );
                                let input = app.number(tool, &format!("{key}.error"), "0.02")?;
                                let error = input * if relative { span / 100.0 } else { 1.0 };
                                let converted =
                                    convert_unit(Dimension::Pressure, error, source, target)?;
                                errors.push(converted);
                                if i > 0 {
                                    steps.push(String::new());
                                }
                                if relative {
                                    let (low, high) = range_bounds.unwrap();
                                    steps.push(format!("分量 {}：跨度 = 上限 − 下限 = {} − {} = {} {}，相对误差 = {}%；绝对误差 = {} × ({} / 100) = {} {}。", i + 1, format_process_number(high), format_process_number(low), format_process_computed(span), units[source].code, format_process_number(input), format_process_number(input), format_process_computed(span), format_process_computed(error), units[source].code));
                                } else {
                                    steps.push(format!(
                                        "分量 {}：输入绝对误差 = {} {}。",
                                        i + 1,
                                        format_process_number(input),
                                        units[source].code
                                    ));
                                }
                                let (from, _) = units[source].conversion_coefficients();
                                let (to, _) = units[target].conversion_coefficients();
                                steps.push(format!(
                                    "统一单位：{} {} × ({} / {}) = {} {}。",
                                    format_process_computed(error),
                                    units[source].code,
                                    format_process_number(from),
                                    format_process_number(to),
                                    format_process_computed(converted),
                                    units[target].code
                                ));
                                Ok::<_, String>(())
                            })
                            .inner?;
                        ui.add_space(6.0);
                    }
                    let total = rss(errors)?;
                    steps.push(String::new());
                    steps.push(format!(
                        "合成误差 = √Σeᵢ² = {} {}；程序使用 hypot 逐项累加。",
                        format_process_computed(total),
                        units[target].code
                    ));
                    Ok((
                        vec![
                            ("分量个数", n.to_string()),
                            (
                                "合成误差（平方和根）",
                                format!("{} {}", format_number(total), units[target].code),
                            ),
                        ],
                        steps,
                    ))
                })
                .inner
            },
            |ui, _app, result| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match result {
                        Ok((metrics, steps)) => {
                            render_metrics(ui, "合成结果", &metrics);
                            process = Some(steps);
                        }
                        Err(error) => render_error_message(ui, &error),
                    }
                });
            },
        );
        if let Some(lines) = &process {
            calculation_process(ui, "pressure_error_process", lines);
        }
        self.calculation_note(ui, tool);
    }
}
