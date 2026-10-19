//! 拟合曲线与残差图；复用工程刻度轴，图表只悬停取值，不截获滚轮。
use super::model::CurveFit;
use crate::calculators::{axis::Axis, common::format_number};
use crate::{BLUE, INK, MUTED, RED};
use eframe::egui::{self, Align2, Color32, FontId, Rect, Stroke, Vec2};

pub(super) fn draw(
    ui: &mut egui::Ui,
    fit: &CurveFit,
    points: &[(f64, f64)],
    x_label: &str,
    y_label: &str,
    residual: bool,
) -> Result<(), String> {
    let samples: Vec<_> = if residual {
        Vec::new()
    } else {
        (0..=180)
            .map(|index| {
                let fraction = index as f64 / 180.0;
                // 加权端点插值避免最大值减最小值先溢出。
                let x = (1.0 - fraction).mul_add(fit.minimum_x, fraction * fit.maximum_x);
                fit.predict(x).map(|y| (x, y))
            })
            .collect::<Result<_, _>>()?
    };
    let plotted: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(index, &(x, y))| (x, if residual { fit.residuals[index] } else { y }))
        .collect();
    let (mut minimum_y, mut maximum_y) = plotted
        .iter()
        .chain(&samples)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), point| {
            (low.min(point.1), high.max(point.1))
        });
    if residual {
        minimum_y = minimum_y.min(0.0);
        maximum_y = maximum_y.max(0.0);
    }
    let width = ui.available_width().max(180.0);
    let height = if residual {
        270.0
    } else {
        (width * 0.39).clamp(310.0, 390.0)
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let font = FontId::proportional(12.0);
    let y_axis = Axis::new(y_label, minimum_y, maximum_y, 5, true);
    let measure = |label: &str| {
        painter
            .layout_no_wrap(label.to_owned(), font.clone(), MUTED)
            .size()
            .x
    };
    let left = y_axis
        .ticks
        .iter()
        .map(|tick| measure(&tick.label))
        .fold(0.0, f32::max)
        + 23.0;
    let plot_width = (width - left - 32.0).max(1.0);
    let mut intervals = (plot_width / 88.0).floor().clamp(2.0, 9.0) as usize;
    let mut x_axis = Axis::new(x_label, fit.minimum_x, fit.maximum_x, intervals, false);
    let mut tick_width = x_axis
        .ticks
        .iter()
        .map(|tick| measure(&tick.label))
        .fold(0.0, f32::max);
    let adjusted = (plot_width / (tick_width + 24.0)).floor().max(2.0) as usize;
    if adjusted < intervals {
        intervals = adjusted;
        x_axis = Axis::new(x_label, fit.minimum_x, fit.maximum_x, intervals, false);
        tick_width = x_axis
            .ticks
            .iter()
            .map(|tick| measure(&tick.label))
            .fold(0.0, f32::max);
    }
    let plot = Rect::from_min_max(
        rect.min + egui::vec2(left.max(tick_width / 2.0 + 12.0), 43.0),
        rect.max - egui::vec2((tick_width / 2.0 + 12.0).max(24.0), 54.0),
    );
    painter.rect_filled(rect, 7.0, Color32::WHITE);
    painter.rect_stroke(
        rect,
        7.0,
        Stroke::new(1.0_f32, Color32::from_rgb(223, 230, 239)),
        egui::StrokeKind::Inside,
    );
    for tick in &y_axis.ticks {
        let y = plot.bottom() - y_axis.tick_fraction(tick.value) as f32 * plot.height();
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            Stroke::new(1.0_f32, Color32::from_rgb(232, 237, 244)),
        );
        painter.text(
            egui::pos2(plot.left() - 9.0, y),
            Align2::RIGHT_CENTER,
            &tick.label,
            font.clone(),
            MUTED,
        );
    }
    for tick in &x_axis.ticks {
        let x = plot.left() + x_axis.tick_fraction(tick.value) as f32 * plot.width();
        painter.line_segment(
            [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
            Stroke::new(1.0_f32, Color32::from_rgb(239, 243, 248)),
        );
        painter.text(
            egui::pos2(x, plot.bottom() + 10.0),
            Align2::CENTER_TOP,
            &tick.label,
            font.clone(),
            MUTED,
        );
    }
    painter.line_segment(
        [plot.left_top(), plot.left_bottom()],
        Stroke::new(1.2_f32, MUTED),
    );
    painter.line_segment(
        [plot.left_bottom(), plot.right_bottom()],
        Stroke::new(1.2_f32, MUTED),
    );
    painter.text(
        egui::pos2(plot.left(), rect.top() + 19.0),
        Align2::LEFT_CENTER,
        &y_axis.caption,
        font.clone(),
        INK,
    );
    painter.text(
        egui::pos2(plot.center().x, rect.bottom() - 14.0),
        Align2::CENTER_CENTER,
        &x_axis.caption,
        font.clone(),
        INK,
    );
    let to_screen = |point: (f64, f64)| {
        egui::pos2(
            plot.left() + x_axis.fraction(point.0) as f32 * plot.width(),
            plot.bottom() - y_axis.fraction(point.1) as f32 * plot.height(),
        )
    };
    let clipped = painter.with_clip_rect(plot);
    if residual {
        let zero = to_screen((fit.minimum_x, 0.0)).y;
        clipped.line_segment(
            [
                egui::pos2(plot.left(), zero),
                egui::pos2(plot.right(), zero),
            ],
            Stroke::new(1.4_f32, BLUE),
        );
    } else {
        clipped.add(egui::Shape::line(
            samples.iter().copied().map(to_screen).collect(),
            Stroke::new(2.2_f32, BLUE),
        ));
    }
    for &point in &plotted {
        clipped.circle_filled(to_screen(point), 5.0, Color32::WHITE);
        clipped.circle_filled(to_screen(point), 3.6, RED);
    }
    if let Some(position) = response
        .hover_pos()
        .filter(|position| plot.contains(*position))
    {
        let closest = plotted
            .iter()
            .enumerate()
            .map(|(index, &point)| (index, to_screen(point).distance(position)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, _)) = closest.filter(|(_, distance)| *distance < 12.0) {
            response.on_hover_text(format!(
                "第 {} 组\nx = {}\ny = {}\n拟合 y = {}\n残差 = {}",
                index + 1,
                format_number(points[index].0),
                format_number(points[index].1),
                format_number(fit.fitted[index]),
                format_number(fit.residuals[index]),
            ));
        } else if !residual {
            let fraction = ((position.x - plot.left()) / plot.width()) as f64;
            let input = (1.0 - fraction).mul_add(fit.minimum_x, fraction * fit.maximum_x);
            if let Ok(value) = fit.predict(input) {
                clipped.line_segment(
                    [
                        egui::pos2(position.x, plot.top()),
                        egui::pos2(position.x, plot.bottom()),
                    ],
                    Stroke::new(1.0_f32, Color32::from_rgb(185, 200, 221)),
                );
                clipped.circle_filled(to_screen((input, value)), 4.0, BLUE);
                response.on_hover_text(format!(
                    "x = {}\n拟合 y = {}",
                    format_number(input),
                    format_number(value)
                ));
            }
        }
    }
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(
            BLUE,
            if residual {
                "— 零残差参考线"
            } else {
                "— 拟合曲线"
            },
        );
        ui.colored_label(RED, "● 校准数据点");
    });
    Ok(())
}
