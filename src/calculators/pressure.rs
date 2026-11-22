//! 压力工具页面：标准器评定入口、高差修正和标准大气换算；统一左输右显双栏布局。
//! 表单中 kPa 与模型中的 Pa 在调用边界统一转换。
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use crate::theme;
use eframe::egui::{self, RichText};
mod composite;
mod selection;
pub(crate) use toolbox_core::calculators::pressure::{model, ranges, selection_model};
pub(crate) mod process {
    use crate::calculators::common::calculation_process;
    use eframe::egui;
    pub(crate) use toolbox_core::calculators::pressure::process::*;

    pub(crate) fn show(ui: &mut egui::Ui, id: &str, lines: &[String]) {
        calculation_process(ui, id, lines);
    }
}
use model::{Range, hydrostatic_difference, select_standard};

use crate::calculators::math::{isa_altitude_from_display, isa_kinetic_temperature, isa_molecular_temperature};

impl ToolboxApp {
    /// 保留独立通道跨度比评定，供用户主动选择；默认使用压力与电测合成模型。
    pub(crate) fn render_transmitter_reference_legacy(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
    ) {
        let p = theme::palette(ui.ctx());
        let mut steps_out = None;
        split_workspace(
            ui,
            0.44,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("变送器通道参数")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(10.0);
                    app.input(ui, tool, "dut_low", "被检压力下限", "0", "压力量程");
                    app.input(ui, tool, "dut_high", "被检压力上限", "1", "压力量程");
                    app.input(
                        ui,
                        tool,
                        "dut_pressure_accuracy",
                        "被检压力准确度 (%)",
                        "0.1",
                        "按满量程百分比",
                    );
                    app.input(
                        ui,
                        tool,
                        "ref_pressure_accuracy",
                        "标准压力准确度 (%)",
                        "0.01",
                        "按满量程百分比",
                    );
                    app.input(
                        ui,
                        tool,
                        "ref_pressure_low",
                        "标准压力量程下限",
                        "0",
                        "按标准器满量程计算允差",
                    );
                    app.input(
                        ui,
                        tool,
                        "ref_pressure_high",
                        "标准压力量程上限",
                        "1.6",
                        "标准器压力范围",
                    );
                    app.input(
                        ui,
                        tool,
                        "dut_signal_low",
                        "被检输出下限 (mA)",
                        "4",
                        "例如 4–20 mA",
                    );
                    app.input(
                        ui,
                        tool,
                        "dut_signal_high",
                        "被检输出上限 (mA)",
                        "20",
                        "例如 4–20 mA",
                    );
                    app.input(
                        ui,
                        tool,
                        "dut_signal_accuracy",
                        "被检电信号准确度 (%)",
                        "0.1",
                        "按信号跨度百分比",
                    );
                    app.input(
                        ui,
                        tool,
                        "ref_signal_accuracy",
                        "标准电测准确度 (%)",
                        "0.02",
                        "按信号跨度百分比",
                    );
                    app.input(
                        ui,
                        tool,
                        "ref_signal_low",
                        "标准电测量程下限 (mA)",
                        "0",
                        "按标准器测量跨度计算允差",
                    );
                    app.input(
                        ui,
                        tool,
                        "ref_signal_high",
                        "标准电测量程上限 (mA)",
                        "24",
                        "标准器电流范围",
                    );
                    app.input(
                        ui,
                        tool,
                        "required_ratio",
                        "量传比要求",
                        "3",
                        "按使用规程设置，如 3 或 4",
                    );
                });
            },
            |ui, app, _| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let dut_pressure = Range::new(
                            app.number(tool, "dut_low", "0")?,
                            app.number(tool, "dut_high", "1")?,
                        )?;
                        let dut_signal = Range::new(
                            app.number(tool, "dut_signal_low", "4")?,
                            app.number(tool, "dut_signal_high", "20")?,
                        )?;
                        let ref_pressure = Range::new(
                            app.number(tool, "ref_pressure_low", "0")?,
                            app.number(tool, "ref_pressure_high", "1.6")?,
                        )?;
                        let ref_signal = Range::new(
                            app.number(tool, "ref_signal_low", "0")?,
                            app.number(tool, "ref_signal_high", "24")?,
                        )?;
                        let dpa = app.number(tool, "dut_pressure_accuracy", "0.1")?;
                        let rpa = app.number(tool, "ref_pressure_accuracy", "0.01")?;
                        let dsa = app.number(tool, "dut_signal_accuracy", "0.1")?;
                        let rsa = app.number(tool, "ref_signal_accuracy", "0.02")?;
                        let required = app.number(tool, "required_ratio", "3")?;
                        let pressure =
                            select_standard(dut_pressure, ref_pressure, dpa, rpa, required)?;
                        let signal = select_standard(dut_signal, ref_signal, dsa, rsa, required)?;
                        let mut steps =
                            vec!["压力通道（量程和允差均使用压力输入单位）：".to_owned()];
                        steps.extend(process::selection(
                            dut_pressure,
                            ref_pressure,
                            dpa,
                            rpa,
                            required,
                            false,
                            &pressure,
                            "压力输入单位",
                        ));
                        steps.push(String::new());
                        steps.push("电信号通道：".to_owned());
                        steps.extend(process::selection(
                            dut_signal, ref_signal, dsa, rsa, required, false, &signal, "mA",
                        ));
                        steps.push(format!(
                            "综合判断：{}。",
                            if pressure.meets && signal.meets {
                                "两个通道均满足"
                            } else {
                                "至少一个通道不满足"
                            }
                        ));
                        Ok::<_, String>((
                            vec![
                                ("被检压力允差", format_number(pressure.dut_tolerance)),
                                ("标准压力允差", format_number(pressure.reference_tolerance)),
                                ("压力通道量传比", format_number(pressure.ratio)),
                                ("被检输出允差", format_number(signal.dut_tolerance)),
                                ("标准电测允差", format_number(signal.reference_tolerance)),
                                ("电信号通道量传比", format_number(signal.ratio)),
                                (
                                    "压力量程覆盖",
                                    if pressure.coverage { "符合" } else { "不符合" }.to_owned(),
                                ),
                                (
                                    "电信号量程覆盖",
                                    if signal.coverage { "符合" } else { "不符合" }.to_owned(),
                                ),
                                ("规程要求", format!("≥ {} 倍", format_number(required))),
                                (
                                    "综合判断",
                                    if pressure.meets && signal.meets {
                                        "两通道量程覆盖且量传比满足"
                                    } else {
                                        "至少一个通道不满足"
                                    }
                                    .to_owned(),
                                ),
                            ],
                            steps,
                        ))
                    })();
                    match result {
                        Ok((metrics, steps)) => {
                            render_metrics(ui, "选型结果", &metrics);
                            steps_out = Some(steps);
                        }
                        Err(message) => render_error_message(ui, &message),
                    }
                });
            },
        );
        if let Some(steps) = &steps_out {
            process::show(ui, "two_channel_process", steps);
        }
        self.calculation_note(ui, tool);
    }

    pub(crate) fn render_height_correction(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        struct HeightParams {
            gas_state: usize,
            density_key: String,
            density_default: &'static str,
            gauge: bool,
        }

        let mut process_data = None;
        split_workspace(
            ui,
            0.44,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("高差与传压介质")
                            .size(14.5)
                            .strong()
                            .color(p.ink),
                    );
                    ui.add_space(10.0);
                    let medium = app.select_index(
                        ui,
                        tool,
                        "medium",
                        "介质类型",
                        &["液体", "气体（输入密度）"],
                        0,
                    );
                    app.input(
                        ui,
                        tool,
                        "height",
                        "高度差 ΔH (m)",
                        "0.1",
                        "被检表高于标准器为正",
                    );
                    let gas_state = if medium == 1 {
                        app.select_index(
                            ui,
                            tool,
                            "gas.state",
                            "气体密度条件",
                            &["输入当地状态密度", "由标准状态密度换算"],
                            1,
                        )
                    } else {
                        0
                    };
                    let density_key = if gas_state == 1 {
                        "density.gas.normal".into()
                    } else {
                        format!("density.{medium}")
                    };
                    let density_default = if medium == 0 {
                        "998.2"
                    } else if gas_state == 1 {
                        "1.293"
                    } else {
                        "1.204"
                    };
                    app.input(
                        ui,
                        tool,
                        &density_key,
                        "介质密度 (kg/m³)",
                        density_default,
                        if gas_state == 1 {
                            "0 °C、101.325 kPa 时的气体密度"
                        } else {
                            "填写测量状态下的局部密度"
                        },
                    );
                    let gauge = if gas_state == 1 {
                        let gauge = app.select_index(
                            ui,
                            tool,
                            "gas.pressure_type",
                            "压力类型",
                            &["表压", "绝压"],
                            0,
                        ) == 0;
                        app.input(
                            ui,
                            tool,
                            "gas.temperature",
                            "气体温度 (°C)",
                            "20",
                            "实际温度",
                        );
                        app.input(
                            ui,
                            tool,
                            "gas.pressure",
                            "标准器压力 (kPa)",
                            "100",
                            "与所选压力类型一致",
                        );
                        app.input(
                            ui,
                            tool,
                            "gas.ambient",
                            "环境大气压 (kPa)",
                            "101.325",
                            "环境绝对压力",
                        );
                        gauge
                    } else {
                        false
                    };
                    app.input(
                        ui,
                        tool,
                        "gravity",
                        "重力加速度 g (m/s²)",
                        "9.80665",
                        "当地重力加速度",
                    );
                    HeightParams {
                        gas_state,
                        density_key,
                        density_default,
                        gauge,
                    }
                })
                .inner
            },
            |ui, app, params| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let h = app.number(tool, "height", "0.1")?;
                        let rho = app.number(tool, &params.density_key, params.density_default)?;
                        let g = app.number(tool, "gravity", "9.80665")?;
                        let (dp, gas_process) = if params.gas_state == 1 {
                            let temperature = app.number(tool, "gas.temperature", "20")?;
                            let pressure = app.number(tool, "gas.pressure", "100")? * 1000.0;
                            let ambient = app.number(tool, "gas.ambient", "101.325")? * 1000.0;
                            let gas = model::gas_difference_details(
                                h,
                                rho,
                                g,
                                temperature,
                                pressure,
                                ambient,
                                params.gauge,
                            )?;
                            (gas.difference, Some((gas, temperature, pressure, ambient)))
                        } else {
                            (hydrostatic_difference(h, rho, g)?, None)
                        };
                        let metrics = vec![
                            (
                                "标准器压力 − 被检表压力",
                                format!("{} Pa", format_number(dp)),
                            ),
                            (
                                "标准值移至被检高度的修正量",
                                format!("{} kPa", format_number(-dp / 1_000.0)),
                            ),
                            (
                                "等效水柱（4 °C）",
                                format!("{} mmH₂O", format_number(dp / 9.806_38)),
                            ),
                            (
                                "修正方向",
                                if h > 0.0 {
                                    "被检表高于标准器"
                                } else if h < 0.0 {
                                    "被检表低于标准器"
                                } else {
                                    "无高差"
                                }
                                .to_owned(),
                            ),
                        ];
                        Ok::<_, String>((metrics, h, rho, g, dp, gas_process, params.gauge))
                    })();
                    match result {
                        Ok((metrics, h, rho, g, dp, gas_process, gauge)) => {
                            render_metrics(ui, "修正结果", &metrics);
                            process_data = Some((h, rho, g, dp, gas_process, gauge));
                        }
                        Err(message) => render_error_message(ui, &message),
                    }
                });
            },
        );

        if let Some((h, rho, g, dp, gas_process, gauge)) = process_data {
            ui.add_space(10.0);
            collapsible_note(ui, "height_process", "计算过程", |ui| {
                let mut display = ProcessDisplay::new(ui, "height_process");
                let mut line = |ui: &mut egui::Ui, text: String| {
                    display.line(ui, &text);
                };
                ui.label(RichText::new("1. 输入与约定").strong());
                line(
                    ui,
                    format!(
                        "高度差 ΔH = {} m；重力加速度 g = {} m/s²",
                        format_process_number(h),
                        format_process_number(g)
                    ),
                );
                ui.label("被检表高于标准器时 ΔH 为正；部分计算值经过显示修约，实际计算精度不变。");
                if let Some((gas, temperature, pressure, ambient)) = gas_process {
                    line(
                        ui,
                        format!(
                            "标准状态密度 ρ₀ = {} kg/m³（0 °C、101.325 kPa）",
                            format_process_number(rho)
                        ),
                    );
                    line(
                        ui,
                        format!(
                            "气体温度 = {} °C；标准器{} = {} Pa；环境绝对压力 = {} Pa",
                            format_process_number(temperature),
                            if gauge { "表压" } else { "绝压" },
                            format_process_number(pressure),
                            format_process_number(ambient)
                        ),
                    );
                    ui.label(RichText::new("2. 换算当地气体密度").strong());
                    if gauge {
                        line(
                            ui,
                            format!(
                                "标准器绝压 = 表压 + 环境绝压 = {} + {} = {} Pa",
                                format_process_number(pressure),
                                format_process_number(ambient),
                                format_process_computed(gas.absolute_pressure)
                            ),
                        );
                    } else {
                        line(
                            ui,
                            format!(
                                "标准器绝压 = {} Pa",
                                format_process_computed(gas.absolute_pressure)
                            ),
                        );
                    }
                    ui.label(
                        "ρ = ρ₀ × (标准器绝压 / 101325 Pa) × 273.15 K / (温度 + 273.15 K)",
                    );
                    line(
                        ui,
                        format!(
                            "ρ = {} × ({} / 101325) × 273.15 / ({} + 273.15) = {} kg/m³",
                            format_process_number(rho),
                            format_process_computed(gas.absolute_pressure),
                            format_process_number(temperature),
                            format_process_computed(gas.density)
                        ),
                    );
                    ui.label(RichText::new("3. 计算压力差").strong());
                    line(
                        ui,
                        format!(
                            "气柱压力差 = ρ × g × ΔH = {} × {} × {} = {} Pa",
                            format_process_computed(gas.density),
                            format_process_number(g),
                            format_process_number(h),
                            format_process_computed(gas.gas_column)
                        ),
                    );
                    if let Some(atmosphere) = gas.atmosphere {
                        line(
                            ui,
                            format!(
                                "标准大气模型：标准器高度 = {} m；被检高度处环境压力 = {} Pa",
                                format_process_computed(atmosphere.altitude),
                                format_process_computed(atmosphere.upper_pressure)
                            ),
                        );
                        ui.label("由环境绝压在 −1000～84852 m 的标准大气模型中二分反算标准器位势高度，再从海平面逐层正算被检高度处的压力；标准大气使用 g₀ = 9.80665 m/s²、R = 287.05287 J/(kg·K)。");
                        let _ = crate::calculators::math::isa_pressure_with_steps(
                            atmosphere.altitude + h,
                            |base, end, temperature, lapse, start, result| {
                                ui.add_space(ui.text_style_height(&egui::TextStyle::Body));
                                for detail in process::atmosphere_layer(
                                    base,
                                    end,
                                    temperature,
                                    lapse,
                                    start,
                                    result,
                                ) {
                                    line(ui, detail);
                                }
                            },
                        );
                        line(
                            ui,
                            format!(
                                "环境气柱压力差 = {} − {} = {} Pa",
                                format_process_number(ambient),
                                format_process_computed(atmosphere.upper_pressure),
                                format_process_computed(atmosphere.difference)
                            ),
                        );
                        line(
                            ui,
                            format!(
                                "表压差 = 气柱压力差 − 环境气柱压力差 = {} − {} = {} Pa",
                                format_process_computed(gas.gas_column),
                                format_process_computed(atmosphere.difference),
                                format_process_computed(dp)
                            ),
                        );
                    }
                    ui.label("采用局部气体密度近似，不沿高度积分气体自重。");
                } else {
                    line(
                        ui,
                        format!("当地介质密度 ρ = {} kg/m³", format_process_number(rho)),
                    );
                    ui.label(RichText::new("2. 计算压力差").strong());
                    line(
                        ui,
                        format!(
                            "标准器压力 − 被检表压力 = ρ × g × ΔH = {} × {} × {} = {} Pa",
                            format_process_number(rho),
                            format_process_number(g),
                            format_process_number(h),
                            format_process_computed(dp)
                        ),
                    );
                }
                ui.label(RichText::new("修正量与等效水柱").strong());
                line(
                    ui,
                    format!(
                        "标准值移至被检高度的修正量 = −压力差 ÷ 1000 = {} kPa",
                        format_process_computed(-dp / 1_000.0)
                    ),
                );
                line(
                    ui,
                    format!(
                        "等效水柱（4 °C）= 压力差 ÷ 9.80638 = {} mmH₂O",
                        format_process_computed(dp / 9.806_38)
                    ),
                );
            });
        }
        self.calculation_note(ui, tool);
    }

    pub(crate) fn render_altitude_pressure(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        let p = theme::palette(ui.ctx());
        let mut steps_out = None;

        split_workspace(
            ui,
            0.42,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("气压与高度").size(14.5).strong().color(p.ink));
                    ui.add_space(10.0);
                    let mode = app.select_segmented(
                        ui,
                        tool,
                        "mode",
                        "计算方式",
                        &["位势高度 → 气压", "气压 → 位势高度"],
                        0,
                    );
                    let input_key = if mode == 0 { "altitude" } else { "pressure" };
                    if mode == 0 {
                        app.input(
                            ui,
                            tool,
                            input_key,
                            "位势高度 (m)",
                            "0",
                            "范围 −1000 至 84 852 m",
                        );
                    } else {
                        app.input(
                            ui,
                            tool,
                            input_key,
                            "标准气压 (kPa)",
                            "101.325",
                            "范围对应 −1000 至 84 852 m",
                        );
                    }
                    (mode, input_key)
                })
                .inner
            },
            |ui, app, (mode, input_key)| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let result = (|| {
                        let input = app.number(
                            tool,
                            input_key,
                            if mode == 0 { "0" } else { "101.325" },
                        )?;
                        let (h, reverse_pressure) = if mode == 0 {
                            (input, None)
                        } else {
                            let (height, pressure) = isa_altitude_from_display(input, 1_000.0, format_number)
                                .ok_or_else(|| "气压应为正值，并对应 −1000 至 84 852 m".to_owned())?;
                            (height, Some(pressure))
                        };
                        let mut layers = Vec::new();
                        let model_pressure = crate::calculators::math::isa_pressure_with_steps(
                            h,
                            |base, end, temperature, lapse, start_pressure, end_pressure| {
                                layers.push(String::new());
                                layers.extend(process::atmosphere_layer(
                                    base,
                                    end,
                                    temperature,
                                    lapse,
                                    start_pressure,
                                    end_pressure,
                                ));
                            },
                        )
                        .ok_or_else(|| "位势高度应在 −1000 至 84 852 m 之间".to_owned())?;
                        let pressure_pa = reverse_pressure.unwrap_or(model_pressure);
                        let temperature = isa_molecular_temperature(h);
                        let density = pressure_pa / (287.052_87 * temperature);
                        let mut steps = vec![
                            if mode == 0 {
                                format!(
                                    "输入位势高度 = {} m；从 0 m、101325 Pa 开始逐层计算。",
                                    format_process_number(input)
                                )
                            } else if pressure_pa != input * 1_000.0 {
                                format!(
                                    "输入标准气压 = {} kPa，与边界显示值一致，使用完整端点压力 {} Pa，得到位势高度 {} m。",
                                    format_process_number(input),
                                    format_process_computed(pressure_pa),
                                    format_process_computed(h)
                                )
                            } else {
                                format!(
                                    "输入标准气压 = {} kPa = {} Pa；在 −1000～84852 m 的同一分层模型中二分反算，得到位势高度 {} m。",
                                    format_process_number(input),
                                    format_process_computed(pressure_pa),
                                    format_process_computed(h)
                                )
                            },
                            "标准大气常数：g₀ = 9.80665 m/s²，R = 287.05287 J/(kg·K)。".to_owned(),
                        ];
                        steps.extend(layers);
                        if mode != 0 {
                            steps.push(format!("反算校验：该高度对应模型气压 {} Pa（与输入值的末位可能因数值精度略有差异）。", format_process_computed(model_pressure)));
                        }
                        steps.push(format!("动力学温度 T = Tₘ × (M/M₀) = {} K，分子尺度温度 Tₘ = {} K；空气密度 ρ = p / (R × Tₘ) = {} / (287.05287 × {}) = {} kg/m³。", format_process_computed(isa_kinetic_temperature(h)), format_process_computed(temperature), format_process_computed(pressure_pa), format_process_computed(temperature), format_process_computed(density)));
                        steps.push(
                            "使用标准大气模型；位势高度不等同于实际几何海拔，不含当地气象修正。"
                                .to_owned(),
                        );
                        Ok::<_, String>((
                            vec![
                                ("气压高度", format!("{} m", format_number(h))),
                                (
                                    "标准气压",
                                    format!("{} kPa", format_number(pressure_pa / 1_000.0)),
                                ),
                                ("空气密度", format!("{} kg/m³", format_number(density))),
                            ],
                            steps,
                        ))
                    })();
                    match result {
                        Ok((metrics, steps)) => {
                            render_metrics(ui, "换算结果", &metrics);
                            steps_out = Some(steps);
                        }
                        Err(message) => render_error_message(ui, &message),
                    }
                });
            },
        );
        if let Some(steps) = &steps_out {
            process::show(ui, "altitude_process", steps);
        }
        self.calculation_note(ui, tool);
    }
}
