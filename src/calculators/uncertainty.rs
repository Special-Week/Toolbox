//! 可嵌套不确定度预算表；递归深度和分量数量设有上限，控制页面与计算规模。
pub(crate) mod model;
use crate::calculators::common::*;
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
use model::{Component, Detail, Kind};

impl ToolboxApp {
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
                Kind::TypeA {
                    values,
                    range_method,
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
                    // Calculate collapsed nodes without drawing their controls.
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
                Kind::TypeB {
                    half_width: self.number(tool, &hk, "0.02")?,
                    distribution,
                }
            }
        };
        Ok(Component {
            name,
            sensitivity,
            kind,
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
            _ => Kind::TypeB {
                half_width: self.number(tool, &format!("{key}.half_width"), "0.02")?,
                distribution: idx(self, "distribution", "0"),
            },
        };
        Ok(Component {
            name: self.value(tool, &format!("{key}.name"), "分量"),
            sensitivity: self.number(tool, &format!("{key}.sensitivity"), "1")?,
            kind,
        })
    }

    pub(crate) fn render_uncertainty(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(820.0);
        if self.select_index(
            ui,
            tool,
            "budget.mode",
            "评定方式",
            &["分量预算表", "A/B 类快速评定"],
            0,
        ) == 1
        {
            self.render_simple_uncertainty(ui, tool);
            return;
        }
        let result = (|| {
            let count = self.dynamic_count(ui, tool, "budget.components", "不确定度分量", 2, 1, 20);
            let mut details = Vec::new();
            for i in 0..count {
                let key = format!("budget.component.{i}");
                let data = ui
                    .group(|ui| self.uncertainty_component(ui, tool, &key, 0))
                    .inner?;
                details.push(data.calculate()?);
                ui.add_space(8.0);
            }
            let maximum = self.select_index(
                ui,
                tool,
                "budget.combine",
                "总合成方法",
                &["平方和根", "最大绝对贡献"],
                0,
            ) == 1;
            self.input(ui, tool, "budget.k", "包含因子 k", "2", "须大于 0");
            let k = self.number(tool, "budget.k", "2")?;
            if k <= 0.0 {
                return Err("包含因子须大于 0".into());
            }
            let digits = self.select_index(
                ui,
                tool,
                "budget.digits",
                "有效数字",
                &["1 位", "2 位", "3 位", "4 位"],
                1,
            ) + 1;
            let carry = self.select_index(
                ui,
                tool,
                "budget.rounding",
                "修约方式",
                &["四舍五入", "进位"],
                0,
            ) == 1;
            let uc = model::combine(
                &details.iter().map(|d| d.contribution).collect::<Vec<_>>(),
                maximum,
            )?;
            let expanded = uc * k;
            super::numerics::finite(&[expanded])?;
            let maximum = uc.max(expanded);
            let places = if maximum == 0.0 {
                digits as i32 - 1
            } else {
                (digits as i32 - maximum.log10().floor() as i32 - 1).max(0)
            };
            let rounded = model::rounded(uc, places, carry);
            fn show(ui: &mut egui::Ui, items: &[Detail]) {
                for d in items {
                    ui.label(format!(
                        "{}：u={}，c={}，贡献={}",
                        d.name,
                        format_number(d.standard),
                        format_number(d.sensitivity),
                        format_number(d.contribution)
                    ));
                    if !d.children.is_empty() {
                        ui.indent(&d.name, |ui| show(ui, &d.children));
                    }
                }
            }
            ui.collapsing("分量计算明细", |ui| show(ui, &details));
            Ok(vec![
                ("合成标准不确定度 u_c", format_number(uc)),
                ("扩展不确定度 U", format_number(expanded)),
                ("修约后标准不确定度", format_number(rounded)),
                ("修约后扩展不确定度", format_number(rounded * k)),
                ("包含因子 k", format_number(k)),
            ])
        })();
        show_result_or_error(ui, "不确定度结果", result);
        self.calculation_note(ui, tool);
    }
}
