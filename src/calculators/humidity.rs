//! 湿度页面：四种已知参数共用同一组输出，显示单位在输入/输出边界转换。
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
pub(crate) mod website;
impl ToolboxApp {
    pub(crate) fn render_humidity(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(820.0);
        let result = ui
            .group(|ui| {
                let unit = TemperatureUnit(self.select_index(
                    ui,
                    tool,
                    "unit",
                    "温度单位",
                    &["°C", "K", "°F"],
                    0,
                ));
                let tk = format!("temperature.{}", unit.0);
                let td = unit.display(20.0).to_string();
                self.input(
                    ui,
                    tool,
                    &tk,
                    &format!("环境温度 ({})", unit.symbol()),
                    &td,
                    "−100～100 °C",
                );
                self.input(
                    ui,
                    tool,
                    "pressure",
                    "大气压 (kPa)",
                    "101.325",
                    "当地绝对压力",
                );
                let known = self.select_index(
                    ui,
                    tool,
                    "known",
                    "已知参数",
                    &["相对湿度", "露点 / 霜点", "湿球温度值", "混合比"],
                    0,
                );
                let key = format!(
                    "known.{known}.{}",
                    if known == 1 || known == 2 { unit.0 } else { 0 }
                );
                let label = match known {
                    1 => format!("露点 / 霜点 ({})", unit.symbol()),
                    2 => format!("湿球温度 ({})", unit.symbol()),
                    3 => "混合比 (g/kg)".into(),
                    _ => "相对湿度 (%RH)".into(),
                };
                let default = match known {
                    1 => unit.display(10.0).to_string(),
                    2 => unit.display(15.0).to_string(),
                    3 => "7.5".into(),
                    _ => "50".into(),
                };
                self.input(
                    ui,
                    tool,
                    &key,
                    &label,
                    &default,
                    if known == 1 {
                        "≤0 °C 按霜点输入，>0 °C 按露点输入"
                    } else {
                        "输入已知参数值"
                    },
                );
                let t = unit.celsius(self.number(tool, &tk, &td)?);
                let mut v = self.number(tool, &key, &default)?;
                if known == 1 || known == 2 {
                    v = unit.celsius(v);
                }
                Ok::<_, String>((
                    website::calculate(
                        t,
                        self.number(tool, "pressure", "101.325")? * 1000.0,
                        known,
                        v,
                    )?,
                    unit,
                ))
            })
            .inner;
        match result {
            Err(e) => render_error_message(ui, &e),
            Ok((r, u)) => {
                let temp = |t: Option<f64>| {
                    t.map(|t| format!("{} {}", format_number(u.display(t)), u.symbol()))
                        .unwrap_or("无有限值（干燥空气）".into())
                };
                let mut metrics = vec![
                    ("相对湿度", format!("{} %RH", format_number(r.rh))),
                    ("露点（水面）", temp(r.dew)),
                ];
                if r.frost.is_some_and(|t| t <= 0.0) {
                    metrics.push(("霜点（冰面）", temp(r.frost)));
                }
                metrics.extend([
                    ("饱和水气压", format!("{} Pa", format_number(r.saturated))),
                    ("水气压", format!("{} Pa", format_number(r.vapor))),
                    (
                        "含湿量（体积分数）",
                        format!("{} L/L", format_number(r.volume_fraction)),
                    ),
                    (
                        "体积比（以干空气计）",
                        format!("{} μL/L", format_number(r.ppm)),
                    ),
                    ("绝对湿度", format!("{} g/m³", format_number(r.absolute))),
                    ("质量比", format!("{} μg/g", format_number(r.mass))),
                    ("比湿", format!("{} g/kg", format_number(r.specific))),
                    ("混合比", format!("{} g/kg", format_number(r.mixing))),
                    ("含水量", format!("{} g/Nm³", format_number(r.normal))),
                    ("湿球温度", temp(Some(r.wet))),
                ]);
                render_metrics(ui, "换算结果", &metrics);
            }
        }
        info_note(ui, tool.kind);
    }
}
