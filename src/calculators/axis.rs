//! 曲线坐标轴：选择整齐步长，并通过基准偏移和数量级缩放控制标签宽度。
use super::common::format_number;
use crate::{BLUE, INK};
use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Stroke};

pub(crate) fn draw_curve_cursor(
    painter: &Painter,
    plot: Rect,
    point: Pos2,
    labels: (&str, &str),
    value: (f64, f64),
    details: Option<&str>,
) {
    let painter = painter.with_clip_rect(plot);
    painter.line_segment(
        [
            egui::pos2(point.x, plot.top()),
            egui::pos2(point.x, plot.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(185, 200, 221)),
    );
    painter.circle_filled(point, 4.0, BLUE);

    let mut text = format!(
        "{} = {}\n{} = {}",
        labels.0,
        format_number(value.0),
        labels.1,
        format_number(value.1)
    );
    if let Some(details) = details {
        text.push_str(details);
    }
    let galley = painter.layout(
        text,
        FontId::proportional(12.0),
        INK,
        (plot.width() - 24.0).max(1.0),
    );
    let size = galley.size() + egui::vec2(20.0, 14.0);
    let x = if point.x + 12.0 + size.x <= plot.right() {
        point.x + 12.0
    } else {
        point.x - 12.0 - size.x
    };
    let y = if point.y - 12.0 - size.y >= plot.top() {
        point.y - 12.0 - size.y
    } else {
        point.y + 12.0
    };
    let origin = egui::pos2(
        x.clamp(plot.left(), (plot.right() - size.x).max(plot.left())),
        y.clamp(plot.top(), (plot.bottom() - size.y).max(plot.top())),
    );
    let label = Rect::from_min_size(origin, size);
    painter.rect_filled(label, 6, Color32::WHITE);
    painter.rect_stroke(
        label,
        6,
        Stroke::new(1.0_f32, Color32::from_rgb(202, 214, 230)),
        egui::StrokeKind::Inside,
    );
    painter.galley(origin + egui::vec2(10.0, 7.0), galley, INK);
}

pub(crate) struct Tick {
    pub(crate) value: f64,
    pub(crate) label: String,
}

pub(crate) struct Axis {
    // minimum/maximum/ticks 使用变换后的坐标；原值 = origin + 坐标 * factor。
    pub(crate) minimum: f64,
    pub(crate) maximum: f64,
    pub(crate) origin: f64,
    pub(crate) factor: f64,
    pub(crate) ticks: Vec<Tick>,
    pub(crate) caption: String,
}

impl Axis {
    /// 输入应为有限且有序的边界；相同边界会自动留白，避免常量曲线除以零。
    pub(crate) fn new(
        name: &str,
        minimum: f64,
        maximum: f64,
        intervals: usize,
        round_bounds: bool,
    ) -> Self {
        let magnitude = minimum.abs().max(maximum.abs());
        let span = maximum - minimum;
        // 较大基准上的微小变化单独标明基准，避免刻度挤满重复的前导数字。
        let origin = if span.is_finite() && span > 0.0 && magnitude / span > 100_000.0 {
            minimum
        } else {
            0.0
        };
        let magnitude = (minimum - origin).abs().max((maximum - origin).abs());
        let exponent = if magnitude > 0.0 && !(0.001..1_000_000.0).contains(&magnitude) {
            ((magnitude.log10() / 3.0).floor() as i32 * 3).clamp(-300, 300)
        } else {
            0
        };
        let factor = 10.0_f64.powi(exponent);
        let mut low = (minimum - origin) / factor;
        let mut high = (maximum - origin) / factor;
        if low == high {
            let padding = low.abs().max(1.0) * 0.05;
            low -= padding;
            high += padding;
        } else if round_bounds {
            let padding = (high - low) * 0.06;
            low -= padding;
            high += padding;
        }
        let step = nice_step((high - low) / intervals.max(2) as f64);
        if round_bounds {
            low = (low / step).floor() * step;
            high = (high / step).ceil() * step;
        }
        let first = (low / step - 1.0e-9).ceil();
        let last = (high / step + 1.0e-9).floor();
        let count = ((last - first).max(0.0) as usize + 1).min(32);
        let precision = tick_precision(step);
        let ticks = (0..count)
            .map(|index| {
                let value = (first + index as f64) * step;
                let value = if value.abs() < step * 1.0e-8 {
                    0.0
                } else {
                    value
                };
                Tick {
                    value,
                    label: format!("{value:.precision$}"),
                }
            })
            .collect();
        let mut caption = name.to_owned();
        if origin != 0.0 {
            caption.push_str(&format!(" - ({})", format_number(origin)));
        }
        if exponent != 0 {
            caption.push_str(&format!(" (×10^{exponent})"));
        }
        Self {
            minimum: low,
            maximum: high,
            origin,
            factor,
            ticks,
            caption,
        }
    }

    pub(crate) fn fraction(&self, value: f64) -> f64 {
        self.tick_fraction((value - self.origin) / self.factor)
    }

    pub(crate) fn tick_fraction(&self, scaled_value: f64) -> f64 {
        (scaled_value - self.minimum) / (self.maximum - self.minimum)
    }
}

// 在 1、2、2.5、5、10 的十进制倍数中取向上覆盖的步长。
fn nice_step(raw: f64) -> f64 {
    let power = 10.0_f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let multiple = [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .find(|multiple| fraction <= *multiple * (1.0 + 1.0e-12))
        .unwrap_or(10.0);
    multiple * power
}

pub(crate) fn tick_precision(step: f64) -> usize {
    let mut precision = (-step.log10().floor()).max(0.0) as usize;
    let scaled = step * 10.0_f64.powi(precision.min(15) as i32);
    if (scaled - scaled.round()).abs() > 1.0e-8 {
        precision += 1;
    }
    precision.min(15)
}
