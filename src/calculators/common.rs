//! 表单、结果卡片、分段选择器、双栏工作台、错误提示及普通小数格式化的共用实现。
use crate::catalog::ToolKind;
use crate::theme;
use eframe::egui::{self, Align2, CornerRadius, FontId, RichText, Sense, Stroke, TextEdit, Vec2};
use egui_phosphor::regular as icons;

pub(crate) type Results = Result<Vec<(&'static str, String)>, String>;

pub(crate) fn panel<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let p = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(16)
        .show(ui, contents)
}

/// 宽屏自适应双栏工作台（左输右显），窄屏自动回退为上下单栏。
/// 严格限制左右两栏的 max_width，防止宽表或长控件向右撑出视口导致裁切。
pub(crate) fn split_workspace<S, L, R>(
    ui: &mut egui::Ui,
    left_ratio: f32,
    state: &mut S,
    left: impl FnOnce(&mut egui::Ui, &mut S) -> L,
    right: impl FnOnce(&mut egui::Ui, &mut S, L) -> R,
) -> R {
    let width = ui.available_width();
    if width >= 900.0 {
        let gap = 16.0;
        let left_width = ((width - gap) * left_ratio).clamp(360.0, 560.0);
        let right_width = (width - gap - left_width).max(320.0);
        let mut right_out = None;
        ui.horizontal_top(|ui| {
            ui.set_max_width(width);
            ui.spacing_mut().item_spacing.x = gap;
            let left_out = ui
                .allocate_ui_with_layout(
                    Vec2::new(left_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(left_width);
                        ui.set_max_width(left_width);
                        left(ui, state)
                    },
                )
                .inner;
            right_out = Some(
                ui.allocate_ui_with_layout(
                    Vec2::new(right_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(right_width);
                        ui.set_max_width(right_width);
                        right(ui, state, left_out)
                    },
                )
                .inner,
            );
        });
        right_out.unwrap()
    } else {
        ui.set_max_width(width);
        let left_out = left(ui, state);
        ui.add_space(12.0);
        right(ui, state, left_out)
    }
}

/// 现代胶囊分段选择器：固定单行精确均分槽宽，缩放窗口零闪烁、绝不折行。
pub(crate) fn segmented_control(
    ui: &mut egui::Ui,
    options: &[&str],
    selected: usize,
) -> usize {
    let p = theme::palette(ui.ctx());
    let mut next = selected.min(options.len().saturating_sub(1));
    let total_w = ui.available_width().max(80.0);
    let (outer_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, 36.0), Sense::hover());

    let painter = ui.painter();
    painter.rect_filled(outer_rect, CornerRadius::same(9), p.surface_subtle);
    painter.rect_stroke(
        outer_rect,
        CornerRadius::same(9),
        Stroke::new(1.0_f32, p.border),
        egui::StrokeKind::Inside,
    );

    let count = options.len().max(1);
    let inner = outer_rect.shrink(3.0);
    let gap = 3.0_f32;
    let seg_w = ((inner.width() - gap * (count - 1) as f32) / count as f32).max(1.0);
    let seg_h = inner.height().max(1.0);

    for (index, label) in options.iter().enumerate() {
        let left = inner.left() + index as f32 * (seg_w + gap);
        let seg_rect = egui::Rect::from_min_size(egui::pos2(left, inner.top()), Vec2::new(seg_w, seg_h));
        let seg_id = ui.id().with(("segmented_opt", index));
        let response = ui
            .interact(seg_rect, seg_id, Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let is_active = index == next;
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                is_active,
                *label,
            )
        });
        if response.clicked() {
            next = index;
        }

        let is_active = index == next;
        if is_active {
            painter.rect_filled(seg_rect, CornerRadius::same(7), p.surface);
            painter.rect_stroke(
                seg_rect,
                CornerRadius::same(7),
                Stroke::new(1.0_f32, p.accent_border),
                egui::StrokeKind::Inside,
            );
        } else if response.hovered() {
            painter.rect_filled(seg_rect, CornerRadius::same(7), p.surface_hover);
        }

        let text_color = if is_active { p.accent } else { p.ink_secondary };
        let galley = egui::WidgetText::from(
            RichText::new(*label)
                .size(12.5)
                .strong()
                .color(text_color),
        )
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (seg_rect.width() - 8.0).max(1.0),
            egui::TextStyle::Body,
        );
        let text_pos = egui::pos2(
            seg_rect.center().x - galley.size().x * 0.5,
            seg_rect.center().y - galley.size().y * 0.5,
        );
        painter.galley(text_pos, galley, text_color);
    }
    next
}

/// 保持已有卡片的列位置，新增卡片放入当前较短的一列。
pub(crate) fn balanced_columns(
    ui: &mut egui::Ui,
    id: &str,
    count: usize,
    mut card: impl FnMut(&mut egui::Ui, usize),
) {
    let layout_id = ui.make_persistent_id(id);
    let mut assignments = ui
        .data_mut(|data| data.get_temp::<Vec<usize>>(layout_id))
        .unwrap_or_default();
    assignments.truncate(count);
    if count > 1 && ui.available_width() >= 760.0 {
        ui.columns(2, |columns| {
            for i in 0..count {
                let column = if let Some(&column) = assignments.get(i) {
                    column
                } else {
                    let column = usize::from(
                        columns[1].next_widget_position().y < columns[0].next_widget_position().y,
                    );
                    assignments.push(column);
                    column
                };
                card(&mut columns[column], i);
                columns[column].add_space(8.0);
            }
        });
    } else {
        for i in 0..count {
            card(ui, i);
            ui.add_space(8.0);
        }
    }
    ui.data_mut(|data| data.insert_temp(layout_id, assignments));
}

pub(crate) fn form_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let p = theme::palette(ui.ctx());
    if ui.available_width() < 340.0 {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(RichText::new(label).size(12.0).color(p.muted));
            contents(ui)
        })
        .inner
    } else {
        ui.horizontal(|ui| {
            let label_width = if ui.available_width() < 460.0 {
                132.0
            } else if ui.available_width() < 680.0 {
                156.0
            } else {
                186.0
            };
            ui.add_sized(
                [label_width, 36.0],
                egui::Label::new(RichText::new(label).size(12.5).color(p.muted)).wrap(),
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
    let p = theme::palette(ui.ctx());
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
        egui::Shape::rect_filled(rect, CornerRadius::same(8), p.input_bg),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(
            if response.has_focus() { 1.5_f32 } else { 1.0_f32 },
            if response.has_focus() {
                p.accent
            } else if response.hovered() {
                p.accent_border
            } else {
                p.border_strong
            },
        ),
        egui::StrokeKind::Inside,
    );
    response
}

pub(crate) fn input_row(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) -> bool {
    form_row(ui, label, |ui| {
        text_input(ui, value, ui.available_width(), hint).changed()
    })
}

pub(crate) use super::format::{format_number, format_unit, parse_number};
pub(crate) use toolbox_core::calculators::common::{
    format_process_computed, format_process_number, format_process_rounded,
    temperature_conversion_note,
};

/// 对明显的浮点尾数适当减少显示位数，不改变实际计算。
fn format_process_result(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    let original = value.to_string();
    let digits: String = original
        .chars()
        .filter(char::is_ascii_digit)
        .skip_while(|c| *c == '0')
        .collect();
    let tail = original.contains('.')
        && digits.len() >= 13
        && (digits.contains("0000000") || digits.contains("9999999"));
    let rounded = if tail {
        format!("{value:.11e}")
    } else {
        format!("{value:.12e}")
    }
    .parse::<f64>()
    .unwrap_or(value);
    if rounded.is_finite() && rounded != 0.0 {
        rounded
    } else {
        value
    }
    .to_string()
}

/// 相近数相减时，避免显示修约掩盖对结果有意义的差值。
fn safe_substitution(
    expression: &str,
    results: &std::collections::HashMap<String, String>,
) -> bool {
    sensitive_numbers(expression, results).is_empty()
}

fn sensitive_numbers(
    expression: &str,
    results: &std::collections::HashMap<String, String>,
) -> std::collections::HashSet<String> {
    static DIFFERENCE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let difference = DIFFERENCE.get_or_init(|| regex::Regex::new(
        r"(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*−\s*(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)"
    ).unwrap());
    let mut preserve = std::collections::HashSet::new();
    for captures in difference.captures_iter(expression) {
        let left = &captures[1];
        let right = &captures[2];
        let (Ok(a), Ok(b)) = (left.parse::<f64>(), right.parse::<f64>()) else {
            continue;
        };
        let shown_a = results
            .get(left)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(a);
        let shown_b = results
            .get(right)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(b);
        let actual = a - b;
        let shown = shown_a - shown_b;
        if !actual.is_finite()
            || !shown.is_finite()
            || (shown - actual).abs() > actual.abs() * 1e-12
        {
            preserve.insert(left.to_owned());
            preserve.insert(right.to_owned());
        }
    }
    preserve
}

/// 整理公式结果，并在后续代入该结果时使用相同的显示值。
fn compact_process_line(
    line: &str,
    results: &mut std::collections::HashMap<String, String>,
) -> String {
    use std::sync::OnceLock;
    static NUMBER: OnceLock<regex::Regex> = OnceLock::new();
    static NUMBERS: OnceLock<regex::Regex> = OnceLock::new();
    let number = NUMBER
        .get_or_init(|| regex::Regex::new(r"^-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?").unwrap());
    let numbers = NUMBERS
        .get_or_init(|| regex::Regex::new(r"-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?").unwrap());
    let mut out = String::with_capacity(line.len());
    let mut offset = 0;
    let mut context_start = 0;
    while let Some(relative) = line[offset..].find(" = ") {
        let equals = offset + relative;
        let after_equals = equals + 3;
        let context = &line[context_start..equals];
        let arithmetic = [" + ", " − ", " × ", " / ", "√", "Σ", "exp(", "ln(", "^"]
            .iter()
            .any(|part| context.contains(part));
        let piece = &line[offset..equals];
        if arithmetic && !piece.contains('；') && safe_substitution(context, results) {
            out.push_str(&numbers.replace_all(piece, |caps: &regex::Captures<'_>| {
                results
                    .get(&caps[0])
                    .cloned()
                    .unwrap_or_else(|| caps[0].to_owned())
            }));
        } else {
            out.push_str(piece);
        }
        out.push_str(" = ");
        let rest = &line[after_equals..];
        if let Some(matched) = number.find(rest) {
            let token = matched.as_str();
            let suffix = &rest[token.len()..];
            let continues = [" + ", " − ", " × ", " / ", "⁴", "³", "²", "^"]
                .iter()
                .any(|part| suffix.starts_with(part));
            if continues {
                let expression = rest
                    .split_once(" = ")
                    .map_or(rest, |(expression, _)| expression);
                out.push_str(if safe_substitution(expression, results) {
                    results.get(token).map(String::as_str).unwrap_or(token)
                } else {
                    token
                });
            } else if arithmetic && !continues {
                if let Ok(value) = token.parse::<f64>() {
                    let formatted = format_process_result(value);
                    if formatted != token {
                        results.insert(token.to_owned(), formatted.clone());
                    }
                    out.push_str(&formatted);
                } else {
                    out.push_str(token);
                }
            } else {
                out.push_str(token);
            }
            offset = after_equals + token.len();
        } else {
            offset = after_equals;
        }
        context_start = after_equals;
    }
    out.push_str(&line[offset..]);
    out
}

pub(crate) fn show_result_or_error(ui: &mut egui::Ui, title: &str, result: Results) {
    match result {
        Ok(metrics) => render_metrics(ui, title, &metrics),
        Err(message) => render_error_message(ui, &message),
    }
}

pub(crate) fn render_metrics(ui: &mut egui::Ui, title: &str, metrics: &[(&str, String)]) {
    let p = theme::palette(ui.ctx());
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(14.0).strong().color(p.ink));
        ui.label(
            RichText::new("点击卡片可复制数值")
                .size(11.0)
                .color(p.muted),
        );
    });
    ui.add_space(8.0);
    let width = ui.available_width();
    let columns = if width > 800.0 {
        3
    } else if width > 460.0 {
        2
    } else {
        1
    };
    let mut copied: Option<String> = None;
    for row in metrics.chunks(columns) {
        ui.columns(columns, |cols| {
            for (index, (label, value)) in row.iter().enumerate() {
                let is_primary = index == 0 && std::ptr::eq(row.as_ptr(), metrics.as_ptr());
                let card_ui = &mut cols[index];
                let card_w = card_ui.available_width();
                let resp = egui::Frame::new()
                    .fill(if is_primary {
                        p.accent_soft
                    } else {
                        p.surface_subtle
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_primary { p.accent_border } else { p.border },
                    ))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 12,
                        top: 10,
                        bottom: 10,
                    })
                    .show(card_ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.set_min_width((card_w - 26.0).max(60.0));
                        ui.set_min_height(56.0);
                        ui.label(RichText::new(*label).size(11.5).color(p.muted));
                        ui.add(
                            egui::Label::new(
                                RichText::new(value.as_str())
                                    .font(FontId::monospace(if is_primary { 16.5 } else { 15.0 }))
                                    .strong()
                                    .color(if is_primary { p.accent } else { p.ink }),
                            )
                            .wrap(),
                        );
                    });
                let rect = resp.response.rect;
                // 左侧强调竖条
                if is_primary {
                    card_ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.left() + 1.0, rect.top() + 8.0),
                            Vec2::new(3.0, (rect.height() - 16.0).max(8.0)),
                        ),
                        CornerRadius::same(2),
                        p.accent,
                    );
                }
                let click = card_ui
                    .interact(
                        rect,
                        card_ui.id().with(("metric_copy", *label, index)),
                        Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("点击复制该结果");
                if click.hovered() {
                    card_ui.painter().rect_stroke(
                        rect,
                        CornerRadius::same(10),
                        Stroke::new(1.2_f32, p.accent),
                        egui::StrokeKind::Inside,
                    );
                    card_ui.painter().text(
                        egui::pos2(rect.right() - 12.0, rect.top() + 14.0),
                        Align2::CENTER_CENTER,
                        icons::COPY,
                        FontId::proportional(13.0),
                        p.accent,
                    );
                }
                if click.clicked() {
                    copied = Some(value.clone());
                }
            }
        });
        ui.add_space(6.0);
    }
    if let Some(val) = copied {
        ui.ctx().copy_text(val.clone());
        theme::toast_success(ui.ctx(), format!("已复制：{val}"));
    }
}

pub(crate) fn render_error_message(ui: &mut egui::Ui, message: &str) {
    let p = theme::palette(ui.ctx());
    ui.add_space(10.0);
    egui::Frame::new()
        .fill(p.danger_soft)
        .stroke(Stroke::new(1.0_f32, p.danger_border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(format!("⚠ {message}")).color(p.danger)).wrap());
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
    let p = theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.accent_soft)
        .stroke(Stroke::new(1.0_f32, p.accent_border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            egui::CollapsingHeader::new(RichText::new(title).size(14.0).strong().color(p.accent))
                .id_salt(id)
                .default_open(false)
                .show(ui, contents);
        });
}

/// 仅在当前计算成功时调用，过程数据与结果由同一次计算产生。
pub(crate) fn calculation_process(ui: &mut egui::Ui, id: &str, lines: &[String]) {
    calculation_process_lazy(ui, id, || lines);
}

/// 仅在展开时构建较长的逐点过程，避免页面重绘重复反算。
pub(crate) fn calculation_process_lazy<T: AsRef<[String]>>(
    ui: &mut egui::Ui,
    id: &str,
    make_lines: impl FnOnce() -> T,
) {
    ui.add_space(10.0);
    collapsible_note(ui, id, "计算过程", |ui| {
        let lines = make_lines();
        let mut display = ProcessDisplay::new(ui, id);
        for line in lines.as_ref() {
            if line.is_empty() {
                ui.add_space(ui.text_style_height(&egui::TextStyle::Body));
            } else {
                display.line(ui, line);
            }
        }
        ui.label("部分计算结果和后续代入值经过显示修约，实际计算精度不变。");
    });
}

pub(crate) struct ProcessDisplay {
    results: std::collections::HashMap<String, String>,
}

impl ProcessDisplay {
    pub(crate) fn new(_ui: &mut egui::Ui, _id: &str) -> Self {
        Self {
            results: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn line(&mut self, ui: &mut egui::Ui, line: &str) {
        let displayed = process_display_line(line, false, &mut self.results);
        super::formula_layout::show(ui, &displayed);
    }
}

fn process_display_line(
    line: &str,
    full_precision: bool,
    results: &mut std::collections::HashMap<String, String>,
) -> String {
    let raw = line.replace('\u{2063}', "");
    if full_precision {
        return super::math_notation::parenthesize_negative_subtrahends(&raw).into_owned();
    }
    let mut candidate = results.clone();
    for (i, part) in line.split('\u{2063}').enumerate() {
        if i % 2 == 1
            && let Ok(value) = part.parse::<f64>()
        {
            candidate.insert(part.to_owned(), format_process_result(value));
        }
    }
    // 只保留有相消风险的操作数；同一行的其他结果仍按显示规则整理。
    let preserve = sensitive_numbers(&raw, &candidate);
    if !preserve.is_empty() {
        let mut displayed = String::new();
        for (index, part) in line.split('\u{2063}').enumerate() {
            if index % 2 == 0 || preserve.contains(part) {
                displayed.push_str(part);
            } else if let Ok(value) = part.parse::<f64>() {
                let formatted = format_process_result(value);
                results.insert(part.to_owned(), formatted.clone());
                displayed.push_str(&formatted);
            } else {
                displayed.push_str(part);
            }
        }
        return super::math_notation::parenthesize_negative_subtrahends(&displayed).into_owned();
    }
    let prepared = {
        let mut parts = line.split('\u{2063}');
        let mut prepared = parts.next().unwrap_or_default().to_owned();
        while let Some(value) = parts.next() {
            let Some(next) = parts.next() else {
                prepared.push_str(value);
                break;
            };
            let formatted = value
                .parse::<f64>()
                .map(format_process_result)
                .unwrap_or_else(|_| value.to_owned());
            if formatted != value {
                results.insert(value.to_owned(), formatted.clone());
            }
            prepared.push_str(&formatted);
            prepared.push_str(next);
        }
        prepared
    };
    let displayed = compact_process_line(&prepared, results);
    super::math_notation::parenthesize_negative_subtrahends(&displayed).into_owned()
}

#[cfg(test)]
pub(crate) fn process_lines_for_test(lines: &[String], full_precision: bool) -> Vec<String> {
    let mut results = std::collections::HashMap::new();
    lines
        .iter()
        .map(|line| process_display_line(line, full_precision, &mut results))
        .collect()
}

/// 计算说明与公式一起展开，切换模型时保留展开状态。
pub(crate) fn info_note_mode(ui: &mut egui::Ui, kind: ToolKind, mode: usize) {
    let p = theme::palette(ui.ctx());
    let (summary, formula) = super::explanations::selected(kind, mode);
    if summary.is_empty() && formula.is_empty() {
        return;
    }
    ui.add_space(10.0);
    collapsible_note(ui, "calculation_note", "计算说明", |ui| {
        if !summary.is_empty() {
            note_paragraph(ui, summary);
        }
        if !formula.is_empty() {
            ui.label(
                RichText::new("详细公式与取值")
                    .size(12.0)
                    .strong()
                    .color(p.ink),
            );
            note_paragraph(ui, &formula);
            if matches!(kind, ToolKind::Uncertainty) {
                note_paragraph(
                    ui,
                    "分量填写有效数字后先按所选方式修约，再乘灵敏系数参与合成；留空保留原始计算精度。四舍六入按五成双处理。",
                );
                if mode == 1 {
                    note_paragraph(
                        ui,
                        "先用原始 uc 计算 U=k×uc，再分别对 uc 和 U 按所选有效数字修约。",
                    );
                }
            }
        }
    });
}

fn note_paragraph(ui: &mut egui::Ui, text: &str) {
    let p = theme::palette(ui.ctx());
    if text
        .lines()
        .any(super::formula_layout::has_formattable_math)
    {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for line in text.lines() {
                super::formula_layout::show(ui, line);
            }
        });
    } else {
        ui.add(egui::Label::new(RichText::new(text).size(12.0).color(p.ink)).wrap());
    }
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
    fn process_numbers_preserve_coefficients_needed_for_recalculation() {
        let r0 = 10000.0_f64;
        let a0 = 1.0 / 298.15 - r0.ln() / 3950.0;
        let a1 = 1.0 / 3950.0;
        let displayed_a0 = format_process_number(a0).parse::<f64>().unwrap();
        let displayed_a1 = format_process_number(a1).parse::<f64>().unwrap();
        assert_eq!(displayed_a0, a0);
        assert_eq!(displayed_a1, a1);
        let recalculated = 1.0 / (displayed_a0 + displayed_a1 * r0.ln()) - 273.15;
        assert!((recalculated - 25.0).abs() < 1e-10);
        for value in [f64::MIN_POSITIVE, f64::from_bits(1), f64::MAX, -0.0, 0.0001] {
            assert_eq!(
                format_process_number(value)
                    .parse::<f64>()
                    .unwrap()
                    .to_bits(),
                value.to_bits()
            );
        }
    }

    #[test]
    fn process_results_hide_float_tails_without_losing_small_values() {
        let mut results = std::collections::HashMap::new();
        assert_eq!(format_process_result(10.03 - 10.01), "0.02");
        assert_eq!(format_process_result(100.02 - 100.0), "0.02");
        assert_eq!(format_process_result(0.1 + 0.2), "0.3");
        assert!(format_process_result(1e-30).parse::<f64>().unwrap() > 0.0);
        assert_eq!(
            format_process_result(f64::from_bits(1))
                .parse::<f64>()
                .unwrap(),
            f64::from_bits(1)
        );
        let largest = format_process_result(f64::MAX).parse::<f64>().unwrap();
        assert!(largest.is_finite() && (largest / f64::MAX - 1.0).abs() < 1e-12);
        assert_eq!(
            format_process_number(1.2345678901234567),
            "1.2345678901234567"
        );
        assert_eq!(
            compact_process_line("10.03 − 10.01 = 0.019999999999999574。", &mut results),
            "10.03 − 10.01 = 0.02。"
        );
        assert_eq!(
            compact_process_line(
                "绝对误差 = 100.02 − 100 = 0.01999999999999602。",
                &mut results
            ),
            "绝对误差 = 100.02 − 100 = 0.02。"
        );
        assert_eq!(
            compact_process_line("c[0] = 1.2345678901234567", &mut results),
            "c[0] = 1.2345678901234567"
        );
        assert_eq!(
            compact_process_line(
                "结果 = 1.2345678901234567 × 2 = 2.4691357802469134。",
                &mut results
            ),
            "结果 = 1.2345678901234567 × 2 = 2.469135780247。"
        );
        assert_eq!(
            compact_process_line("平均值 x̄ = Σxᵢ/n = 10.017999999999999。", &mut results),
            "平均值 x̄ = Σxᵢ/n = 10.018。"
        );
        assert_eq!(
            compact_process_line(
                "测量点 1：xᵢ = 10.02；xᵢ − x̄ = 10.02 − 10.017999999999999 = 0.002000000000001。",
                &mut results
            ),
            "测量点 1：xᵢ = 10.02；xᵢ − x̄ = 10.02 − 10.018 = 0.002。"
        );
        assert_eq!(
            compact_process_line(
                "测量点 2：xᵢ = 10.01；xᵢ − x̄ = 10.01 − 10.017999999999999 = -0.007999999999999。",
                &mut results
            ),
            "测量点 2：xᵢ = 10.01；xᵢ − x̄ = 10.01 − 10.018 = -0.008。"
        );
        assert_eq!(
            compact_process_line("样本标准差 s = √Σx² = 0.008366600265340578。", &mut results),
            "样本标准差 s = √Σx² = 0.008366600265341。"
        );
        assert_eq!(
            compact_process_line(
                "u = s/√n = 0.008366600265340578 / √1 = 0.008366600265340578。",
                &mut results
            ),
            "u = s/√n = 0.008366600265341 / √1 = 0.008366600265341。"
        );
        assert_eq!(
            compact_process_line("平均值 = Σx/n = 1.000000000000001。", &mut results),
            "平均值 = Σx/n = 1。"
        );
        let precise = compact_process_line(
            "测量点：x = 1.000000000000002；差值 = 1.000000000000002 − 1.000000000000001 = 0.0000000000000011102230246251565。",
            &mut results,
        );
        assert!(
            precise.contains("1.000000000000002 − 1.000000000000001"),
            "{precise}"
        );
        assert_eq!(
            compact_process_line(
                "输入 x = 1.000000000000001；系数 c[0] = 1.2345678901234567。",
                &mut results
            ),
            "输入 x = 1.000000000000001；系数 c[0] = 1.2345678901234567。"
        );
    }

    #[test]
    fn standalone_results_and_precision_toggle_preserve_important_values() {
        let mut results = std::collections::HashMap::new();
        let value = 100.02_f64 - 100.0;
        let line = format!("误差绝对值 |Δ| = {}。", format_process_computed(value));
        assert_eq!(
            process_display_line(&line, false, &mut results),
            "误差绝对值 |Δ| = 0.02。"
        );
        assert_eq!(
            process_display_line(&line, true, &mut results),
            format!("误差绝对值 |Δ| = {}。", value)
        );

        let precise = 1.000000000000001_f64;
        let input = format!(
            "输入 = {}；系数 = {}。",
            format_process_number(precise),
            format_process_number(precise)
        );
        assert_eq!(process_display_line(&input, false, &mut results), input);
        let rounded = format_process_rounded(precise, Some(17));
        assert_eq!(rounded, format_process_number(precise));

        let a = 1.000000000000002_f64;
        let b = 1.000000000000001_f64;
        let cancellation = format!(
            "差值 = {} − {} = {}。",
            format_process_computed(a),
            format_process_computed(b),
            format_process_computed(a - b)
        );
        let shown = process_display_line(&cancellation, false, &mut results);
        assert!(
            shown.contains("1.000000000000002 − 1.000000000000001"),
            "{shown}"
        );
    }

    #[test]
    fn parenthesized_subtractions_preserve_significant_differences() {
        let a = -1.0_f64;
        let b = -0.9999999999999948_f64;
        let line = format!(
            "残差 e = (Y − Ŷ) × s = ({} − {}) × 1.5 = {}。",
            format_process_computed(a),
            format_process_computed(b),
            format_process_computed((a - b) * 1.5)
        );
        let mut results = std::collections::HashMap::new();
        let shown = process_display_line(&line, false, &mut results);
        assert!(shown.contains("-0.9999999999999948"), "{shown}");
        assert!(!shown.contains("(-1 − -1)"));
        assert_eq!(
            process_display_line(&line, true, &mut results),
            super::super::math_notation::parenthesize_negative_subtrahends(
                &line.replace('\u{2063}', "")
            )
        );
        let line = format!(
            "绝对误差 = {} − {} = {}。",
            format_process_number(-3.0),
            format_process_number(-5.0),
            format_process_computed(2.0)
        );
        assert_eq!(
            process_display_line(&line, false, &mut results),
            "绝对误差 = -3 − (-5) = 2。"
        );
    }

    #[test]
    fn sensitive_subtraction_keeps_its_operands_without_restoring_unrelated_float_tails() {
        let temperature = 34.99374148609655_f64;
        let line = format!(
            "温差 = {}−35 = {}；误差 = 100.02 − 100 = {}。",
            format_process_computed(temperature),
            format_process_computed(temperature - 35.0),
            format_process_computed(100.02_f64 - 100.0)
        );
        let shown = process_display_line(&line, false, &mut std::collections::HashMap::new());
        assert!(shown.contains("34.99374148609655−35"), "{shown}");
        assert!(shown.ends_with("误差 = 100.02 − 100 = 0.02。"), "{shown}");
    }

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
            if output.shapes.iter().any(|shape| {
                matches!(
                    &shape.shape, egui::Shape::Text(text) if text.galley.text() == expected
                )
            }) {
                return true;
            }
            let fragments: Vec<_> = expected
                .lines()
                .flat_map(super::super::formula_layout::visible_text_fragments)
                .filter(|part| !part.is_empty())
                .collect();
            !fragments.is_empty() && fragments.iter().all(|fragment| output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains(fragment))
            }))
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
