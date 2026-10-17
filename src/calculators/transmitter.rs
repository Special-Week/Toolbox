//! 压力、温度和湿度变送器页面；量程、方向及温标分别保存独立输入状态。
use crate::INK;
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use eframe::egui::{self, RichText};
mod model;
use crate::calculators::pressure::model::Range;
use model::Transmitter;

impl ToolboxApp {
    pub(crate) fn render_transmitter(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(
                RichText::new("变送器量程换算")
                    .size(15.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(8.0);

            let quantity = self.select_index(
                ui,
                tool,
                "quantity",
                "被测量类型",
                &["压力", "温度", "湿度"],
                0,
            );
            let unit_key = format!("unit_{quantity}");
            let pressure_options: Vec<_> =
                crate::catalog::units_for(crate::catalog::Dimension::Pressure)
                    .iter()
                    .map(|u| u.code)
                    .collect();
            let unit_options: &[&str] = match quantity {
                0 => &pressure_options,
                1 => &["°C", "°F", "K"],
                _ => &["%RH", "°C（露点）", "g/m³"],
            };
            let unit_index = self.select_index(ui, tool, &unit_key, "量程单位", unit_options, 0);
            let (quantity_key, quantity_name, low_default, high_default) =
                match (quantity, unit_index) {
                    (0, _) => ("pressure", "压力", "0", "100"),
                    (1, 0) => ("temperature_c", "温度", "0", "100"),
                    (1, 1) => ("temperature_f", "温度", "32", "212"),
                    (1, _) => ("temperature_k", "温度", "273.15", "373.15"),
                    (2, 0) => ("humidity_rh", "相对湿度", "0", "100"),
                    (2, 1) => ("humidity_dew", "露点", "-20", "60"),
                    _ => ("humidity_absolute", "绝对湿度", "0", "30"),
                };
            let profile = format!("{quantity_key}.{unit_index}");
            let low_key = format!("{profile}.low");
            let high_key = format!("{profile}.high");
            let mode = self.select_index(
                ui,
                tool,
                "direction",
                "计算方向",
                &["被测量 → 输出信号", "输出信号 → 被测量"],
                0,
            );
            let function =
                self.select_index(ui, tool, "function", "传递特性", &["线性", "开方"], 0);
            let signal_unit = self.select_index(
                ui,
                tool,
                "signal_unit",
                "输出信号单位",
                &["mA", "V", "mV"],
                0,
            );
            let signal_symbol = ["mA", "V", "mV"][signal_unit];
            self.input(
                ui,
                tool,
                &low_key,
                &format!("{quantity_name}下限 ({})", unit_options[unit_index]),
                low_default,
                "变送器量程起点",
            );
            self.input(
                ui,
                tool,
                &high_key,
                &format!("{quantity_name}上限 ({})", unit_options[unit_index]),
                high_default,
                "变送器量程终点",
            );
            self.input(
                ui,
                tool,
                "signal_low",
                &format!("输出信号下限 ({signal_symbol})"),
                "4",
                "例如 4 mA",
            );
            self.input(
                ui,
                tool,
                "signal_high",
                &format!("输出信号上限 ({signal_symbol})"),
                "20",
                "例如 20 mA",
            );

            let input_default = if mode == 0 {
                ((low_default.parse::<f64>().unwrap_or(0.0)
                    + high_default.parse::<f64>().unwrap_or(100.0))
                    / 2.0)
                    .to_string()
            } else {
                "12".to_owned()
            };
            let input_label = if mode == 0 {
                format!("输入{quantity_name} ({})", unit_options[unit_index])
            } else {
                format!("输入信号 ({signal_symbol})")
            };
            let input_hint = if mode == 0 {
                "需位于设定量程内"
            } else {
                "需位于输出范围内"
            };
            let input_key = if mode == 0 {
                format!("{profile}.process_input")
            } else {
                "signal_input".to_owned()
            };
            self.input(
                ui,
                tool,
                &input_key,
                &input_label,
                &input_default,
                input_hint,
            );

            let result = (|| {
                let low = self.number(tool, &low_key, low_default)?;
                let high = self.number(tool, &high_key, high_default)?;
                let signal_low = self.number(tool, "signal_low", "4")?;
                let signal_high = self.number(tool, "signal_high", "20")?;
                let input = self.number(tool, &input_key, &input_default)?;
                if quantity == 2 && unit_index == 0 && (low < 0.0 || high > 100.0) {
                    return Err("相对湿度量程应在 0～100 %RH 内".to_owned());
                }
                if quantity == 2 && unit_index == 2 && low < 0.0 {
                    return Err("绝对湿度量程不能为负数".to_owned());
                }
                if (quantity == 1 || (quantity == 2 && unit_index == 1))
                    && low
                        < if quantity == 1 {
                            match unit_index {
                                1 => -459.67,
                                2 => 0.0,
                                _ => -273.15,
                            }
                        } else {
                            -273.15
                        }
                {
                    return Err("温度量程不能低于绝对零度".to_owned());
                }
                let transmitter = Transmitter::new(
                    Range::new(low, high)?,
                    Range::new(signal_low, signal_high)?,
                    function == 1,
                );
                let (process_value, signal, fraction) = if mode == 0 {
                    let (signal, fraction) = transmitter.forward(input)?;
                    (input, signal, fraction)
                } else {
                    let (value, fraction) = transmitter.inverse(input)?;
                    (value, input, fraction)
                };

                Ok(vec![
                    (
                        "被测量值",
                        format!(
                            "{} {} {}",
                            quantity_name,
                            format_number(process_value),
                            unit_options[unit_index]
                        ),
                    ),
                    (
                        "输出信号",
                        format!("{} {signal_symbol}", format_number(signal)),
                    ),
                    ("量程比例", format!("{} %", format_number(fraction * 100.0))),
                ])
            })();
            show_result_or_error(ui, "换算结果", result);
        });
        info_note(ui, tool.kind);
    }
}
