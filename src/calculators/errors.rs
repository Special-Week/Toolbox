//! 重复性、极差法、合成误差、误差分析及快速不确定度页面。
//! 统计以样本标准差为基础；绝对误差保留符号，不与误差绝对值混用。
use crate::INK;
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::calculators::numerics::{finite, rss, statistics};
use crate::catalog::ToolDef;
use eframe::egui::{self, RichText};

impl ToolboxApp {
    pub(crate) fn render_repeatability(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            let defaults = ["10.02", "10.01", "10.03", "10.02", "10.01"];
            let count = self.dynamic_count(ui, tool, "samples", "测量值", 5, 2, 50);
            let values = self.dynamic_values(
                ui,
                tool,
                "samples.value",
                count,
                "测量值",
                &defaults,
                "输入一个测量结果",
            );
            let result = (|| {
                let values = values?;
                let stats = statistics(&values)?;
                let (mean, sd, min, max) = (stats.mean, stats.sd, stats.min, stats.max);
                finite(&[max - min])?;
                ui.collapsing("各项残差", |ui| {
                    for (i, value) in values.iter().enumerate() {
                        ui.label(format!("测量 {}：{}", i + 1, format_number(value - mean)));
                    }
                });
                let rsd = if mean == 0.0 {
                    "平均值为 0".to_owned()
                } else {
                    let percent = sd / mean.abs() * 100.0;
                    finite(&[percent])?;
                    format!("{} %", format_number(percent))
                };
                Ok(vec![
                    ("测量次数", values.len().to_string()),
                    ("平均值", format_number(mean)),
                    ("样本标准差 s", format_number(sd)),
                    ("极差", format_number(max - min)),
                    (
                        "平均值标准差 s/√n",
                        format_number(sd / (values.len() as f64).sqrt()),
                    ),
                    ("相对标准偏差", rsd),
                ])
            })();
            show_result_or_error(ui, "统计结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_range_method(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            let defaults = ["10.02", "10.01", "10.03", "10.02", "10.01"];
            let input_method = self.select_index(
                ui,
                tool,
                "range.input_method",
                "输入方式",
                &["逐项测量值", "最大值、最小值与测量次数"],
                0,
            );
            let count = self.dynamic_count(
                ui,
                tool,
                "range_samples",
                if input_method == 0 {
                    "测量值"
                } else {
                    "测量次数"
                },
                5,
                2,
                9,
            );
            if input_method == 1 {
                self.input(ui, tool, "range.max", "最大值", "10.03", "测量最大值");
                self.input(ui, tool, "range.min", "最小值", "10.01", "测量最小值");
            }
            let values = if input_method == 0 {
                self.dynamic_values(
                    ui,
                    tool,
                    "range_samples.value",
                    count,
                    "测量值",
                    &defaults,
                    "输入一个测量结果",
                )
            } else {
                Ok(Vec::new())
            };
            let result = (|| {
                let values = if input_method == 0 {
                    values?
                } else {
                    vec![
                        self.number(tool, "range.min", "10.01")?,
                        self.number(tool, "range.max", "10.03")?,
                    ]
                };
                let n = count;
                let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if input_method == 1 && values[0] > values[1] {
                    return Err("最大值不能小于最小值".into());
                }
                let d2 = super::range_method::coefficient(n)?;
                let range = max - min;
                finite(&[range])?;
                let sd = range / d2;
                Ok(vec![
                    ("测量次数", n.to_string()),
                    ("极差 R", format_number(range)),
                    ("极差系数 d₂", format_number(d2)),
                    ("重复性标准差 s ≈ R/d₂", format_number(sd)),
                    ("平均值标准差 s/√n", format_number(sd / (n as f64).sqrt())),
                ])
            })();
            show_result_or_error(ui, "极差法结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_resultant_error(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        if self.select_index(
            ui,
            tool,
            "components.mode",
            "分量输入",
            &["同单位误差分量", "压力分量（相对 / 绝对误差）"],
            0,
        ) == 1
        {
            self.render_pressure_composite(ui, tool);
            return;
        }
        ui.group(|ui| {
            ui.set_max_width(820.0);
            let count = self.dynamic_count(ui, tool, "components", "误差分量", 3, 1, 50);
            let errors = self.dynamic_values(
                ui,
                tool,
                "components.error",
                count,
                "误差分量",
                &["0.02", "0.03", "0.01"],
                "输入带正负号的误差值",
            );
            let sensitivities = self.dynamic_values(
                ui,
                tool,
                "components.sensitivity",
                count,
                "灵敏系数",
                &["1", "1", "1"],
                "对应误差分量的灵敏系数",
            );
            let result = (|| {
                let errors = errors?;
                let sensitivities = sensitivities?;
                let contributions: Vec<f64> = errors
                    .iter()
                    .zip(&sensitivities)
                    .map(|(e, c)| e * c)
                    .collect();
                let combined = rss(contributions.iter().copied())?;
                let linear_bound = contributions.iter().map(|x| x.abs()).sum::<f64>();
                finite(&[linear_bound])?;
                Ok(vec![
                    ("分量个数", errors.len().to_string()),
                    ("合成误差（平方和根）", format_number(combined)),
                    ("最坏情况界限（绝对值求和）", format_number(linear_bound)),
                    (
                        "最大单项贡献",
                        format_number(
                            contributions
                                .iter()
                                .map(|x| x.abs())
                                .fold(0.0_f64, f64::max),
                        ),
                    ),
                ])
            })();
            show_result_or_error(ui, "合成结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_error_analysis(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(RichText::new("误差参数").size(14.0).strong().color(INK));
            ui.add_space(5.0);
            self.input(ui, tool, "measured", "测量值 x", "100.02", "测量结果");
            self.input(ui, tool, "true", "真值 x₀", "100", "参考真值");
            self.input(
                ui,
                tool,
                "reference",
                "引用值（选填）",
                "",
                "留空时使用真值",
            );
            let result = (|| {
                let measured = self.number(tool, "measured", "100.02")?;
                let truth = self.number(tool, "true", "100")?;
                let reference_text = self.value(tool, "reference", "");
                let reference = if reference_text.trim().is_empty() {
                    truth
                } else {
                    parse_number(&reference_text)?
                };
                let delta = measured - truth;
                finite(&[delta])?;
                let relative = if truth == 0.0 {
                    "真值为 0，未定义".to_owned()
                } else {
                    let percent = delta / truth * 100.0;
                    finite(&[percent])?;
                    format!("{} %", format_number(percent))
                };
                let fiducial = if reference == 0.0 {
                    "引用值为 0，未定义".to_owned()
                } else {
                    let percent = delta / reference * 100.0;
                    finite(&[percent])?;
                    format!("{} %", format_number(percent))
                };
                Ok(vec![
                    ("绝对误差 Δ", format_number(delta)),
                    ("相对误差 δ", relative),
                    ("引用误差 γ", fiducial),
                    ("绝对值", format_number(delta.abs())),
                ])
            })();
            show_result_or_error(ui, "误差结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_simple_uncertainty(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(RichText::new("A 类评定数据").size(14.0).strong().color(INK));
            ui.add_space(4.0);
            let a_count = self.dynamic_count(ui, tool, "a_samples", "重复测量值", 5, 0, 50);
            let a_values = self.dynamic_values(
                ui,
                tool,
                "a_samples.value",
                a_count,
                "测量值",
                &["10.02", "10.01", "10.03", "10.02", "10.01"],
                "输入一个测量结果",
            );
            self.input(ui, tool, "a_sensitivity", "A 类灵敏系数", "1", "默认 1");
            ui.add_space(7.0);
            ui.label(RichText::new("B 类评定数据").size(14.0).strong().color(INK));
            let b_count = self.dynamic_count(ui, tool, "b_widths", "B 类半宽", 2, 0, 50);
            let b_widths = self.dynamic_values(
                ui,
                tool,
                "b_widths.value",
                b_count,
                "半宽",
                &["0.02", "0.03"],
                "输入该 B 类分量的半宽",
            );
            let distribution = self.select_index(
                ui,
                tool,
                "distribution",
                "B 类分布",
                &[
                    "均匀分布",
                    "三角分布",
                    "正态分布 k=2",
                    "正态分布 k=3",
                    "两点分布",
                    "反正弦分布",
                ],
                0,
            );
            self.input(
                ui,
                tool,
                "b_sensitivity",
                "B 类灵敏系数",
                "1",
                "当前统一应用于 B 类分量",
            );
            self.input(ui, tool, "coverage", "包含因子 k", "2", "常用值为 2");
            let result = (|| {
                let data = a_values?;
                let u_a = if data.is_empty() {
                    0.0
                } else {
                    if data.len() < 2 {
                        return Err(
                            "A 类评定至少需要 2 个有效测量值；不使用 A 类时将数量减为 0".to_owned()
                        );
                    }
                    statistics(&data)?.standard_error
                };
                let a_sensitivity = self.number(tool, "a_sensitivity", "1")?;
                let b_widths = b_widths?;
                if b_widths.iter().any(|x| *x < 0.0) {
                    return Err("B 类半宽不能为负数".to_owned());
                }
                let divisor = match distribution {
                    0 => 3.0_f64.sqrt(),
                    1 => 6.0_f64.sqrt(),
                    2 => 2.0,
                    3 => 3.0,
                    4 => 1.0,
                    _ => 2.0_f64.sqrt(),
                };
                if data.is_empty() && b_widths.is_empty() {
                    return Err("至少需要一组 A 类数据或一个 B 类分量".to_owned());
                }
                let u_b = rss(b_widths.iter().map(|a| a / divisor))?;
                let b_sensitivity = self.number(tool, "b_sensitivity", "1")?;
                let k = self.number(tool, "coverage", "2")?;
                if k <= 0.0 {
                    return Err("包含因子必须大于 0".to_owned());
                }
                let combined = rss([a_sensitivity * u_a, b_sensitivity * u_b])?;
                finite(&[k * combined])?;
                Ok(vec![
                    ("A 类标准不确定度", format_number(u_a)),
                    ("B 类合成标准不确定度", format_number(u_b)),
                    ("合成标准不确定度 u_c", format_number(combined)),
                    ("扩展不确定度 U", format_number(k * combined)),
                    ("包含因子 k", format_number(k)),
                ])
            })();
            show_result_or_error(ui, "不确定度结果", result);
        });
        info_note(ui, tool.kind);
    }
}
