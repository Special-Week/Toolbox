//! 可嵌套不确定度预算表；递归深度和分量数量设有上限，控制页面与计算规模。
pub(crate) use toolbox_core::calculators::uncertainty::{model, process};
use crate::calculators::common::*;
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
use model::{Component, Detail, Kind, Rounding};

impl ToolboxApp {
    pub(crate) fn uncertainty_rounding(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
    ) -> Result<Rounding, String> {
        let digits_key = format!("{key}.digits");
        self.input(
            ui,
            tool,
            &digits_key,
            &format!("{label}有效数字"),
            "",
            "留空保留原始计算精度（1～17 位）",
        );
        let carry = self.select_index(
            ui,
            tool,
            &format!("{key}.rounding"),
            &format!("{label}修约方式"),
            &["四舍六入", "进位"],
            1,
        ) == 1;
        Ok(Rounding {
            digits: model::significant_digits(&self.value(tool, &digits_key, ""))?,
            carry,
        })
    }
    fn uncertainty_component(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        depth: usize,
    ) -> Result<Component, String> {
        self.input(
            ui,
            tool,
            &format!("{key}.name"),
            "分量名称",
            "分量",
            "填写名称",
        );
        let name = self.value(tool, &format!("{key}.name"), "分量");
        let options = if depth < 4 {
            &["A 类评定", "B 类评定", "合成分量"][..]
        } else {
            &["A 类评定", "B 类评定"][..]
        };
        let kind = self.select_index(ui, tool, &format!("{key}.type"), "分量类型", options, 1);
        let sk = format!("{key}.sensitivity");
        self.input(ui, tool, &sk, "灵敏系数", "1", "折算至相同输出单位");
        let sensitivity = self.number(tool, &sk, "1")?;
        let rounding = self.uncertainty_rounding(ui, tool, key, "")?;
        let kind = match kind {
            0 => {
                let range_method = self.select_index(
                    ui,
                    tool,
                    &format!("{key}.a_method"),
                    "A 类计算方法",
                    &["贝塞尔法", "极差法"],
                    0,
                ) == 1;
                let count = self.dynamic_count(
                    ui,
                    tool,
                    &format!("{key}.samples"),
                    "测量数据",
                    5,
                    2,
                    if range_method { 9 } else { 50 },
                );
                let values = self.dynamic_values(
                    ui,
                    tool,
                    &format!("{key}.sample"),
                    count,
                    "测量值",
                    &["10.02", "10.01", "10.03", "10.02", "10.01"],
                    "输入测量值",
                )?;
                let average_count = if range_method {
                    None
                } else {
                    let field = format!("{key}.average_count");
                    self.input(ui, tool, &field, "取测量次数 n", "", "留空默认 1 次");
                    model::average_count(&self.value(tool, &field, ""))?
                };
                Kind::TypeA {
                    values,
                    range_method,
                    average_count,
                }
            }
            2 => {
                let maximum = self.select_index(
                    ui,
                    tool,
                    &format!("{key}.method"),
                    "合成方法",
                    &["平方和根", "最大绝对贡献"],
                    0,
                ) == 1;
                let n =
                    self.dynamic_count(ui, tool, &format!("{key}.children"), "子分量", 2, 1, 12);
                let mut children = Vec::new();
                for i in 0..n {
                    let k = format!("{key}.child.{i}");
                    let mut value = None;
                    egui::CollapsingHeader::new(format!("子分量 {}", i + 1))
                        .id_salt(&k)
                        .default_open(true)
                        .show(ui, |ui| {
                            value = Some(self.uncertainty_component(ui, tool, &k, depth + 1));
                        });
                    // 折叠节点仍参与计算，但不绘制其控件。
                    if let Some(value) = value {
                        children.push(value?);
                    } else {
                        children.push(self.uncertainty_component_data(tool, &k, depth + 1)?);
                    }
                }
                Kind::Combined { children, maximum }
            }
            _ => {
                let distribution = self.select_index(
                    ui,
                    tool,
                    &format!("{key}.distribution"),
                    "分布类型",
                    &[
                        "均匀分布",
                        "三角分布",
                        "正态 k=2",
                        "正态 k=3",
                        "两点分布",
                        "反正弦分布",
                    ],
                    0,
                );
                let hk = format!("{key}.half_width");
                let label = if matches!(distribution, 2 | 3) {
                    "证书扩展不确定度 U"
                } else {
                    "区间半宽"
                };
                self.input(ui, tool, &hk, label, "0.02", "须非负");
                let dof_key = format!("{key}.dof");
                self.input(
                    ui,
                    tool,
                    &dof_key,
                    "自由度 ν（选填）",
                    "",
                    "留空表示 ∞（完全可靠）",
                );
                Kind::TypeB {
                    half_width: self.number(tool, &hk, "0.02")?,
                    distribution,
                    dof: model::parse_dof(&self.value(tool, &dof_key, ""))
                        .map_err(|e| format!("{name}：{e}"))?,
                }
            }
        };
        Ok(Component {
            name,
            sensitivity,
            kind,
            rounding: Some(rounding),
        })
    }

    /// 折叠面板只影响绘制，仍从相同字段读取并计算，不能让隐藏分量退出预算。
    fn uncertainty_component_data(
        &mut self,
        tool: &ToolDef,
        key: &str,
        depth: usize,
    ) -> Result<Component, String> {
        let idx = |app: &mut Self, k: &str, default: &str| {
            app.value(tool, &format!("{key}.{k}"), default)
                .parse::<usize>()
                .unwrap_or(0)
        };
        let kind = match idx(self, "type", "1") {
            0 => {
                let range_method = idx(self, "a_method", "0") == 1;
                let n = idx(self, "samples.count", "5").clamp(2, if range_method { 9 } else { 50 });
                let defaults = ["10.02", "10.01", "10.03", "10.02", "10.01"];
                let mut values = Vec::new();
                for i in 0..n {
                    values.push(self.number(
                        tool,
                        &format!("{key}.sample.{i}"),
                        defaults.get(i).unwrap_or(&"0"),
                    )?);
                }
                Kind::TypeA {
                    values,
                    range_method,
                    average_count: if range_method {
                        None
                    } else {
                        model::average_count(&self.value(
                            tool,
                            &format!("{key}.average_count"),
                            "",
                        ))?
                    },
                }
            }
            2 if depth < 4 => {
                let n = idx(self, "children.count", "2").clamp(1, 12);
                let maximum = idx(self, "method", "0") == 1;
                let mut children = Vec::new();
                for i in 0..n {
                    children.push(self.uncertainty_component_data(
                        tool,
                        &format!("{key}.child.{i}"),
                        depth + 1,
                    )?);
                }
                Kind::Combined { children, maximum }
            }
            _ => {
                let comp_name = self.value(tool, &format!("{key}.name"), "分量");
                Kind::TypeB {
                    half_width: self.number(tool, &format!("{key}.half_width"), "0.02")?,
                    distribution: idx(self, "distribution", "0"),
                    dof: model::parse_dof(&self.value(tool, &format!("{key}.dof"), ""))
                        .map_err(|e| format!("{comp_name}：{e}"))?,
                }
            }
        };
        Ok(Component {
            name: self.value(tool, &format!("{key}.name"), "分量"),
            sensitivity: self.number(tool, &format!("{key}.sensitivity"), "1")?,
            kind,
            rounding: Some(Rounding {
                digits: model::significant_digits(&self.value(tool, &format!("{key}.digits"), ""))?,
                carry: idx(self, "rounding", "1").min(1) == 1,
            }),
        })
    }

    fn uncertainty_component_card(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        index: usize,
    ) -> Result<(Component, Detail), String> {
        let key = format!("budget.component.{index}");
        let component = panel(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(format!("分量 {}", index + 1))
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(4.0);
            self.uncertainty_component(ui, tool, &key, 0)
        })
        .inner?;
        let detail = component.calculate()?;
        Ok((component, detail))
    }

    pub(crate) fn render_uncertainty(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = crate::theme::palette(ui.ctx());
        let mode = panel(ui, |ui| {
            ui.set_width(ui.available_width());
            self.select_index(
                ui,
                tool,
                "budget.mode",
                "评定方式",
                &["分量预算表", "A/B 类快速评定"],
                0,
            )
        })
        .inner;
        ui.add_space(10.0);
        if mode == 1 {
            self.render_simple_uncertainty(ui, tool);
            return;
        }
        let count = panel(ui, |ui| {
            ui.set_width(ui.available_width());
            self.dynamic_count(ui, tool, "budget.components", "不确定度分量", 2, 1, 20)
        })
        .inner;
        ui.add_space(10.0);
        let mut results = Vec::with_capacity(count);
        balanced_columns(ui, "uncertainty_component_columns", count, |ui, i| {
            results.push(self.uncertainty_component_card(ui, tool, i));
        });
        let results = results.into_iter().collect::<Result<Vec<_>, _>>();

        let mut process_lines = None;
        split_workspace(
            ui,
            0.38,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new("总合成设置")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(8.0);
                    let maximum = app.select_index(
                        ui,
                        tool,
                        "budget.combine",
                        "总合成方法",
                        &["平方和根", "最大绝对贡献"],
                        0,
                    ) == 1;
                    let effective_dof = results.as_ref().map_err(String::clone).and_then(|rows| {
                        let terms: Vec<_> =
                            rows.iter().map(|(_, d)| (d.contribution, d.dof)).collect();
                        model::combine_dof(&terms, maximum)
                    });
                    let needs_manual = maximum
                        || matches!(effective_dof, Ok(model::DegreesOfFreedom::NotApplicable));
                    let coverage_mode = if needs_manual {
                        app.set_derived_value(tool, "budget.coverage_mode", "0".into());
                        ui.label("有效自由度不适用，使用手动包含因子。");
                        0
                    } else {
                        app.select_index(
                            ui,
                            tool,
                            "budget.coverage_mode",
                            "包含因子方式",
                            &["手动指定 k", "t 分布近似（95%）", "t 分布近似（99%）"],
                            0,
                        )
                    };
                    if coverage_mode == 0 {
                        app.input(ui, tool, "budget.k", "包含因子 k", "2", "须大于 0");
                    }
                    let digits = app.select_index(
                        ui,
                        tool,
                        "budget.digits",
                        "合成结果有效数字",
                        &["1 位", "2 位", "3 位", "4 位"],
                        1,
                    ) + 1;
                    let carry = app.select_index(
                        ui,
                        tool,
                        "budget.rounding_v2",
                        "合成结果修约方式",
                        &["四舍六入", "进位"],
                        1,
                    ) == 1;
                    (maximum, coverage_mode, digits, carry, effective_dof)
                })
                .inner
            },
            |ui, app, (maximum, coverage_mode, digits, carry, effective_dof)| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let rows = results.as_ref().map_err(String::clone)?;
                        let uc = model::combine(
                            &rows.iter().map(|(_, d)| d.contribution).collect::<Vec<_>>(),
                            maximum,
                        )?;
                        let effective_dof = effective_dof?;
                        let manual_k = if coverage_mode == 0 {
                            app.number(tool, "budget.k", "2")?
                        } else {
                            2.0
                        };
                        let k = model::coverage_factor(coverage_mode, manual_k, effective_dof)?;
                        let expanded = uc * k;
                        super::numerics::finite(&[expanded])?;
                        let (rounded, rounded_expanded) =
                            model::rounded_combined(uc, k, digits, carry)?;
                        let mut steps = Vec::new();
                        for (component, detail) in rows {
                            process::component(component, detail, "", &mut steps);
                        }
                        steps.push(String::new());
                        steps.push(format!(
                            "总合成方法：{}；u_c = {}。",
                            if maximum {
                                "取各分量绝对贡献的最大值"
                            } else {
                                "各分量贡献平方和根（使用 hypot 累加）"
                            },
                            format_process_computed(uc)
                        ));
                        steps.push(format!(
                            "有效自由度 ν_eff = {}。",
                            model::format_dof(effective_dof)
                        ));
                        if coverage_mode != 0 {
                            steps.push(super::error_process::coverage(coverage_mode, k));
                        }
                        super::error_process::combined(
                            &mut steps,
                            uc,
                            k,
                            digits,
                            carry,
                            rounded,
                            rounded_expanded,
                        );
                        process_lines = Some(steps);
                        Ok(vec![
                            ("合成标准不确定度 u_c", format_number(uc)),
                            ("扩展不确定度 U", format_number(expanded)),
                            ("修约后标准不确定度", format_number(rounded)),
                            ("修约后扩展不确定度", format_number(rounded_expanded)),
                            ("有效自由度 ν_eff", model::format_dof(effective_dof)),
                            ("包含因子 k", format_number(k)),
                        ])
                    })();
                    let success = result.is_ok();
                    show_result_or_error(ui, "不确定度结果", result);
                    if success && let Ok(rows) = &results {
                        fn show(ui: &mut egui::Ui, items: &[Detail]) {
                            for d in items {
                                ui.label(format!(
                                    "{}：u={}，修约后 u={}，c={}，贡献={}，ν={}",
                                    d.name,
                                    format_number(d.raw_standard),
                                    model::format_rounded(d.standard, d.rounding),
                                    format_number(d.sensitivity),
                                    format_number(d.contribution),
                                    model::format_dof(d.dof)
                                ));
                                if !d.children.is_empty() {
                                    ui.indent(&d.name, |ui| show(ui, &d.children));
                                }
                            }
                        }
                        ui.add_space(6.0);
                        ui.collapsing("分量计算明细", |ui| {
                            for (_, detail) in rows {
                                show(ui, std::slice::from_ref(detail));
                            }
                        });
                    }
                });
            },
        );
        if let Some(lines) = &process_lines {
            calculation_process(ui, "uncertainty_budget_process", lines);
        }
        self.calculation_note(ui, tool);
    }
}
