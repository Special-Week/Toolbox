//! 标准热电偶参数、校准拟合和参考端补偿界面；热电势一律使用 mV。
use super::standard::{StandardCouple, zones};
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
impl ToolboxApp {
    pub(crate) fn render_standard_thermocouple(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(820.0);
        let mut table = None;
        let result = ui
            .group(|ui| {
                let k = self.select_index(ui, tool, "stc.type", "分度号", &["S 型", "B 型"], 0);
                let kind = ["S", "B"][k];
                let mut zone_id = 0;
                let mode = self.select_index(
                    ui,
                    tool,
                    "stc.calibration",
                    "校准方式",
                    &["固定点实测热电势", "温区多项式系数"],
                    0,
                );
                let mut model = StandardCouple {
                    kind,
                    points: Vec::new(),
                    zone: None,
                };
                if mode == 0 {
                    let defaults: &[(f64, f64)] = if k == 0 {
                        &[(419.527, 3.4472), (660.323, 5.8605), (1084.62, 10.5822)]
                    } else {
                        &[
                            (1100., 5.8029),
                            (1200., 6.8050),
                            (1300., 7.8809),
                            (1400., 8.9926),
                            (1500., 10.1205),
                        ]
                    };
                    for (i, &(t, e)) in defaults.iter().enumerate() {
                        let key = format!("stc.{kind}.cal.{i}");
                        let d = e.to_string();
                        self.input(
                            ui,
                            tool,
                            &key,
                            &format!("{} °C · 实测电势 (mV)", format_number(t)),
                            &d,
                            "证书中的实际电势，不是修正量",
                        );
                        model.points.push((t, self.number(tool, &key, &d)?));
                    }
                } else {
                    let ranges = zones(kind);
                    let labels: Vec<_> = ranges
                        .iter()
                        .map(|(a, b, _)| {
                            format!("{} ～ {} °C", format_number(*a), format_number(*b))
                        })
                        .collect();
                    let z = self.select_index(
                        ui,
                        tool,
                        &format!("stc.{kind}.zone"),
                        "系数温区",
                        &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                        0,
                    );
                    zone_id = z;
                    let mut zone = ranges[z].clone();
                    let fit = self.select_index(
                        ui,
                        tool,
                        "stc.coefficient_source",
                        "系数设置",
                        &["手动输入系数", "校准点拟合"],
                        0,
                    ) == 1;
                    if fit {
                        let count =
                            self.dynamic_count(ui, tool, "stc.fit.points", "校准点", 3, 2, 30);
                        let mut points = Vec::new();
                        for i in 0..count {
                            let t =
                                zone.0 + (zone.1 - zone.0) * (i + 1) as f64 / (count + 1) as f64;
                            let e = crate::calculators::calibration::polynomial(&zone.2, t);
                            let key = format!("stc.fit.{kind}.{z}.{i}");
                            self.input(
                                ui,
                                tool,
                                &format!("{key}.t"),
                                &format!("点 {} 温度 (°C)", i + 1),
                                &t.to_string(),
                                "校准温度",
                            );
                            self.input(
                                ui,
                                tool,
                                &format!("{key}.e"),
                                &format!("点 {} 电势 (mV)", i + 1),
                                &e.to_string(),
                                "实际热电势",
                            );
                            points.push((
                                self.number(tool, &format!("{key}.t"), &t.to_string())?,
                                self.number(tool, &format!("{key}.e"), &e.to_string())?,
                            ));
                        }
                        StandardCouple::fit_zone_coefficients(&mut zone, &points)?;
                        ui.collapsing("拟合系数与残差", |ui| {
                            for (i, c) in zone.2.iter().enumerate() {
                                ui.label(format!("c[{i}] = {}", format_number(*c)));
                            }
                            for (i, &(t, e)) in points.iter().enumerate() {
                                ui.label(format!(
                                    "点 {} 电势残差：{} mV",
                                    i + 1,
                                    format_number(
                                        e - crate::calculators::calibration::polynomial(&zone.2, t)
                                    )
                                ));
                            }
                        });
                    } else {
                        for (i, c) in zone.2.iter_mut().enumerate() {
                            let key = format!("stc.{kind}.{z}.c.{i}");
                            let d = c.to_string();
                            self.input(
                                ui,
                                tool,
                                &key,
                                &format!("系数 c[{i}]"),
                                &d,
                                "E(t) = Σ c[i]·t^i；t 为 °C，E 为 mV",
                            );
                            *c = self.number(tool, &key, &d)?;
                        }
                    }
                    model.zone = Some(zone);
                }
                let u = TemperatureUnit(self.select_index(
                    ui,
                    tool,
                    "stc.unit",
                    "温度单位",
                    &["°C", "K", "°F"],
                    0,
                ));
                let direction = self.select_index(
                    ui,
                    tool,
                    "stc.direction",
                    "计算方式",
                    &["温度 → 热电势", "热电势 → 温度"],
                    0,
                );
                let def_t = model
                    .zone
                    .as_ref()
                    .map(|(a, b, _)| (a + b) / 2.0)
                    .unwrap_or(if k == 0 { 1000.0 } else { 1200.0 });
                let key = if direction == 0 {
                    format!("stc.{kind}.{mode}.{zone_id}.t.{}", u.0)
                } else {
                    format!("stc.{kind}.{mode}.{zone_id}.emf")
                };
                let default = if direction == 0 {
                    u.display(def_t).to_string()
                } else {
                    model.emf(def_t)?.to_string()
                };
                self.input(
                    ui,
                    tool,
                    &key,
                    if direction == 0 {
                        u.symbol()
                    } else {
                        "测得热电势 (mV)"
                    },
                    &default,
                    "输入待换算值",
                );
                let ck = format!("stc.cold.{}", u.0);
                let cd = u.display(0.0).to_string();
                self.input(
                    ui,
                    tool,
                    &ck,
                    &format!("参考端温度 ({})", u.symbol()),
                    &cd,
                    "多项式温区以外的参考端请使用 0 °C",
                );
                let cold = u.celsius(self.number(tool, &ck, &cd)?);
                let cold_e = model.emf(cold)?;
                let v = self.number(tool, &key, &default)?;
                let (t, e) = if direction == 0 {
                    let t = u.celsius(v);
                    (t, model.emf(t)? - cold_e)
                } else {
                    (model.temperature(v + cold_e)?, v)
                };
                let h = 0.01;
                let (lo, hi) = model
                    .zone
                    .as_ref()
                    .map(|(a, b, _)| (*a, *b))
                    .unwrap_or(if k == 0 {
                        (-50.0, 1768.1)
                    } else {
                        (0.0, 1820.0)
                    });
                let a = (t - h).max(lo);
                let b = (t + h).min(hi);
                let slope = (model.emf(b)? - model.emf(a)?) / (b - a);
                table = Some((model.clone(), u, lo, hi, cold_e, zone_id));
                Ok(vec![
                    (
                        "热端温度",
                        format!("{} {}", format_number(u.display(t)), u.symbol()),
                    ),
                    ("测得热电势", format!("{} mV", format_number(e))),
                    (
                        "微分热电势 dE/dt",
                        format!("{} mV/{}", format_number(slope / u.span()), u.symbol()),
                    ),
                    ("参考端电势", format!("{} mV", format_number(cold_e))),
                ])
            })
            .inner;
        show_result_or_error(ui, "换算结果", result);
        if let Some((model, u, lo, hi, cold, z)) = table {
            self.render_division_table(
                ui,
                tool,
                &format!("standard.{}.{z}", model.kind),
                (lo, hi),
                u,
                "mV",
                |t| model.emf(t).map(|e| e - cold),
            );
        }
        self.calculation_note(ui, tool);
    }
}
