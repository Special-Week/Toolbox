//! 压力误差预算：相对项先乘量程跨度，再统一为目标压力单位，最后合成。
use crate::calculators::{common::*, numerics::rss};
use crate::{
    app::ToolboxApp,
    catalog::{Dimension, ToolDef, convert_unit, units_for},
};
use eframe::egui;
impl ToolboxApp {
    pub(crate) fn render_pressure_composite(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let mut process = None;
        let result = (|| {
            let units = units_for(Dimension::Pressure);
            let names: Vec<_> = units.iter().map(|u| u.code).collect();
            let target =
                self.select_index(ui, tool, "pressure_error.target", "结果单位", &names, 0);
            let n = self.dynamic_count(
                ui,
                tool,
                "pressure_error.components",
                "压力误差分量",
                3,
                1,
                50,
            );
            let mut errors = Vec::new();
            let mut steps = Vec::new();
            for i in 0..n {
                panel(ui, |ui| {
                    ui.label(format!("分量 {}", i + 1));
                    let key = format!("pressure_error.{i}");
                    let source =
                        self.select_index(ui, tool, &format!("{key}.unit"), "分量单位", &names, 0);
                    let relative = self.select_index(
                        ui,
                        tool,
                        &format!("{key}.type"),
                        "误差类型",
                        &["相对误差（量程百分比）", "绝对误差"],
                        0,
                    ) == 0;
                    let mut range_bounds = None;
                    let span = if relative {
                        self.input(
                            ui,
                            tool,
                            &format!("{key}.low"),
                            "量程下限",
                            "0",
                            "使用分量单位",
                        );
                        self.input(
                            ui,
                            tool,
                            &format!("{key}.high"),
                            "量程上限",
                            "100",
                            "使用分量单位",
                        );
                        let range = super::model::Range::new(
                            self.number(tool, &format!("{key}.low"), "0")?,
                            self.number(tool, &format!("{key}.high"), "100")?,
                        )?;
                        range_bounds = Some((range.low, range.high));
                        range.span
                    } else {
                        1.0
                    };
                    self.input(
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
                    let input = self.number(tool, &format!("{key}.error"), "0.02")?;
                    let error = input * if relative { span / 100.0 } else { 1.0 };
                    let converted = convert_unit(Dimension::Pressure, error, source, target)?;
                    errors.push(converted);
                    if i > 0 { steps.push(String::new()); }
                    if relative {
                        let (low, high) = range_bounds.unwrap();
                        steps.push(format!("分量 {}：跨度 = 上限 − 下限 = {} − {} = {} {}，相对误差 = {}%；绝对误差 = {} × ({} / 100) = {} {}。", i + 1, format_process_number(high), format_process_number(low), format_process_computed(span), units[source].code, format_process_number(input), format_process_number(input), format_process_computed(span), format_process_computed(error), units[source].code));
                    } else {
                        steps.push(format!("分量 {}：输入绝对误差 = {} {}。", i + 1, format_process_number(input), units[source].code));
                    }
                    let (from, _) = units[source].conversion_coefficients();
                    let (to, _) = units[target].conversion_coefficients();
                    steps.push(format!("统一单位：{} {} × ({} / {}) = {} {}。", format_process_computed(error), units[source].code, format_process_number(from), format_process_number(to), format_process_computed(converted), units[target].code));
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
            process = Some(steps);
            Ok(vec![
                ("分量个数", n.to_string()),
                (
                    "合成误差（平方和根）",
                    format!("{} {}", format_number(total), units[target].code),
                ),
            ])
        })();
        show_result_or_error(ui, "合成结果", result);
        if let Some(lines) = &process {
            calculation_process(ui, "pressure_error_process", lines);
        }
        self.calculation_note(ui, tool);
    }
}
