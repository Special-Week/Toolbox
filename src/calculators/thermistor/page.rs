//! 热敏电阻页面：手动系数或校准点拟合，以及独立的 β 模型入口。
//! 系数编辑器折叠时仍读取已保存输入；显示温标不改变校准多项式的单位。
use super::{BetaThermistor, polynomial::PolynomialThermistor};
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
impl ToolboxApp {
    pub(crate) fn render_thermistor(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(820.0);
        let family = self.select_index(
            ui,
            tool,
            "ntc.model",
            "计算模型",
            &["多项式 / Steinhart–Hart", "β 模型"],
            0,
        );
        if family == 1 {
            self.render_beta_thermistor(ui, tool);
            return;
        }
        let mut table = None;
        let result = ui
            .group(|ui| {
                let sh = self.select_index(
                    ui,
                    tool,
                    "ntc.sh",
                    "多项式形式",
                    &["完整多项式", "Steinhart–Hart（去除二次项）"],
                    0,
                ) == 1;
                let order = self.select_index(
                    ui,
                    tool,
                    "ntc.order",
                    "多项式阶数",
                    &["3 阶", "4 阶", "5 阶", "6 阶"],
                    0,
                ) + 3;
                let source = self.select_index(
                    ui,
                    tool,
                    "ntc.source",
                    "系数设置",
                    &["手动输入系数", "校准点最小二乘拟合"],
                    0,
                );
                let mut residuals = Vec::new();
                let model = if source == 0 {
                    self.read_thermistor_coefficients(ui, tool, order, sh)?
                } else {
                    // 完整 n 阶多项式需 n+1 个独立点，去除二次项后少一个未知量。
                    let count = self.dynamic_count(
                        ui,
                        tool,
                        "ntc.points",
                        "校准点",
                        7,
                        if sh { order } else { order + 1 },
                        30,
                    );
                    let beta = BetaThermistor::new(10000.0, 25.0, 3950.0)?;
                    let mut points = Vec::new();
                    for i in 0..count {
                        let t = -20.0 + i as f64 * 20.0;
                        let tk = format!("ntc.point.{i}.t");
                        let rk = format!("ntc.point.{i}.r");
                        let default_r = beta.resistance(t)?.to_string();
                        self.input(
                            ui,
                            tool,
                            &tk,
                            &format!("校准点 {} · 温度 (°C)", i + 1),
                            &t.to_string(),
                            "证书温度",
                        );
                        self.input(
                            ui,
                            tool,
                            &rk,
                            &format!("校准点 {} · 电阻 (Ω)", i + 1),
                            &default_r,
                            "证书电阻",
                        );
                        points.push((
                            self.number(tool, &tk, &t.to_string())?,
                            self.number(tool, &rk, &default_r)?,
                        ));
                    }
                    let model = PolynomialThermistor::fit(&points, order, sh)?;
                    for (i, (t, r)) in points.iter().enumerate() {
                        residuals.push((
                            format!("点 {} 温度残差", i + 1),
                            format!("{} °C", format_number(t - model.temperature(*r)?)),
                        ));
                    }
                    model
                };
                if source == 1 {
                    let mut coefficients = Vec::new();
                    for (name, values) in [("A", &model.a), ("B", &model.b)] {
                        for (i, coefficient) in values.iter().enumerate() {
                            coefficients
                                .push((format!("{name}[{i}]"), format_number(*coefficient)));
                        }
                    }
                    ui.collapsing("拟合系数与校准点残差", |ui| {
                        render_metrics(
                            ui,
                            "拟合系数",
                            &coefficients
                                .iter()
                                .map(|(a, b)| (a.as_str(), b.clone()))
                                .collect::<Vec<_>>(),
                        );
                        render_metrics(
                            ui,
                            "残差 = 校准温度 − 电阻反算温度",
                            &residuals
                                .iter()
                                .map(|(a, b)| (a.as_str(), b.clone()))
                                .collect::<Vec<_>>(),
                        );
                    });
                }
                let unit = TemperatureUnit(self.select_index(
                    ui,
                    tool,
                    "ntc.unit",
                    "温度单位",
                    &["°C", "K", "°F"],
                    0,
                ));
                let direction = self.select_index(
                    ui,
                    tool,
                    "mode",
                    "计算方式",
                    &["电阻 → 温度", "温度 → 电阻"],
                    0,
                );
                let key = if direction == 0 {
                    "resistance".into()
                } else {
                    format!("ntc.temperature.{}", unit.0)
                };
                let default = if direction == 0 {
                    "10000".into()
                } else {
                    unit.display(25.0).to_string()
                };
                self.input(
                    ui,
                    tool,
                    &key,
                    if direction == 0 {
                        "电阻 (Ω)"
                    } else {
                        unit.symbol()
                    },
                    &default,
                    "输入待换算值",
                );
                let input = self.number(tool, &key, &default)?;
                let (t, r) = if direction == 0 {
                    (model.temperature(input)?, input)
                } else {
                    let t = unit.celsius(input);
                    (t, model.resistance(t)?)
                };
                let slope = (model.resistance(t + 1.0)? - model.resistance(t - 1.0)?) / 2.0;
                self.input(
                    ui,
                    tool,
                    "ntc.range.min",
                    "校准温区下限 (°C)",
                    "-20",
                    "按证书填写",
                );
                self.input(
                    ui,
                    tool,
                    "ntc.range.max",
                    "校准温区上限 (°C)",
                    "100",
                    "按证书填写",
                );
                let min = self.number(tool, "ntc.range.min", "-20")?;
                let max = self.number(tool, "ntc.range.max", "100")?;
                if min >= max || t < min || t > max {
                    return Err("温度须处于所填校准温区内，温区下限须小于上限".into());
                }
                table = Some((model, unit, min, max));
                Ok(vec![
                    (
                        "温度",
                        format!("{} {}", format_number(unit.display(t)), unit.symbol()),
                    ),
                    ("电阻", format!("{} Ω", format_number(r))),
                    (
                        "电阻变化率",
                        format!("{} Ω/{}", format_number(slope / unit.span()), unit.symbol()),
                    ),
                ])
            })
            .inner;
        show_result_or_error(ui, "换算结果", result);
        if let Some((m, u, min, max)) = table {
            self.render_division_table(ui, tool, "ntc", (min, max), u, "Ω", |t| m.resistance(t));
        }
        self.calculation_note(ui, tool);
    }

    /// 编辑器是否展开只影响绘制，所有生效系数始终从状态中读取。
    fn read_thermistor_coefficients(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        order: usize,
        sh: bool,
    ) -> Result<PolynomialThermistor, String> {
        ui.label("A：1/T = Σ A[i]·ln(R)^i；B：ln(R) = Σ B[i]·(1/T)^i");
        ui.label("示例系数对应 10 kΩ、25 °C、β=3950 K；实际使用请填写证书系数。T 用 K，R 用 Ω。");
        let a0 = 1.0 / 298.15 - 10000.0_f64.ln() / 3950.0;
        let b0 = 10000.0_f64.ln() - 3950.0 / 298.15;
        let mut a = vec![0.0; order + 1];
        let mut b = a.clone();
        for (prefix, coefficients, defaults) in [
            ("a", &mut a, [a0, 1.0 / 3950.0]),
            ("b", &mut b, [b0, 3950.0]),
        ] {
            ui.collapsing(
                if prefix == "a" {
                    "电阻 → 温度系数 A"
                } else {
                    "温度 → 电阻系数 B"
                },
                |ui| {
                    for (i, item) in coefficients.iter_mut().enumerate() {
                        if sh && i == 2 {
                            continue;
                        }
                        let key = format!("ntc.{prefix}.{order}.{sh}.{i}");
                        let default = defaults.get(i).copied().unwrap_or(0.0).to_string();
                        self.input(
                            ui,
                            tool,
                            &key,
                            &format!("{}[{i}]", prefix.to_uppercase()),
                            &default,
                            "证书系数，按升幂排列",
                        );
                        *item = self.number(tool, &key, &default)?;
                    }
                    Ok::<_, String>(())
                },
            )
            .body_returned
            .unwrap_or(Ok(()))?;
            for (i, item) in coefficients.iter_mut().enumerate() {
                if !sh || i != 2 {
                    *item = self.number(
                        tool,
                        &format!("ntc.{prefix}.{order}.{sh}.{i}"),
                        &defaults.get(i).copied().unwrap_or(0.0).to_string(),
                    )?;
                }
            }
        }
        Ok(PolynomialThermistor { a, b })
    }
}
