//! 压力、温度和湿度变送器页面；统一左输右显双栏布局，量程、方向及温标分别保存独立输入状态。
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use crate::theme;
use eframe::egui::{self, RichText};
mod model;
use crate::calculators::pressure::model::Range;
use model::Transmitter;

impl ToolboxApp {
    pub(crate) fn render_transmitter(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        let mut process = None;

        struct TxParams {
            quantity: usize,
            unit_index: usize,
            unit_symbol: String,
            quantity_name: &'static str,
            low_key: String,
            high_key: String,
            low_default: &'static str,
            high_default: &'static str,
            mode: usize,
            function: usize,
            signal_symbol: &'static str,
            input_key: String,
            input_default: String,
        }

        split_workspace(
            ui,
            0.44,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("变送器量程换算")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(10.0);

                    let quantity = app.select_segmented(
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
                    let unit_index =
                        app.select_index(ui, tool, &unit_key, "量程单位", unit_options, 0);
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
                    let mode = app.select_segmented(
                        ui,
                        tool,
                        "direction",
                        "计算方向",
                        &["被测量 → 信号", "信号 → 被测量"],
                        0,
                    );
                    let function = app.select_segmented(
                        ui,
                        tool,
                        "function",
                        "传递特性",
                        &["线性", "开方"],
                        0,
                    );
                    let signal_unit = app.select_segmented(
                        ui,
                        tool,
                        "signal_unit",
                        "输出信号单位",
                        &["mA", "V", "mV"],
                        0,
                    );
                    let signal_symbol = ["mA", "V", "mV"][signal_unit];
                    let unit_symbol = unit_options[unit_index].to_owned();
                    app.input(
                        ui,
                        tool,
                        &low_key,
                        &format!("{quantity_name}下限 ({unit_symbol})"),
                        low_default,
                        "变送器量程起点",
                    );
                    app.input(
                        ui,
                        tool,
                        &high_key,
                        &format!("{quantity_name}上限 ({unit_symbol})"),
                        high_default,
                        "变送器量程终点",
                    );
                    app.input(
                        ui,
                        tool,
                        "signal_low",
                        &format!("输出信号下限 ({signal_symbol})"),
                        "4",
                        "例如 4 mA",
                    );
                    app.input(
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
                        format!("输入{quantity_name} ({unit_symbol})")
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
                    app.input(
                        ui,
                        tool,
                        &input_key,
                        &input_label,
                        &input_default,
                        input_hint,
                    );

                    TxParams {
                        quantity,
                        unit_index,
                        unit_symbol,
                        quantity_name,
                        low_key,
                        high_key,
                        low_default,
                        high_default,
                        mode,
                        function,
                        signal_symbol,
                        input_key,
                        input_default,
                    }
                })
                .inner
            },
            |ui, app, params| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let low = app.number(tool, &params.low_key, params.low_default)?;
                        let high = app.number(tool, &params.high_key, params.high_default)?;
                        let signal_low = app.number(tool, "signal_low", "4")?;
                        let signal_high = app.number(tool, "signal_high", "20")?;
                        let input = app.number(tool, &params.input_key, &params.input_default)?;
                        if params.quantity == 2
                            && params.unit_index == 0
                            && (low < 0.0 || high > 100.0)
                        {
                            return Err("相对湿度量程应在 0～100 %RH 内".to_owned());
                        }
                        if params.quantity == 2 && params.unit_index == 2 && low < 0.0 {
                            return Err("绝对湿度量程不能为负数".to_owned());
                        }
                        if (params.quantity == 1
                            || (params.quantity == 2 && params.unit_index == 1))
                            && low
                                < if params.quantity == 1 {
                                    match params.unit_index {
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
                            params.function == 1,
                        );
                        let (process_value, signal, fraction) = if params.mode == 0 {
                            let (signal, fraction) = transmitter.forward(input)?;
                            (input, signal, fraction)
                        } else {
                            let (value, fraction) = transmitter.inverse(input)?;
                            (value, input, fraction)
                        };

                        let n = format_process_number;
                        let c = format_process_computed;
                        let process_span = high - low;
                        let signal_span = signal_high - signal_low;
                        let signal_symbol = params.signal_symbol;
                        let unit_symbol = &params.unit_symbol;
                        let mut steps = vec![
                            format!(
                                "被测量范围 {}～{} {}，跨度 = {} − {} = {} {}。",
                                n(low),
                                n(high),
                                unit_symbol,
                                n(high),
                                n(low),
                                c(process_span),
                                unit_symbol
                            ),
                            format!(
                                "信号范围 {}～{} {signal_symbol}，跨度 = {} − {} = {} {signal_symbol}。",
                                n(signal_low),
                                n(signal_high),
                                n(signal_high),
                                n(signal_low),
                                c(signal_span)
                            ),
                            String::new(),
                        ];
                        if params.mode == 0 {
                            steps.push(format!(
                                "被测量比例 r = (输入 − 下限) / 跨度 = ({} − {}) / {} = {}。",
                                n(input),
                                n(low),
                                c(process_span),
                                c(fraction)
                            ));
                            let output_fraction = if params.function == 1 {
                                fraction.sqrt()
                            } else {
                                fraction
                            };
                            steps.push(if params.function == 1 {
                                format!(
                                    "开方特性：信号比例 q = √r = √{} = {}。",
                                    c(fraction),
                                    c(output_fraction)
                                )
                            } else {
                                format!("线性特性：信号比例 q = r = {}。", c(output_fraction))
                            });
                            steps.push(format!("输出信号 = 下限 + q × 信号跨度 = {} + {} × {} = {} {signal_symbol}；程序使用融合乘加。", n(signal_low), c(output_fraction), c(signal_span), c(signal)));
                        } else {
                            let signal_fraction = (signal - signal_low) / signal_span;
                            steps.push(format!(
                                "信号比例 q = (输入 − 下限) / 跨度 = ({} − {}) / {} = {}。",
                                n(input),
                                n(signal_low),
                                c(signal_span),
                                c(signal_fraction)
                            ));
                            steps.push(if params.function == 1 {
                                format!(
                                    "开方特性的反算：被测量比例 r = q² = {}² = {}。",
                                    c(signal_fraction),
                                    c(fraction)
                                )
                            } else {
                                format!("线性特性：被测量比例 r = q = {}。", c(fraction))
                            });
                            steps.push(format!(
                                "被测量 = 下限 + r × 被测量跨度 = {} + {} × {} = {} {}；程序使用融合乘加。",
                                n(low),
                                c(fraction),
                                c(process_span),
                                c(process_value),
                                unit_symbol
                            ));
                        }
                        steps.push(format!(
                            "量程比例 = r × 100 = {} × 100 = {}%。",
                            c(fraction),
                            c(fraction * 100.0)
                        ));
                        process = Some(steps);

                        Ok(vec![
                            (
                                "被测量值",
                                format!(
                                    "{} {} {}",
                                    params.quantity_name,
                                    format_number(process_value),
                                    unit_symbol
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
            },
        );

        if let Some(lines) = &process {
            calculation_process(ui, "transmitter_process", lines);
        }
        info_note(ui, tool.kind);
    }
}
