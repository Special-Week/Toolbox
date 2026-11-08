//! 表单、结果卡片、错误提示及普通小数格式化的共用实现。
use crate::catalog::ToolKind;
use crate::{BLUE, BLUE_SOFT, INK, MUTED, RED};
use eframe::egui::{self, Color32, RichText, TextEdit};

pub(crate) type Results = Result<Vec<(&'static str, String)>, String>;

pub(crate) fn panel<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(220, 229, 241)))
        .corner_radius(8)
        .inner_margin(16)
        .show(ui, contents)
}

pub(crate) fn form_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if ui.available_width() < 720.0 {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).size(12.0).color(MUTED));
            contents(ui)
        })
        .inner
    } else {
        ui.horizontal(|ui| {
            ui.add_sized(
                [186.0, 36.0],
                egui::Label::new(RichText::new(label).size(12.0).color(MUTED)),
            );
            contents(ui)
        })
        .inner
    }
}

pub(crate) fn text_input(
    ui: &mut egui::Ui,
    value: &mut String,
    width: f32,
    hint: &str,
) -> egui::Response {
    let background = ui.painter().add(egui::Shape::Noop);
    let response = ui.add_sized(
        [width, 36.0],
        TextEdit::singleline(value)
            .hint_text(hint)
            .vertical_align(egui::Align::Center)
            .margin(egui::Margin::symmetric(10, 8))
            .frame(false),
    );
    // TextEdit 的 response 矩形不含内边距，边框需要还原完整控件区域。
    let rect = response.rect.expand2(egui::vec2(10.0, 8.0));
    ui.painter().set(
        background,
        egui::Shape::rect_filled(rect, 6, Color32::from_rgb(250, 252, 255)),
    );
    ui.painter().rect_stroke(
        rect,
        6,
        egui::Stroke::new(
            1.0_f32,
            if response.has_focus() {
                BLUE
            } else {
                Color32::from_rgb(150, 168, 192)
            },
        ),
        egui::StrokeKind::Inside,
    );
    response
}

pub(crate) fn input_row(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    form_row(ui, label, |ui| {
        text_input(ui, value, ui.available_width(), hint);
    });
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
                egui::Frame::new()
                    .fill(Color32::from_rgb(247, 250, 255))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(220, 229, 241)))
                    .corner_radius(8)
                    .inner_margin(10)
                    .show(&mut cols[index], |ui| {
                        ui.set_min_width(ui.available_width());
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
    egui::Frame::new()
        .fill(Color32::from_rgb(255, 244, 243))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(242, 205, 202)))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(format!("⚠ {message}")).color(RED)).wrap());
        });
}

pub(crate) fn info_note(ui: &mut egui::Ui, kind: ToolKind) {
    info_note_mode(ui, kind, 0);
}

pub(crate) fn collapsible_note(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    title: &str,
    contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(BLUE_SOFT)
        .corner_radius(8)
        .inner_margin(18)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            egui::CollapsingHeader::new(RichText::new(title).size(14.0).strong().color(BLUE))
                .id_salt(id)
                .default_open(false)
                .show(ui, contents);
        });
}

/// 计算说明与公式一起展开，切换模型时保留展开状态。
pub(crate) fn info_note_mode(ui: &mut egui::Ui, kind: ToolKind, mode: usize) {
    let (summary, formula) = super::explanations::selected(kind, mode);
    if summary.is_empty() && formula.is_empty() {
        return;
    }
    ui.add_space(10.0);
    collapsible_note(ui, "calculation_note", "计算说明", |ui| {
        if !summary.is_empty() {
            ui.add(egui::Label::new(RichText::new(summary).size(12.0).color(INK)).wrap());
        }
        if !formula.is_empty() {
            ui.label(
                RichText::new("详细公式与取值")
                    .size(12.0)
                    .strong()
                    .color(INK),
            );
            ui.add(egui::Label::new(RichText::new(formula).size(12.0).color(INK)).wrap());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_text_and_hint_are_centered_vertically() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let mut value = "100.02".to_owned();
        let mut empty = String::new();
        let mut centers = [0.0; 2];
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                centers[0] = text_input(ui, &mut value, 200.0, "").rect.center().y;
                centers[1] = text_input(ui, &mut empty, 200.0, "请输入").rect.center().y;
            });
        });
        for (index, expected) in ["100.02", "请输入"].iter().enumerate() {
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == *expected => Some(text),
                    _ => None,
                })
                .expect("应绘制输入文字或提示文字");
            let text_center = text.pos.y + text.galley.size().y / 2.0;
            assert!(
                (text_center - centers[index]).abs() < 1.0,
                "{expected} 未上下居中"
            );
        }
    }

    #[test]
    fn calculation_note_is_collapsed_and_tracks_model_when_expanded() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let kind = ToolKind::BlackbodyError;
        let render = |mode, events, time| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        [800.0, 800.0].into(),
                    )),
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| info_note_mode(ui, kind, mode));
                },
            )
        };
        let contains = |output: &egui::FullOutput, expected: &str| {
            output.shapes.iter().any(|shape| {
                matches!(
                    &shape.shape, egui::Shape::Text(text) if text.galley.text() == expected
                )
            })
        };
        let (summary, formula) = super::super::explanations::selected(kind, 0);
        let initial = render(0, vec![], 0.0);
        assert!(!contains(&initial, summary));
        assert!(!contains(&initial, &formula));
        let header = initial
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "计算说明" => {
                    Some(text.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .expect("应显示计算说明标题");
        for (pressed, time) in [(true, 0.1), (false, 0.2)] {
            render(
                0,
                vec![
                    egui::Event::PointerMoved(header),
                    egui::Event::PointerButton {
                        pos: header,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
                time,
            );
        }
        let expanded = render(0, vec![], 0.5);
        assert!(contains(&expanded, summary));
        assert!(contains(&expanded, &formula));
        let (next_summary, next_formula) = super::super::explanations::selected(kind, 1);
        let switched = render(1, vec![], 0.6);
        assert!(contains(&switched, next_summary));
        assert!(contains(&switched, &next_formula));
        assert!(!contains(&switched, summary));
    }
}
