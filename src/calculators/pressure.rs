//! 压力工具页面：标准器评定入口、高差修正和标准大气换算。
//! 表单中 kPa 与模型中的 Pa 在调用边界统一转换。
use crate::INK;
use crate::app::ToolboxApp;
use crate::calculators::common::*;
use crate::catalog::ToolDef;
use eframe::egui::{self, RichText};
mod composite;
pub(crate) mod model;
mod ranges;
mod selection;
pub(crate) mod selection_model;
use model::{Range, hydrostatic_difference, select_standard};

use crate::calculators::math::{isa_altitude, isa_pressure, isa_temperature};

impl ToolboxApp {
    /// 保留独立通道跨度比评定，供用户主动选择；默认使用压力与电测合成模型。
    pub(crate) fn render_transmitter_reference_legacy(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
    ) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(
                RichText::new("变送器通道参数")
                    .size(14.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(6.0);
            self.input(ui, tool, "dut_low", "被检压力下限", "0", "压力量程");
            self.input(ui, tool, "dut_high", "被检压力上限", "1", "压力量程");
            self.input(
                ui,
                tool,
                "dut_pressure_accuracy",
                "被检压力准确度 (%)",
                "0.1",
                "按满量程百分比",
            );
            self.input(
                ui,
                tool,
                "ref_pressure_accuracy",
                "标准压力准确度 (%)",
                "0.01",
                "按满量程百分比",
            );
            self.input(
                ui,
                tool,
                "ref_pressure_low",
                "标准压力量程下限",
                "0",
                "按标准器满量程计算允差",
            );
            self.input(
                ui,
                tool,
                "ref_pressure_high",
                "标准压力量程上限",
                "1.6",
                "标准器压力范围",
            );
            self.input(
                ui,
                tool,
                "dut_signal_low",
                "被检输出下限 (mA)",
                "4",
                "例如 4–20 mA",
            );
            self.input(
                ui,
                tool,
                "dut_signal_high",
                "被检输出上限 (mA)",
                "20",
                "例如 4–20 mA",
            );
            self.input(
                ui,
                tool,
                "dut_signal_accuracy",
                "被检电信号准确度 (%)",
                "0.1",
                "按信号跨度百分比",
            );
            self.input(
                ui,
                tool,
                "ref_signal_accuracy",
                "标准电测准确度 (%)",
                "0.02",
                "按信号跨度百分比",
            );
            self.input(
                ui,
                tool,
                "ref_signal_low",
                "标准电测量程下限 (mA)",
                "0",
                "按标准器测量跨度计算允差",
            );
            self.input(
                ui,
                tool,
                "ref_signal_high",
                "标准电测量程上限 (mA)",
                "24",
                "标准器电流范围",
            );
            self.input(
                ui,
                tool,
                "required_ratio",
                "量传比要求",
                "3",
                "按使用规程设置，如 3 或 4",
            );
            let result = (|| {
                let dut_pressure = Range::new(
                    self.number(tool, "dut_low", "0")?,
                    self.number(tool, "dut_high", "1")?,
                )?;
                let dut_signal = Range::new(
                    self.number(tool, "dut_signal_low", "4")?,
                    self.number(tool, "dut_signal_high", "20")?,
                )?;
                let ref_pressure = Range::new(
                    self.number(tool, "ref_pressure_low", "0")?,
                    self.number(tool, "ref_pressure_high", "1.6")?,
                )?;
                let ref_signal = Range::new(
                    self.number(tool, "ref_signal_low", "0")?,
                    self.number(tool, "ref_signal_high", "24")?,
                )?;
                let dpa = self.number(tool, "dut_pressure_accuracy", "0.1")?;
                let rpa = self.number(tool, "ref_pressure_accuracy", "0.01")?;
                let dsa = self.number(tool, "dut_signal_accuracy", "0.1")?;
                let rsa = self.number(tool, "ref_signal_accuracy", "0.02")?;
                let required = self.number(tool, "required_ratio", "3")?;
                let pressure = select_standard(dut_pressure, ref_pressure, dpa, rpa, required)?;
                let signal = select_standard(dut_signal, ref_signal, dsa, rsa, required)?;
                let dut_pressure_tolerance = pressure.dut_tolerance;
                let ref_pressure_tolerance = pressure.reference_tolerance;
                let pressure_ratio = pressure.ratio;
                let dut_signal_tolerance = signal.dut_tolerance;
                let ref_signal_tolerance = signal.reference_tolerance;
                let signal_ratio = signal.ratio;
                Ok(vec![
                    ("被检压力允差", format_number(dut_pressure_tolerance)),
                    ("标准压力允差", format_number(ref_pressure_tolerance)),
                    ("压力通道量传比", format_number(pressure_ratio)),
                    ("被检输出允差", format_number(dut_signal_tolerance)),
                    ("标准电测允差", format_number(ref_signal_tolerance)),
                    ("电信号通道量传比", format_number(signal_ratio)),
                    (
                        "压力量程覆盖",
                        if pressure.coverage {
                            "符合"
                        } else {
                            "不符合"
                        }
                        .to_owned(),
                    ),
                    (
                        "电信号量程覆盖",
                        if signal.coverage {
                            "符合"
                        } else {
                            "不符合"
                        }
                        .to_owned(),
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
                ])
            })();
            show_result_or_error(ui, "选型结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_height_correction(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(
                RichText::new("高差与传压介质")
                    .size(14.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(6.0);
            let medium = self.select_index(
                ui,
                tool,
                "medium",
                "介质类型",
                &["液体", "气体（输入密度）"],
                0,
            );
            self.input(
                ui,
                tool,
                "height",
                "高度差 ΔH (m)",
                "0.1",
                "被检表高于标准器为正",
            );
            let gas_state = if medium == 1 {
                self.select_index(
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
            self.input(
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
                let gauge = self.select_index(
                    ui,
                    tool,
                    "gas.pressure_type",
                    "压力类型",
                    &["表压", "绝压"],
                    0,
                ) == 0;
                self.input(
                    ui,
                    tool,
                    "gas.temperature",
                    "气体温度 (°C)",
                    "20",
                    "实际温度",
                );
                self.input(
                    ui,
                    tool,
                    "gas.pressure",
                    "标准器压力 (kPa)",
                    "100",
                    "与所选压力类型一致",
                );
                self.input(
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
            self.input(
                ui,
                tool,
                "gravity",
                "重力加速度 g (m/s²)",
                "9.80665",
                "当地重力加速度",
            );
            let result = (|| {
                let h = self.number(tool, "height", "0.1")?;
                let rho = self.number(tool, &density_key, density_default)?;
                let g = self.number(tool, "gravity", "9.80665")?;
                let dp = if gas_state == 1 {
                    model::gas_difference(
                        h,
                        rho,
                        g,
                        self.number(tool, "gas.temperature", "20")?,
                        self.number(tool, "gas.pressure", "100")? * 1000.0,
                        self.number(tool, "gas.ambient", "101.325")? * 1000.0,
                        gauge,
                    )?
                } else {
                    hydrostatic_difference(h, rho, g)?
                };
                Ok(vec![
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
                ])
            })();
            show_result_or_error(ui, "修正结果", result);
        });
        info_note(ui, tool.kind);
    }

    pub(crate) fn render_altitude_pressure(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.group(|ui| {
            ui.set_max_width(820.0);
            ui.label(RichText::new("气压与高度").size(14.0).strong().color(INK));
            ui.add_space(6.0);
            let mode = self.select_index(
                ui,
                tool,
                "mode",
                "计算方式",
                &["位势高度 → 标准气压", "标准气压 → 位势高度"],
                0,
            );
            let input_key = if mode == 0 { "altitude" } else { "pressure" };
            if mode == 0 {
                self.input(
                    ui,
                    tool,
                    input_key,
                    "位势高度 (m)",
                    "0",
                    "范围 −1000 至 86 000 m",
                );
            } else {
                self.input(
                    ui,
                    tool,
                    input_key,
                    "标准气压 (kPa)",
                    "101.325",
                    "范围对应 −1000 至 86 000 m",
                );
            }
            let result = (|| {
                let input =
                    self.number(tool, input_key, if mode == 0 { "0" } else { "101.325" })?;
                let (h, p) = if mode == 0 {
                    let p = isa_pressure(input)
                        .ok_or_else(|| "位势高度应在 −1000 至 86 000 m 之间".to_owned())?;
                    (input, p)
                } else {
                    let p = input * 1_000.0;
                    let h = isa_altitude(p)
                        .ok_or_else(|| "气压应为正值，并对应 −1000 至 86 000 m".to_owned())?;
                    (h, p)
                };
                Ok(vec![
                    ("气压高度", format!("{} m", format_number(h))),
                    ("标准气压", format!("{} kPa", format_number(p / 1_000.0))),
                    (
                        "空气密度",
                        format!(
                            "{} kg/m³",
                            format_number(p / (287.052_87 * isa_temperature(h)))
                        ),
                    ),
                ])
            })();
            show_result_or_error(ui, "换算结果", result);
        });
        info_note(ui, tool.kind);
    }
}
