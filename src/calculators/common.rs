//! 表单、结果卡片、错误提示及普通小数格式化的共用实现。
use crate::catalog::ToolKind;
use crate::{BLUE, BLUE_SOFT, INK, MUTED, RED};
use eframe::egui::{self, RichText, TextEdit};

pub(crate) type Results = Result<Vec<(&'static str, String)>, String>;

pub(crate) fn input_row(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    if ui.available_width() < 720.0 {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).size(12.0).color(MUTED));
            ui.add_sized(
                [ui.available_width(), 36.0],
                TextEdit::singleline(value).hint_text(hint),
            );
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_sized(
                [186.0, 36.0],
                egui::Label::new(RichText::new(label).color(MUTED)),
            );
            ui.add_sized(
                [ui.available_width(), 36.0],
                TextEdit::singleline(value).hint_text(hint),
            );
        });
    }
}

pub(crate) use super::format::{format_number, format_unit, parse_number};

pub(crate) fn show_result_or_error(ui: &mut egui::Ui, title: &str, result: Results) {
    match result {
        Ok(metrics) => render_metrics(ui, title, &metrics),
        Err(message) => render_error_message(ui, &message),
    }
}

pub(crate) fn render_metrics(ui: &mut egui::Ui, title: &str, metrics: &[(&str, String)]) {
    ui.add_space(14.0);
    ui.label(RichText::new(title).size(14.0).strong().color(INK));
    ui.add_space(7.0);
    let width = ui.available_width();
    let columns = if width > 830.0 {
        3
    } else if width > 510.0 {
        2
    } else {
        1
    };
    for row in metrics.chunks(columns) {
        ui.columns(columns, |cols| {
            for (index, (label, value)) in row.iter().enumerate() {
                cols[index].group(|ui| {
                    ui.set_min_height(66.0);
                    ui.label(RichText::new(*label).size(11.5).color(MUTED));
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(value.as_str())
                            .size(15.5)
                            .strong()
                            .color(BLUE),
                    );
                });
            }
        });
        ui.add_space(4.0);
    }
}

pub(crate) fn render_error_message(ui: &mut egui::Ui, message: &str) {
    ui.add_space(11.0);
    ui.colored_label(RED, format!("⚠ {message}"));
}

pub(crate) fn info_note(ui: &mut egui::Ui, kind: ToolKind) {
    info_note_mode(ui, kind, 0);
}

/// 默认显示专业使用条件，较长公式按当前模型折叠查看。
pub(crate) fn info_note_mode(ui: &mut egui::Ui, kind: ToolKind, mode: usize) {
    let (summary, formula) = super::explanations::selected(kind, mode);
    if summary.is_empty() && formula.is_empty() {
        return;
    }
    ui.add_space(10.0);
    egui::Frame::new()
        .fill(BLUE_SOFT)
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(RichText::new("计算说明").size(12.5).strong().color(BLUE));
            ui.add(egui::Label::new(RichText::new(summary).size(12.0).color(INK)).wrap());
            if !formula.is_empty() {
                egui::CollapsingHeader::new("详细公式与取值")
                    .id_salt((summary, mode, "formula"))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(formula).size(12.0).color(INK)).wrap(),
                        );
                    });
            }
        });
}

impl crate::app::ToolboxApp {
    /// 读取现有选择以匹配说明，不修改模型参数。
    pub(crate) fn calculation_note(&mut self, ui: &mut egui::Ui, tool: &crate::catalog::ToolDef) {
        let key = match tool.kind {
            ToolKind::StandardThermocouple => "stc.calibration",
            ToolKind::StandardPlatinumResistor => "sprt.model",
            ToolKind::Thermistor => "ntc.model",
            ToolKind::PressureReference => "selection.reference_type",
            ToolKind::TransmitterReference => "selection.model",
            ToolKind::EarBlackbody => "ear.model",
            ToolKind::BlackbodyError | ToolKind::RadiationCorrection => "model",
            ToolKind::ResultantError => "components.mode",
            ToolKind::Uncertainty => "budget.mode",
            _ => "",
        };
        let mut mode = if key.is_empty() {
            0
        } else {
            self.value(tool, key, "0").parse().unwrap_or(0)
        };
        if matches!(tool.kind, ToolKind::StandardPlatinumResistor)
            && mode == 0
            && self.value(tool, "sprt.deviation", "0") == "1"
        {
            mode = 2;
        }
        info_note_mode(ui, tool.kind, mode);
    }
}
