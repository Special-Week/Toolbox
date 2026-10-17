//! 标准铂电阻页面；按子区间限制计算温区，并分离证书系数和固定点拟合输入。
mod curves;
pub(crate) mod model;
mod ranges;
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
use model::Sprt;
use ranges::LOW_RANGES;
impl ToolboxApp {
    pub(crate) fn render_standard_resistance(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(820.0);
        if self.select_index(
            ui,
            tool,
            "sprt.model",
            "计算模型",
            &["标准铂电阻（SPRT 换算）", "CVD 系数拟合"],
            0,
        ) == 1
        {
            self.render_cvd_fit(ui, tool);
            return;
        }
        let mut table = None;
        let result = ui
            .group(|ui| {
                let hr = self.select_index(
                    ui,
                    tool,
                    "sprt.hr",
                    "高温分段",
                    &[
                        "A6 / B6 / C6：至 961.78 °C",
                        "A7 / B7 / C7：至 660.323 °C",
                        "A8 / B8：至 419.527 °C",
                        "A9 / B9：至 231.928 °C",
                        "A10：至 156.599 °C",
                        "A11：至 29.765 °C",
                        "不使用高温分区",
                    ],
                    1,
                );
                let lr = self.select_index(
                    ui,
                    tool,
                    "sprt.lr",
                    "低温分区",
                    &LOW_RANGES.iter().map(|r| r.label).collect::<Vec<_>>(),
                    0,
                );
                let low_range = &LOW_RANGES[lr];
                if hr == 6 && lr == 2 {
                    return Err("高温和低温分区不能同时不使用".into());
                }
                let full = self.select_index(
                    ui,
                    tool,
                    "sprt.deviation",
                    "偏差函数",
                    &["ITS-90 子区间偏差函数", "三项高温 / 对数低温偏差"],
                    0,
                ) == 0;
                let source = self.select_index(
                    ui,
                    tool,
                    "sprt.source",
                    "系数设置",
                    &["输入证书系数", "固定点电阻拟合"],
                    0,
                );
                let mut m = Sprt {
                    low_quadratic: full && lr == 1,
                    ..Sprt::default()
                };
                self.input(
                    ui,
                    tool,
                    "sprt.rtp",
                    "水三相点电阻 Rtp (Ω)",
                    &m.rtp.to_string(),
                    "0.01 °C 时的电阻",
                );
                m.rtp = self.number(tool, "sprt.rtp", &m.rtp.to_string())?;
                if source == 0 {
                    for (key, label, item) in if hr == 6 {
                        vec![]
                    } else {
                        vec![
                            ("a", "a", &mut m.a),
                            ("b", "b", &mut m.b),
                            ("c", "c", &mut m.c),
                        ]
                    } {
                        let k = format!("sprt.coefficient.{key}");
                        let d = item.to_string();
                        self.input(ui, tool, &k, label, &d, "证书系数");
                        *item = self.number(tool, &k, &d)?;
                    }
                    if let Some([a_label, b_label]) = low_range.coefficients {
                        for (label, item) in [(a_label, &mut m.a4), (b_label, &mut m.b4)] {
                            let k = format!("sprt.coefficient.{label}");
                            let d = item.to_string();
                            self.input(ui, tool, &k, label, &d, "所选低温分区的证书系数");
                            *item = self.number(tool, &k, &d)?;
                        }
                    }
                    if full && hr == 0 {
                        let w = m.resistance(660.323)? / m.rtp;
                        self.input(ui, tool, "sprt.coefficient.d", "d6", "0", "银点偏差系数");
                        self.input(
                            ui,
                            tool,
                            "sprt.coefficient.w660",
                            "W660",
                            &w.to_string(),
                            "铝点电阻 / 水三相点电阻",
                        );
                        m.d = self.number(tool, "sprt.coefficient.d", "0")?;
                        m.w660 = self.number(tool, "sprt.coefficient.w660", &w.to_string())?;
                        if m.w660 <= 1.0 {
                            return Err("W660 须大于 1".into());
                        }
                    }
                } else {
                    let highs: Vec<f64> = match hr {
                        0 | 1 => vec![231.928, 419.527, 660.323],
                        2 => vec![231.928, 419.527],
                        3 => vec![156.599, 231.928],
                        4 => vec![156.599],
                        5 => vec![29.765],
                        _ => vec![],
                    };
                    let mut hp = Vec::new();
                    let mut lp = Vec::new();
                    for (list, out) in [
                        (highs.as_slice(), &mut hp),
                        (low_range.fixed_points, &mut lp),
                    ] {
                        for &t in list {
                            let key = format!("sprt.fixed.{t}");
                            let d = m.resistance(t)?.to_string();
                            self.input(
                                ui,
                                tool,
                                &key,
                                &format!("{} °C · 实测电阻 (Ω)", format_number(t)),
                                &d,
                                "固定点证书电阻",
                            );
                            out.push((t, self.number(tool, &key, &d)?));
                        }
                    }
                    m.fit(
                        &hp,
                        &lp,
                        if hr <= 1 {
                            3
                        } else if hr <= 3 {
                            2
                        } else {
                            1
                        },
                    )?;
                    if full && hr == 0 {
                        m.w660 = hp
                            .iter()
                            .find(|(t, _)| *t == 660.323)
                            .ok_or("缺少铝点数据")?
                            .1
                            / m.rtp;
                        let default = m.resistance(961.78)?.to_string();
                        self.input(
                            ui,
                            tool,
                            "sprt.fixed.961.78",
                            "961.78 °C · 银点实测电阻 (Ω)",
                            &default,
                            "银点证书电阻",
                        );
                        let resistance = self.number(tool, "sprt.fixed.961.78", &default)?;
                        m.fit_silver(961.78, resistance)?;
                        hp.push((961.78, resistance));
                    }
                    let mut coefficients = vec![
                        ("a", format_number(m.a)),
                        ("b", format_number(m.b)),
                        ("c", format_number(m.c)),
                    ];
                    if let Some([a_label, b_label]) = low_range.coefficients {
                        coefficients.extend([
                            (a_label, format_number(m.a4)),
                            (b_label, format_number(m.b4)),
                        ]);
                    }
                    if full && hr == 0 {
                        coefficients
                            .extend([("d6", format_number(m.d)), ("W660", format_number(m.w660))]);
                    }
                    render_metrics(ui, "拟合系数", &coefficients);
                    ui.collapsing("固定点拟合残差", |ui| {
                        for &(t, r) in hp.iter().chain(&lp) {
                            if let Ok(actual) = m.temperature(r) {
                                ui.label(format!(
                                    "{} °C：{} °C（证书温度 − 反算温度）",
                                    format_number(t),
                                    format_number(t - actual)
                                ));
                            }
                        }
                    });
                }
                m.validate()?;
                let u = TemperatureUnit(self.select_index(
                    ui,
                    tool,
                    "sprt.unit",
                    "温度单位",
                    &["°C", "K", "°F"],
                    0,
                ));
                let direction = self.select_index(
                    ui,
                    tool,
                    "sprt.direction",
                    "计算方式",
                    &["温度 → 电阻", "电阻 → 温度"],
                    0,
                );
                let key = if direction == 0 {
                    format!("sprt.range.{hr}.{lr}.t.{}", u.0)
                } else {
                    format!("sprt.range.{hr}.{lr}.r")
                };
                let d = if direction == 0 {
                    u.display(if hr == 6 { -20.0 } else { 20.0 }).to_string()
                } else {
                    m.resistance(if hr == 6 { -20.0 } else { 20.0 })?
                        .to_string()
                };
                self.input(
                    ui,
                    tool,
                    &key,
                    if direction == 0 {
                        u.symbol()
                    } else {
                        "电阻 (Ω)"
                    },
                    &d,
                    "输入换算值",
                );
                let v = self.number(tool, &key, &d)?;
                let (t, r) = if direction == 0 {
                    let t = u.celsius(v);
                    (t, m.resistance_branch(t, hr != 6 && t >= 0.0)?)
                } else {
                    (
                        if hr == 6 {
                            m.temperature_branch(v, false)?
                        } else {
                            m.temperature(v)?
                        },
                        v,
                    )
                };
                let min = low_range.minimum;
                let max = [961.78, 660.323, 419.527, 231.928, 156.599, 29.765, 0.0][hr];
                if t < min - 1e-8 || t > max + 1e-8 {
                    return Err(format!(
                        "所选分段范围为 {}～{} °C",
                        format_number(min),
                        format_number(max)
                    ));
                }
                // At endpoints use a one-sided difference within the model domain.
                let lo = (t - 1.0).max(min);
                let hi = (t + 1.0).min(max);
                let curve = |t| m.resistance_branch(t, hr != 6 && t >= 0.0);
                let slope = (curve(hi)? - curve(lo)?) / (hi - lo);
                table = Some((m, u, min, max, hr, lr));
                Ok(vec![
                    (
                        "温度",
                        format!("{} {}", format_number(u.display(t)), u.symbol()),
                    ),
                    ("电阻", format!("{} Ω", format_number(r))),
                    ("电阻比 W = R / Rtp", format_number(r / m.rtp)),
                    (
                        "电阻变化率 dR/dt",
                        format!("{} Ω/{}", format_number(slope / u.span()), u.symbol()),
                    ),
                    (
                        "电阻比变化率 dW/dt",
                        format_number(slope / m.rtp / u.span()),
                    ),
                ])
            })
            .inner;
        show_result_or_error(ui, "换算结果", result);
        if let Some((m, u, min, max, hr, lr)) = table {
            self.render_division_table(
                ui,
                tool,
                &format!("sprt.{hr}.{lr}"),
                (min, max),
                u,
                "Ω",
                |t| m.resistance_branch(t, hr != 6 && t >= 0.0),
            );
        }
        info_note(ui, tool.kind);
    }
}
