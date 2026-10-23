//! 指定标准器核验和内置量程推荐；界面组织参数，判据统一委托 selection_model。
use super::{
    model::Range,
    selection_model::{self as model, METRIC_RANGES},
};
use crate::calculators::common::*;
use crate::{
    app::ToolboxApp,
    catalog::{Dimension, ToolDef, convert_unit, units_for},
};
use eframe::egui;
impl ToolboxApp {
    pub(crate) fn render_pressure_reference(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        self.render_selection(ui, tool, false);
    }
    pub(crate) fn render_transmitter_reference(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        self.render_selection(ui, tool, true);
    }
    fn render_selection(&mut self, ui: &mut egui::Ui, tool: &ToolDef, transmitter: bool) {
        ui.set_max_width(820.0);
        if transmitter
            && self.select_index(
                ui,
                tool,
                "selection.model",
                "评定模型",
                &["压力与电测合成评定", "两通道跨度比评定"],
                0,
            ) == 1
        {
            self.render_transmitter_reference_legacy(ui, tool);
            return;
        }
        let result = (|| {
            let mode = self.select_index(
                ui,
                tool,
                "selection.mode",
                "选型方式",
                &["核验指定标准器", "按被检量程推荐", "按检定范围推荐"],
                0,
            );
            let units = units_for(Dimension::Pressure);
            let index = self.select_index(
                ui,
                tool,
                "selection.unit",
                "压力单位",
                &units.iter().map(|u| u.code).collect::<Vec<_>>(),
                units.iter().position(|u| u.code == "MPa").unwrap_or(0),
            );
            let previous = self
                .value(tool, "selection.previous_unit", &index.to_string())
                .parse::<usize>()
                .unwrap_or(index);
            if previous < units.len() && previous != index {
                // 切换显示单位时保持已输入量程的物理值，不把 MPa 数字直接解释为 Pa。
                for key in [
                    "dut_low",
                    "dut_high",
                    "ref_low",
                    "ref_high",
                    "ref_pressure_low",
                    "ref_pressure_high",
                ] {
                    let text = self.value(tool, key, "");
                    if let Ok(value) = parse_number(&text) {
                        self.set_value(
                            tool,
                            key,
                            convert_unit(Dimension::Pressure, value, previous, index)?.to_string(),
                        );
                    }
                }
            }
            self.set_value(tool, "selection.previous_unit", index.to_string());
            let piston = !transmitter
                && self.select_index(
                    ui,
                    tool,
                    "selection.reference_type",
                    "标准器类型",
                    &["数字压力标准器", "活塞压力计"],
                    0,
                ) == 1;
            let dut_acc = if transmitter {
                "dut_pressure_accuracy"
            } else {
                "dut_accuracy"
            };
            let ref_acc = if transmitter {
                "ref_pressure_accuracy"
            } else {
                "ref_accuracy"
            };
            self.input(
                ui,
                tool,
                "dut_low",
                "被检 / 检定范围下限",
                "0",
                "与压力单位一致",
            );
            self.input(
                ui,
                tool,
                "dut_high",
                "被检 / 检定范围上限",
                "1",
                "上限须大于下限",
            );
            self.input(
                ui,
                tool,
                dut_acc,
                "被检准确度 (%)",
                if transmitter { "0.1" } else { "0.4" },
                "按量程跨度百分数",
            );
            self.input(ui, tool, "required_ratio", "量传比要求", "3", "例如 3 或 4");
            let dut = Range::new(
                self.number(tool, "dut_low", "0")?,
                self.number(tool, "dut_high", "1")?,
            )?;
            let da = self.number(tool, dut_acc, if transmitter { "0.1" } else { "0.4" })?;
            let required = self.number(tool, "required_ratio", "3")?;
            model::verify(dut, dut, da, 0.01, required, piston)?;
            let mut signal = Range::new(4., 20.)?;
            let mut reading = 0.02;
            let mut fixed = 0.001;
            let mut signal_symbol = "mA";
            if transmitter {
                let signal_index = self.select_index(
                    ui,
                    tool,
                    "selection.signal_unit",
                    "电信号单位",
                    &["mA", "V", "mV"],
                    0,
                );
                signal_symbol = ["mA", "V", "mV"][signal_index];
                for (k, l, d) in [
                    ("dut_signal_low", "输出信号下限", "4"),
                    ("dut_signal_high", "输出信号上限", "20"),
                    ("selection.reading", "电测读数准确度 (%)", "0.02"),
                    ("selection.fixed", "电测固定误差", "0.001"),
                ] {
                    self.input(ui, tool, k, l, d, "信号值与固定误差使用相同单位");
                }
                signal = Range::new(
                    self.number(tool, "dut_signal_low", "4")?,
                    self.number(tool, "dut_signal_high", "20")?,
                )?;
                reading = self.number(tool, "selection.reading", "0.02")?;
                fixed = self.number(tool, "selection.fixed", "0.001")?;
            }
            if mode == 0 {
                let (low, high) = if transmitter {
                    ("ref_pressure_low", "ref_pressure_high")
                } else {
                    ("ref_low", "ref_high")
                };
                self.input(ui, tool, low, "标准器量程下限", "0", "与压力单位一致");
                self.input(ui, tool, high, "标准器量程上限", "1.6", "需覆盖被检量程");
                self.input(
                    ui,
                    tool,
                    ref_acc,
                    "标准器准确度 (%)",
                    if transmitter { "0.01" } else { "0.05" },
                    "按标准器技术指标",
                );
                let reference = Range::new(
                    self.number(tool, low, "0")?,
                    self.number(tool, high, "1.6")?,
                )?;
                let ra = self.number(tool, ref_acc, if transmitter { "0.01" } else { "0.05" })?;
                if transmitter {
                    let s = model::transmitter(
                        dut, reference, signal, da, ra, reading, fixed, required,
                    )?;
                    return Ok(vec![
                        ("被检压力允差", format_number(s.pressure.dut_tolerance)),
                        (
                            "标准压力允差",
                            format_number(s.pressure.reference_tolerance),
                        ),
                        (
                            "合成扩展不确定度 (k=2)",
                            format!("{} {signal_symbol}", format_number(s.expanded)),
                        ),
                        (
                            "电信号允许限值",
                            format!("{} {signal_symbol}", format_number(s.allowance)),
                        ),
                        ("综合判断", if s.meets { "符合" } else { "不符合" }.into()),
                        (
                            "压力量程覆盖",
                            if s.pressure.coverage {
                                "符合"
                            } else {
                                "不符合"
                            }
                            .into(),
                        ),
                    ]);
                }
                let s = model::verify(dut, reference, da, ra, required, piston)?;
                return Ok(vec![
                    ("被检允差", format_number(s.dut_tolerance)),
                    ("标准器允差", format_number(s.reference_tolerance)),
                    ("量传比", format_number(s.ratio)),
                    (
                        "量程覆盖",
                        if s.coverage { "符合" } else { "不符合" }.into(),
                    ),
                    ("综合判断", if s.meets { "符合" } else { "不符合" }.into()),
                ]);
            }
            // Store catalogue ranges in MPa and convert into the selected unit.
            let series = if transmitter {
                0
            } else {
                self.select_index(
                    ui,
                    tool,
                    "selection.series",
                    "标准量程系列",
                    &["常用公制量程", "扩展量程"],
                    0,
                )
            };
            let ranges = if series == 0 {
                METRIC_RANGES
            } else {
                super::ranges::EXTENDED_RANGES
            };
            let mpa = units.iter().position(|u| u.code == "MPa").unwrap();
            let catalogue: Vec<_> = ranges
                .iter()
                .map(|&(l, h)| {
                    Range::new(
                        convert_unit(Dimension::Pressure, l, mpa, index)?,
                        convert_unit(Dimension::Pressure, h, mpa, index)?,
                    )
                })
                .collect::<Result<_, String>>()?;
            let fixed_accuracy = if mode == 2 {
                self.input(
                    ui,
                    tool,
                    ref_acc,
                    "标准器准确度 (%)",
                    "0.05",
                    "用于检定范围推荐",
                );
                Some(self.number(tool, ref_acc, "0.05")?)
            } else {
                None
            };
            let duties = if mode == 1 {
                vec![dut]
            } else {
                catalogue
                    .iter()
                    .copied()
                    .filter(|r| dut.covers(*r))
                    .collect()
            };
            let mut count = 0;
            for duty in duties {
                let accuracies = if let Some(a) = fixed_accuracy {
                    vec![a]
                } else if piston {
                    vec![0.003, 0.005, 0.01, 0.05]
                } else {
                    vec![0.01, 0.02, 0.05]
                };
                let mut matches = Vec::new();
                for r in &catalogue {
                    for a in &accuracies {
                        let good = if transmitter {
                            model::transmitter(duty, *r, signal, da, *a, reading, fixed, required)?
                                .meets
                        } else {
                            model::verify(duty, *r, da, *a, required, piston)?.meets
                        };
                        if good {
                            matches.push((*r, *a));
                        }
                    }
                }
                // 符合条件后优先较宽松的准确度等级，再优先较小跨度；这不是价格排序。
                matches.sort_by(|a, b| {
                    b.1.total_cmp(&a.1)
                        .then_with(|| a.0.span.total_cmp(&b.0.span))
                });
                ui.label(format!(
                    "被检量程：{}～{} {}",
                    format_number(duty.low),
                    format_number(duty.high),
                    units[index].code
                ));
                if matches.is_empty() {
                    ui.label("没有满足当前条件的标准器组合");
                }
                for (i, (r, a)) in matches.iter().enumerate() {
                    ui.label(format!(
                        "{}{}～{} {}，{}%",
                        if i == 0 { "推荐：" } else { "可选：" },
                        format_number(r.low),
                        format_number(r.high),
                        units[index].code,
                        format_number(*a)
                    ));
                    count += 1;
                }
            }
            Ok(vec![("满足条件的量程 / 准确度组合", count.to_string())])
        })();
        show_result_or_error(ui, "选型结果", result);
        self.calculation_note(ui, tool);
    }
}
