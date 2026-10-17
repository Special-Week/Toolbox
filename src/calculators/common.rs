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

/// 输入阶段拒绝 NaN 和无穷大，避免非有限值进入计算或坐标映射。
pub(crate) fn parse_number(text: &str) -> Result<f64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("请填写数值".to_owned());
    }
    let value = text
        .parse::<f64>()
        .map_err(|_| "请输入有效数字".to_owned())?;
    if !value.is_finite() {
        return Err("数值必须为有限数字".to_owned());
    }
    Ok(value)
}

/// 通用结果使用普通小数；极小值按有效数字展开，不因固定小数位被显示成零。
/// 该函数只控制显示精度，文件导出直接写入未经过显示修约的 f64。
pub(crate) fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.abs() >= 1.0e10 || value.abs() < 1.0e-4 {
        // Retain the previous eleven significant digits. Display expands the
        // rounded value as a plain decimal, including very small coefficients.
        let rounded = format!("{value:.10e}").parse::<f64>().unwrap_or(value);
        // Rounding a value near f64::MAX can overflow; keep that finite value.
        return if rounded.is_finite() { rounded } else { value }.to_string();
    }
    format!("{value:.8}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

pub(crate) fn format_unit(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value == 0.0 {
        return "0.0000000".to_owned();
    }
    if value.abs() >= 1.0e9 || (value != 0.0 && value.abs() < 1.0e-4) {
        return format_number(value);
    }
    format!("{value:.7}")
}

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
    ui.add_space(10.0);
    egui::Frame::new()
        .fill(BLUE_SOFT)
        .corner_radius(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(RichText::new("计算说明").size(12.5).strong().color(BLUE));
            ui.add(
                egui::Label::new(
                    RichText::new(super::explanations::for_tool(kind))
                        .size(12.0)
                        .color(INK),
                )
                .wrap(),
            );
        });
}
