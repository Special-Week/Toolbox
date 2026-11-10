//! 重复性、极差法、合成误差、误差分析及快速不确定度页面。
//! 统计以样本标准差为基础；绝对误差保留符号，不与误差绝对值混用。
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::calculators::numerics::{finite, rss, statistics};
use crate::catalog::ToolDef;
use crate::INK;
use eframe::egui::{self, RichText};

impl ToolboxApp {
    pub(crate) fn render_repeatability(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        panel(ui, |ui| {
            ui.set_max_width(ui.available_width().min(820.0));
            let defaults = ["10.02", "10.01", "10.03", "10.02", "10.01"];
            let count = self.dynamic_count(ui, tool, "samples", "测量值", 5, 2, 50);
            let values =
                self.dynamic_values(ui, tool, "samples.value", count, "测量值", &defaults, "");
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
                    ("平均值标准差 s/√n", format_number(stats.standard_error)),
                    ("相对标准偏差", rsd),
                ])
            })();
            show_result_or_error(ui, "统计结果", result);
        });
        self.calculation_note(ui, tool);
    }

    pub(crate) fn render_range_method(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        panel(ui, |ui| {
            ui.set_max_width(ui.available_width().min(820.0));
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
                    "",
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
        self.calculation_note(ui, tool);
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
        panel(ui, |ui| {
            ui.set_max_width(ui.available_width().min(820.0));
            let count = self.dynamic_count(ui, tool, "components", "误差分量", 3, 1, 50);
            let mut errors = Vec::with_capacity(count);
            let mut sensitivities = Vec::with_capacity(count);
            for index in 0..count {
                let error_key = format!("components.error.{index}");
                let sensitivity_key = format!("components.sensitivity.{index}");
                let error_default = ["0.02", "0.03", "0.01"].get(index).copied().unwrap_or("0");
                let sensitivity_default = if index < 3 { "1" } else { "0" };
                let mut fields = |error_ui: &mut egui::Ui, sensitivity_ui: &mut egui::Ui| {
                    self.input(
                        error_ui,
                        tool,
                        &error_key,
                        &format!("误差分量 {}", index + 1),
                        error_default,
                        "输入带正负号的误差值",
                    );
                    self.input(
                        sensitivity_ui,
                        tool,
                        &sensitivity_key,
                        &format!("灵敏系数 {}", index + 1),
                        sensitivity_default,
                        "对应误差分量的灵敏系数",
                    );
                };
                if ui.available_width() >= 360.0 {
                    ui.columns(2, |columns| {
                        let (left, right) = columns.split_at_mut(1);
                        fields(&mut left[0], &mut right[0]);
                    });
                } else {
                    self.input(
                        ui,
                        tool,
                        &error_key,
                        &format!("误差分量 {}", index + 1),
                        error_default,
                        "输入带正负号的误差值",
                    );
                    self.input(
                        ui,
                        tool,
                        &sensitivity_key,
                        &format!("灵敏系数 {}", index + 1),
                        sensitivity_default,
                        "对应误差分量的灵敏系数",
                    );
                }
                errors.push(
                    self.number(tool, &error_key, error_default)
                        .map_err(|e| format!("第 {} 项：{e}", index + 1)),
                );
                sensitivities.push(
                    self.number(tool, &sensitivity_key, sensitivity_default)
                        .map_err(|e| format!("第 {} 项：{e}", index + 1)),
                );
            }
            let result = (|| {
                let errors = errors.into_iter().collect::<Result<Vec<_>, _>>()?;
                let sensitivities = sensitivities.into_iter().collect::<Result<Vec<_>, _>>()?;
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
        self.calculation_note(ui, tool);
    }

    pub(crate) fn render_error_analysis(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        panel(ui, |ui| {
            ui.set_max_width(ui.available_width().min(820.0));
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
        self.calculation_note(ui, tool);
    }

    pub(crate) fn render_simple_uncertainty(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let b_count = self.dynamic_count(ui, tool, "b_widths", "B 类分量", 2, 0, 50);
        let distribution = self.select_index(
            ui,
            tool,
            "distribution",
            "B 类共用分布",
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
            "B 类共用灵敏系数",
            "1",
            "当前统一应用于 B 类分量",
        );
        ui.add_space(8.0);
        let mut a_values = None;
        let mut a_rounding = None;
        let mut b_widths = Vec::new();
        let mut b_rounding = Vec::new();
        balanced_columns(ui, "quick_uncertainty_columns", 1 + b_count, |ui, index| {
            panel(ui, |ui| {
                ui.set_width(ui.available_width());
                if index == 0 {
                    ui.label(RichText::new("A 类评定数据").size(14.0).strong().color(INK));
                    ui.add_space(4.0);
                    let count = self.dynamic_count(ui, tool, "a_samples", "重复测量值", 5, 0, 50);
                    a_values = Some(self.dynamic_values(
                        ui,
                        tool,
                        "a_samples.value",
                        count,
                        "测量值",
                        &["10.02", "10.01", "10.03", "10.02", "10.01"],
                        "",
                    ));
                    if count > 0 {
                        self.input(
                            ui,
                            tool,
                            "a_average_count",
                            "取测量次数 n",
                            "",
                            "留空默认 1 次",
                        );
                    }
                    self.input(ui, tool, "a_sensitivity", "A 类灵敏系数", "1", "");
                    if count > 0 {
                        a_rounding = Some(self.uncertainty_rounding(ui, tool, "quick.a", ""));
                    }
                } else {
                    let i = index - 1;
                    ui.label(
                        RichText::new(format!("B 类分量 {}", i + 1))
                            .size(14.0)
                            .strong()
                            .color(INK),
                    );
                    ui.add_space(4.0);
                    let key = format!("b_widths.value.{i}");
                    let default = ["0.02", "0.03"].get(i).copied().unwrap_or("0");
                    let label = if matches!(distribution, 2 | 3) {
                        "扩展不确定度 U"
                    } else {
                        "半宽"
                    };
                    self.input(ui, tool, &key, label, default, "");
                    b_widths.push(self.number(tool, &key, default));
                    b_rounding.push(self.uncertainty_rounding(
                        ui,
                        tool,
                        &format!("quick.b.{i}"),
                        "",
                    ));
                }
            });
        });
        ui.label(RichText::new("合成设置").size(14.0).strong().color(INK));
        ui.add_space(4.0);
        self.input(ui, tool, "coverage", "包含因子 k", "2", "");
        let digits = self.select_index(
            ui,
            tool,
            "quick.result.digits",
            "合成有效数字",
            &["1 位", "2 位", "3 位", "4 位"],
            1,
        ) + 1;
        let carry = self.select_index(
            ui,
            tool,
            "quick.result.rounding",
            "合成修约方式",
            &["四舍六入", "进位"],
            1,
        ) == 1;
        let a_values = a_values.unwrap_or_else(|| Err("A 类数据未能读取".to_owned()));
        let result = (|| {
            let data = a_values?;
            let raw_u_a = if data.is_empty() {
                0.0
            } else {
                if data.len() < 2 {
                    return Err(
                        "A 类评定至少需要 2 个有效测量值；不使用 A 类时将数量减为 0".to_owned()
                    );
                }
                let count = super::uncertainty::model::average_count(&self.value(
                    tool,
                    "a_average_count",
                    "",
                ))?;
                let stats = statistics(&data)?;
                match count {
                    Some(count) => stats.sd / (count as f64).sqrt(),
                    None => stats.sd,
                }
            };
            let a_rounding = a_rounding.transpose()?;
            let u_a = match a_rounding {
                Some(rounding) => super::uncertainty::model::round_significant(raw_u_a, rounding)?,
                None => 0.0,
            };
            let a_sensitivity = self.number(tool, "a_sensitivity", "1")?;
            let b_widths = b_widths.into_iter().collect::<Result<Vec<_>, _>>()?;
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
            let b_values = b_widths
                .iter()
                .zip(b_rounding)
                .map(|(width, rounding)| {
                    let rounding = rounding?;
                    let raw = width / divisor;
                    Ok((
                        raw,
                        super::uncertainty::model::round_significant(raw, rounding)?,
                        rounding,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let u_b = rss(b_values.iter().map(|(_, rounded, _)| *rounded))?;
            let b_sensitivity = self.number(tool, "b_sensitivity", "1")?;
            let k = self.number(tool, "coverage", "2")?;
            if k <= 0.0 {
                return Err("包含因子必须大于 0".to_owned());
            }
            let combined = rss([a_sensitivity * u_a, b_sensitivity * u_b])?;
            finite(&[k * combined])?;
            let (rounded, rounded_expanded) =
                super::uncertainty::model::rounded_combined(combined, k, digits, carry)?;
            if !b_values.is_empty() {
                ui.collapsing("B 类分量修约明细", |ui| {
                    for (i, (raw, rounded, rounding)) in b_values.iter().enumerate() {
                        ui.label(format!(
                            "B 类分量 {}：u={}，修约后 u={}",
                            i + 1,
                            format_number(*raw),
                            super::uncertainty::model::format_rounded(*rounded, Some(*rounding))
                        ));
                    }
                });
            }
            Ok(vec![
                ("A 类标准不确定度", format_number(raw_u_a)),
                (
                    "A 类修约后标准不确定度",
                    super::uncertainty::model::format_rounded(u_a, a_rounding),
                ),
                ("B 类合成标准不确定度", format_number(u_b)),
                ("合成标准不确定度 u_c", format_number(combined)),
                ("扩展不确定度 U", format_number(k * combined)),
                ("修约后标准不确定度", format_number(rounded)),
                ("修约后扩展不确定度", format_number(rounded_expanded)),
                ("包含因子 k", format_number(k)),
            ])
        })();
        show_result_or_error(ui, "不确定度结果", result);
        self.calculation_note(ui, tool);
    }
}
