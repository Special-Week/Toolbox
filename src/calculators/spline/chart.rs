//! 样条绘图与悬停取值；依据标签实际宽度调整留白和刻度数量，曲线裁剪到绘图区。
use super::{Point, UniformCubicSpline};
use crate::calculators::common::format_number;
use crate::{BLUE, INK, MUTED, RED};
use eframe::egui::{self, Align2, Color32, FontId, Rect, Stroke, Vec2};

use crate::calculators::axis::Axis;

pub(super) fn draw_chart(
    ui: &mut egui::Ui,
    spline: &UniformCubicSpline,
    points: &[Point],
    curve: &[Point],
    minimum: f64,
    maximum: f64,
) -> Rect {
    let width = ui.available_width().max(180.0);
    let height = (width * 0.36).clamp(300.0, 400.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let font = FontId::proportional(12.0);
    let visible_y = curve.iter().map(|point| point.y).chain(
        points
            .iter()
            .filter(|point| (minimum..=maximum).contains(&point.x))
            .map(|point| point.y),
    );
    let (y_min, y_max) = visible_y.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), y| {
        (low.min(y), high.max(y))
    });
    let y_axis = Axis::new("y", y_min, y_max, 6, true);
    let measure = |label: &str| {
        painter
            .layout_no_wrap(label.to_owned(), font.clone(), MUTED)
            .size()
            .x
    };
    let y_label_width = y_axis
        .ticks
        .iter()
        .map(|tick| measure(&tick.label))
        .fold(0.0, f32::max);
    let left_margin = y_label_width + 25.0;
    let available_plot_width = (width - left_margin - 30.0).max(1.0);
    let intervals = (available_plot_width / 90.0).floor().clamp(2.0, 10.0) as usize;
    let mut x_axis = Axis::new("x", minimum, maximum, intervals, false);
    let mut x_label_width = x_axis
        .ticks
        .iter()
        .map(|tick| measure(&tick.label))
        .fold(0.0, f32::max);
    let fitted_intervals = (available_plot_width / (x_label_width + 28.0))
        .floor()
        .max(2.0) as usize;
    if fitted_intervals < intervals {
        x_axis = Axis::new("x", minimum, maximum, fitted_intervals, false);
        x_label_width = x_axis
            .ticks
            .iter()
            .map(|tick| measure(&tick.label))
            .fold(0.0, f32::max);
    }
    let plot = Rect::from_min_max(
        egui::pos2(
            rect.left() + left_margin.max(x_label_width / 2.0 + 12.0),
            rect.top() + 44.0,
        ),
        egui::pos2(
            rect.right() - (x_label_width / 2.0 + 12.0).max(24.0),
            rect.bottom() - 54.0,
        ),
    );
    painter.rect_filled(rect, 8, Color32::WHITE);
    painter.rect_stroke(
        rect,
        8,
        Stroke::new(1.0_f32, Color32::from_rgb(223, 230, 239)),
        egui::StrokeKind::Inside,
    );

    for tick in &y_axis.ticks {
        let y = plot.bottom() - y_axis.tick_fraction(tick.value) as f32 * plot.height();
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            Stroke::new(1.0_f32, Color32::from_rgb(232, 237, 244)),
        );
        painter.line_segment(
            [egui::pos2(plot.left() - 4.0, y), egui::pos2(plot.left(), y)],
            Stroke::new(1.0_f32, MUTED),
        );
        painter.text(
            egui::pos2(plot.left() - 10.0, y),
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
        painter.line_segment(
            [
                egui::pos2(x, plot.bottom()),
                egui::pos2(x, plot.bottom() + 4.0),
            ],
            Stroke::new(1.0_f32, MUTED),
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
        Stroke::new(1.2_f32, Color32::from_rgb(143, 158, 178)),
    );
    painter.line_segment(
        [plot.left_bottom(), plot.right_bottom()],
        Stroke::new(1.2_f32, Color32::from_rgb(143, 158, 178)),
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
    let legend_x = plot.right() - 175.0;
    let legend_y = rect.top() + 20.0;
    painter.line_segment(
        [
            egui::pos2(legend_x, legend_y),
            egui::pos2(legend_x + 20.0, legend_y),
        ],
        Stroke::new(2.2_f32, BLUE),
    );
    painter.text(
        egui::pos2(legend_x + 27.0, legend_y),
        Align2::LEFT_CENTER,
        "插值曲线",
        FontId::proportional(11.5),
        INK,
    );
    painter.circle_filled(egui::pos2(plot.right() - 43.0, legend_y), 3.5, RED);
    painter.text(
        egui::pos2(plot.right() - 33.0, legend_y),
        Align2::LEFT_CENTER,
        "Q 点",
        FontId::proportional(11.5),
        INK,
    );

    let to_screen = |point: Point| {
        egui::pos2(
            plot.left() + x_axis.fraction(point.x) as f32 * plot.width(),
            plot.bottom() - y_axis.fraction(point.y) as f32 * plot.height(),
        )
    };
    let curve_painter = painter.with_clip_rect(plot);
    curve_painter.add(egui::Shape::line(
        curve.iter().copied().map(to_screen).collect(),
        Stroke::new(2.2_f32, BLUE),
    ));
    for point in points
        .iter()
        .filter(|point| (minimum..=maximum).contains(&point.x))
    {
        let position = to_screen(*point);
        curve_painter.circle_filled(position, 5.0, Color32::WHITE);
        curve_painter.circle_filled(position, 3.5, RED);
    }
    if let Some(position) = response.hover_pos()
        && plot.contains(position)
    {
        let fraction = ((position.x - plot.left()) / plot.width()) as f64;
        let x = fraction.mul_add(maximum - minimum, minimum);
        if let Ok(y) = spline.y_at_x(x) {
            curve_painter.line_segment(
                [
                    egui::pos2(position.x, plot.top()),
                    egui::pos2(position.x, plot.bottom()),
                ],
                Stroke::new(1.0_f32, Color32::from_rgb(185, 200, 221)),
            );
            curve_painter.circle_filled(to_screen(Point { x, y }), 4.0, BLUE);
            response.on_hover_text(format!(
                "x = {}\ny = {}",
                format_number(x),
                format_number(y)
            ));
        }
    }
    rect
}
