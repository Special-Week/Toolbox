//! 自然端点条件下的三次均匀参数 B 样条；x/y 均以等距参数插值，x 可非等距。
//! 求 y(x) 必须先反求单调分支上的参数；CSV 导出使用独立线程和原子替换。
use crate::app::ToolboxApp;
use crate::calculators::common::{format_number, info_note, render_error_message, render_metrics};
use crate::catalog::ToolDef;
use crate::{BLUE, INK, MUTED};
use eframe::egui::{self, Color32, RichText, ScrollArea, Vec2};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

mod chart;
use chart::draw_chart;

const MIN_POINT_COUNT: usize = 2;
const MAX_POINT_COUNT: usize = 500;
const MAX_SAMPLE_COUNT: usize = 5_000_000;
const MAX_PREVIEW_COUNT: usize = 100;
const PREVIEW_ROWS_PER_PAGE: usize = 10;
const CHART_SAMPLES: usize = 180;
const DEFAULT_POINTS: [(f64, f64); 9] = [
    (250.0, 2002.0),
    (550.0, 6531.0),
    (850.0, 12285.0),
    (1150.0, 18708.0),
    (1450.0, 25475.0),
    (1750.0, 32170.0),
    (2050.0, 38455.0),
    (2350.0, 43954.0),
    (2650.0, 48340.0),
];

/// 工作线程只通过消息和原子进度与 UI 通信，不直接修改应用状态。
pub(crate) struct ExportJob {
    pub(crate) receiver: Receiver<Result<PathBuf, String>>,
    pub(crate) cancelled: Arc<AtomicBool>,
    pub(crate) progress: Arc<AtomicUsize>,
    pub(crate) total: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy)]
struct Cubic {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl Cubic {
    fn value(self, u: f64) -> f64 {
        self.d
            .mul_add(u, self.c)
            .mul_add(u, self.b)
            .mul_add(u, self.a)
    }

    fn derivative(self, u: f64) -> f64 {
        (3.0 * self.d).mul_add(u, 2.0 * self.c).mul_add(u, self.b)
    }

    fn minimum_derivative(self) -> f64 {
        let mut minimum = self.derivative(0.0).min(self.derivative(1.0));
        if self.d > 0.0 {
            let vertex = -self.c / (3.0 * self.d);
            if (0.0..1.0).contains(&vertex) {
                minimum = minimum.min(self.derivative(vertex));
            }
        }
        minimum
    }
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    x: Cubic,
    y: Cubic,
}

struct UniformCubicSpline {
    points: Vec<Point>,
    segments: Vec<Segment>,
    // 计算使用去基准、归一化后的坐标；显示和导出时恢复原始量纲。
    origin: Point,
    scale: Point,
}

impl UniformCubicSpline {
    fn interpolate(points: &[Point]) -> Result<Self, String> {
        if !(MIN_POINT_COUNT..=MAX_POINT_COUNT).contains(&points.len()) {
            return Err("Q 型值点数量必须在 2 到 500 之间".to_owned());
        }
        for (index, point) in points.iter().enumerate() {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(format!("Q{index} 的坐标必须是有限数值"));
            }
            if index > 0 && point.x <= points[index - 1].x {
                return Err(format!("Q{index} 的 x 必须大于 Q{} 的 x", index - 1));
            }
        }

        let origin = points[0];
        let x_scale = points[points.len() - 1].x - origin.x;
        let y_scale = points
            .iter()
            .map(|point| (point.y - origin.y).abs())
            .fold(0.0, f64::max);
        let scale = Point {
            x: x_scale,
            y: if y_scale == 0.0 { 1.0 } else { y_scale },
        };
        if !scale.x.is_finite() || !scale.y.is_finite() || scale.x <= 0.0 {
            return Err("坐标跨度超出双精度范围，请检查输入".to_owned());
        }
        let normalized: Vec<_> = points
            .iter()
            .map(|point| Point {
                x: (point.x - origin.x) / scale.x,
                y: (point.y - origin.y) / scale.y,
            })
            .collect();
        let controls = solve_controls(&normalized);
        let segments: Vec<_> = controls
            .windows(4)
            .enumerate()
            .map(|(index, window)| {
                let window = [window[0], window[1], window[2], window[3]];
                Segment {
                    x: coefficients(window.map(|point| point.x), normalized[index].x),
                    y: coefficients(window.map(|point| point.y), normalized[index].y),
                }
            })
            .collect();
        for (index, segment) in segments.iter().enumerate() {
            if segment.x.minimum_derivative() < -32.0 * f64::EPSILON {
                return Err(format!(
                    "Q{index} 到 Q{} 之间的 x 出现回折，无法唯一计算 y(x)",
                    index + 1
                ));
            }
        }
        Ok(Self {
            points: points.to_vec(),
            segments,
            origin,
            scale,
        })
    }

    fn y_at_x(&self, x: f64) -> Result<f64, String> {
        if !x.is_finite() {
            return Err("x 必须是有限数值".to_owned());
        }
        let insertion = self.points.partition_point(|point| point.x < x);
        if let Some(point) = self.points.get(insertion)
            && point.x == x
        {
            return Ok(point.y);
        }
        let target = (x - self.origin.x) / self.scale.x;
        if !target.is_finite() {
            return Err("外推距离超出双精度范围".to_owned());
        }
        let (segment, lower, upper) = if insertion == 0 || insertion == self.points.len() {
            let right = insertion == self.points.len();
            let segment = self.endpoint_polynomial(right);
            let (lower, upper) = external_bracket(segment.x, target, right)?;
            (segment, lower, upper)
        } else {
            (self.segments[insertion - 1], 0.0, 1.0)
        };
        let parameter = invert(segment.x, target, lower, upper)?;
        let y = segment
            .y
            .value(parameter)
            .mul_add(self.scale.y, self.origin.y);
        if y.is_finite() {
            Ok(y)
        } else {
            Err(format!("x={x} 对应的 y 超出双精度范围"))
        }
    }

    /// 将端点平移为局部参数零点，保留自然条件（二阶导数为零）下的端点三次式。
    fn endpoint_polynomial(&self, right: bool) -> Segment {
        let index = if right { self.segments.len() - 1 } else { 0 };
        let parameter = if right { 1.0 } else { 0.0 };
        let point = if right {
            self.points[self.points.len() - 1]
        } else {
            self.origin
        };
        let segment = self.segments[index];
        Segment {
            x: Cubic {
                a: (point.x - self.origin.x) / self.scale.x,
                b: segment.x.derivative(parameter),
                c: 0.0,
                d: segment.x.d,
            },
            y: Cubic {
                a: (point.y - self.origin.y) / self.scale.y,
                b: segment.y.derivative(parameter),
                c: 0.0,
                d: segment.y.d,
            },
        }
    }
}

fn coefficients(p: [f64; 4], value: f64) -> Cubic {
    let delta = [p[1] - p[0], p[2] - p[1], p[3] - p[2]];
    let mut c = (delta[1] - delta[0]) / 2.0;
    let mut d = (delta[2] - 2.0 * delta[1] + delta[0]) / 6.0;
    let roundoff = 32.0 * f64::EPSILON * p.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if c.abs() <= roundoff {
        c = 0.0;
    }
    if d.abs() <= roundoff {
        d = 0.0;
    }
    Cubic {
        a: value,
        b: (p[2] - p[0]) / 2.0,
        c,
        d,
    }
}

/// 内部控制点满足 P[i−1]+4P[i]+P[i+1]=6Q[i]，用三对角消元同时求 x/y。
/// 两侧虚控制点由自然端点条件给出；只有两个 Q 点时自然退化为直线。
fn solve_controls(points: &[Point]) -> Vec<Point> {
    let count = points.len();
    let mut controls = vec![Point::default(); count + 2];
    controls[1] = points[0];
    controls[count] = points[count - 1];
    let unknowns = count - 2;
    let mut diagonal = vec![4.0; unknowns];
    let mut rhs = vec![Point::default(); unknowns];
    for row in 0..unknowns {
        rhs[row] = Point {
            x: 6.0 * points[row + 1].x,
            y: 6.0 * points[row + 1].y,
        };
        if row == 0 {
            rhs[row].x -= points[0].x;
            rhs[row].y -= points[0].y;
        }
        if row + 1 == unknowns {
            rhs[row].x -= points[count - 1].x;
            rhs[row].y -= points[count - 1].y;
        }
        if row > 0 {
            let factor = 1.0 / diagonal[row - 1];
            diagonal[row] -= factor;
            rhs[row].x -= factor * rhs[row - 1].x;
            rhs[row].y -= factor * rhs[row - 1].y;
        }
    }
    for row in (0..unknowns).rev() {
        let next = if row + 1 == unknowns {
            Point::default()
        } else {
            controls[row + 3]
        };
        controls[row + 2] = Point {
            x: (rhs[row].x - next.x) / diagonal[row],
            y: (rhs[row].y - next.y) / diagonal[row],
        };
    }
    controls[0] = Point {
        x: 2.0 * controls[1].x - controls[2].x,
        y: 2.0 * controls[1].y - controls[2].y,
    };
    controls[count + 1] = Point {
        x: 2.0 * controls[count].x - controls[count - 1].x,
        y: 2.0 * controls[count].y - controls[count - 1].y,
    };
    controls
}

/// 端点向外扩大搜索区间，到 x 的首个转折处即停止，不跨越多值分支。
fn external_bracket(polynomial: Cubic, target: f64, right: bool) -> Result<(f64, f64), String> {
    let failure = || "端点外推到达 x 转折点，无法唯一反求参数".to_owned();
    if polynomial.b < 0.0 || (polynomial.b == 0.0 && polynomial.d <= 0.0) {
        return Err(failure());
    }
    let limit = if polynomial.d < 0.0 {
        (-polynomial.b / (3.0 * polynomial.d)).sqrt()
    } else {
        f64::INFINITY
    };
    let sign = if right { 1.0 } else { -1.0 };
    let mut distance = 1.0_f64.min(limit);
    for _ in 0..1024 {
        let bound = sign * distance;
        let value = polynomial.value(bound);
        if !value.is_finite() {
            return Err(failure());
        }
        if (right && value >= target) || (!right && value <= target) {
            return Ok(if right { (0.0, bound) } else { (bound, 0.0) });
        }
        if distance == limit {
            return Err(failure());
        }
        distance = (distance * 2.0).min(limit);
    }
    Err(failure())
}

/// 有括区间约束的 Newton 迭代；导数无效或步长越界时回退到二分。
fn invert(polynomial: Cubic, target: f64, mut lower: f64, mut upper: f64) -> Result<f64, String> {
    let low_x = polynomial.value(lower);
    let high_x = polynomial.value(upper);
    let span = high_x - low_x;
    if !span.is_finite() || span <= 0.0 {
        return Err("样条参数区间无效".to_owned());
    }
    let mut parameter = lower + (upper - lower) * ((target - low_x) / span);
    if !parameter.is_finite() || parameter <= lower || parameter >= upper {
        parameter = lower + (upper - lower) / 2.0;
    }
    let tolerance = 8.0 * f64::EPSILON * target.abs().max(1.0);
    for _ in 0..80 {
        let error = polynomial.value(parameter) - target;
        if !error.is_finite() {
            break;
        }
        if error.abs() <= tolerance {
            return Ok(parameter);
        }
        if error < 0.0 {
            lower = parameter;
        } else {
            upper = parameter;
        }
        let derivative = polynomial.derivative(parameter);
        let candidate = parameter - error / derivative;
        parameter = if candidate.is_finite() && candidate > lower && candidate < upper {
            candidate
        } else {
            lower + (upper - lower) / 2.0
        };
    }
    Err("反求样条参数未收敛，请检查 x 间距或外推范围".to_owned())
}

fn render_spline(app: &mut ToolboxApp, ui: &mut egui::Ui, tool: &ToolDef) {
    let mut points = Vec::new();
    let mut input_error = None;
    let (minimum, maximum, step) = ui
        .group(|ui| {
            ui.set_max_width(960.0);
            ui.label(RichText::new("插值参数").size(15.0).strong().color(INK));
            ui.add_space(4.0);
            app.input(ui, tool, "x_min", "曲线最小 x", "0", "绘图范围起点");
            app.input(ui, tool, "x_max", "曲线最大 x", "2650", "绘图范围终点");
            app.input(ui, tool, "step", "采样步进", "0.5", "必须大于 0");
            ui.add_space(4.0);
            let count = app.dynamic_count(ui, tool, "q_points", "Q 型值点", 9, 2, MAX_POINT_COUNT);
            ui.label(
                RichText::new("x 必须严格递增；新增点需要填写 x 与 y。")
                    .size(11.5)
                    .color(MUTED),
            );
            ui.add_space(4.0);
            ScrollArea::vertical()
                .id_salt("bspline_q_points")
                .max_height(270.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized([42.0, 28.0], egui::Label::new(RichText::new("点").strong()));
                        ui.add_sized(
                            [ui.available_width() * 0.5, 28.0],
                            egui::Label::new(RichText::new("x").strong()),
                        );
                        ui.add_sized(
                            [ui.available_width(), 28.0],
                            egui::Label::new(RichText::new("y").strong()),
                        );
                    });
                    for index in 0..count {
                        let (default_x, default_y) = DEFAULT_POINTS
                            .get(index)
                            .copied()
                            .map(|(x, y)| (x.to_string(), y.to_string()))
                            .unwrap_or_default();
                        let label = format!("Q{index}");
                        let x_key = format!("q_{index}_x");
                        let y_key = format!("q_{index}_y");
                        let width = ((ui.available_width() - 58.0) / 2.0).max(52.0);
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [42.0, 36.0],
                                egui::Label::new(RichText::new(label).color(BLUE)),
                            );
                            app.inline_input(ui, tool, &x_key, &default_x, width, "x");
                            app.inline_input(ui, tool, &y_key, &default_y, width, "y");
                        });
                        let x = app.number(tool, &x_key, &default_x);
                        let y = app.number(tool, &y_key, &default_y);
                        match (x, y) {
                            (Ok(x), Ok(y)) => points.push(Point { x, y }),
                            (Err(error), _) | (_, Err(error)) if input_error.is_none() => {
                                input_error = Some(format!("Q{index}：{error}"));
                            }
                            _ => {}
                        }
                    }
                });
            (
                app.number(tool, "x_min", "0"),
                app.number(tool, "x_max", "2650"),
                app.number(tool, "step", "0.5"),
            )
        })
        .inner;

    ui.add_space(8.0);
    if let Some(error) = input_error {
        render_error_message(ui, &error);
        return;
    }
    let spline = match UniformCubicSpline::interpolate(&points) {
        Ok(spline) => spline,
        Err(error) => {
            render_error_message(ui, &error);
            return;
        }
    };
    let (minimum, maximum, step) = match (minimum, maximum, step) {
        (Ok(minimum), Ok(maximum), Ok(step)) if minimum < maximum && step > 0.0 => {
            (minimum, maximum, step)
        }
        (Ok(minimum), Ok(maximum), Ok(_)) if minimum >= maximum => {
            render_error_message(ui, "曲线最小 x 必须小于曲线最大 x");
            return;
        }
        (Ok(_), Ok(_), Ok(_)) => {
            render_error_message(ui, "采样步进必须大于 0");
            return;
        }
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            render_error_message(ui, &error);
            return;
        }
    };
    let sample_count = match planned_sample_count(minimum, maximum, step) {
        Ok(count) => count,
        Err(error) => {
            render_error_message(ui, &error);
            return;
        }
    };
    let span = maximum - minimum;
    if !span.is_finite() || span <= 0.0 {
        render_error_message(ui, "曲线范围超出双精度计算范围");
        return;
    }
    let curve = (0..=CHART_SAMPLES)
        .map(|index| {
            let x = (index as f64 / CHART_SAMPLES as f64).mul_add(span, minimum);
            spline.y_at_x(x).map(|y| Point { x, y })
        })
        .collect::<Result<Vec<_>, _>>();
    let curve = match curve {
        Ok(curve) => curve,
        Err(error) => {
            render_error_message(ui, &error);
            return;
        }
    };
    let preview = (0..sample_count.min(MAX_PREVIEW_COUNT))
        .map(|index| {
            let x = sample_x(minimum, maximum, step, sample_count, index);
            spline.y_at_x(x).map(|y| Point { x, y })
        })
        .collect::<Result<Vec<_>, _>>();
    let preview = match preview {
        Ok(preview) => preview,
        Err(error) => {
            render_error_message(ui, &error);
            return;
        }
    };

    render_metrics(
        ui,
        "插值结果",
        &[
            ("Q 型值点", format!("{} 个", points.len())),
            ("采样点", format!("{} 个", sample_count)),
            ("端点条件", "自然端点".to_owned()),
        ],
    );
    let mut request_export = false;
    let mut request_cancel = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("采样结果预览").size(14.0).strong().color(INK));
        ui.label(
            RichText::new(format!("前 {} / {} 行", preview.len(), sample_count))
                .size(11.5)
                .color(MUTED),
        );
        if let Some((completed, total, cancelled)) = app.spline_export_progress() {
            let percent = completed as f32 / total.max(1) as f32 * 100.0;
            ui.label(RichText::new(format!("导出中 {percent:.0}%")).color(BLUE));
            if ui
                .add_enabled(
                    !cancelled,
                    egui::Button::new("取消导出").min_size(Vec2::new(112.0, 38.0)),
                )
                .clicked()
            {
                request_cancel = true;
            }
        } else if ui
            .add(
                egui::Button::new(
                    RichText::new("导出 CSV")
                        .size(15.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .min_size(Vec2::new(144.0, 42.0))
                .fill(BLUE)
                .corner_radius(6),
            )
            .clicked()
        {
            request_export = true;
        }
    });
    if request_cancel {
        app.cancel_spline_export();
    }
    if request_export {
        match start_csv_export(ui.ctx(), points.clone(), minimum, maximum, step) {
            Ok(Some(job)) => app.start_spline_export(job),
            Ok(None) => {}
            Err(error) => render_error_message(ui, &error),
        }
    }
    if let Some(message) = app.spline_export_message() {
        ui.label(RichText::new(message).size(11.5).color(MUTED));
    }
    render_sample_preview(ui, &preview);
    ui.add_space(4.0);
    ui.label(RichText::new("插值曲线").size(14.0).strong().color(INK));
    draw_chart(ui, &spline, &points, &curve, minimum, maximum);
    info_note(ui, tool.kind);
}

fn render_sample_preview(ui: &mut egui::Ui, preview: &[Point]) {
    let page_count = preview.len().div_ceil(PREVIEW_ROWS_PER_PAGE).max(1);
    let page_id = ui.make_persistent_id("bspline_preview_page");
    let mut page = ui
        .ctx()
        .data_mut(|data| data.get_temp::<usize>(page_id).unwrap_or(0))
        .min(page_count - 1);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(page > 0, egui::Button::new("上一页"))
            .clicked()
        {
            page -= 1;
        }
        if ui
            .add_enabled(page + 1 < page_count, egui::Button::new("下一页"))
            .clicked()
        {
            page += 1;
        }
        ui.label(
            RichText::new(format!(
                "第 {} / {} 页 · 每页 {} 行",
                page + 1,
                page_count,
                PREVIEW_ROWS_PER_PAGE
            ))
            .size(11.5)
            .color(MUTED),
        );
    });
    ui.ctx().data_mut(|data| data.insert_temp(page_id, page));

    // 表格直接参与页面布局，滚轮由外层页面处理，避免内层列表截获滚动。
    let start = page * PREVIEW_ROWS_PER_PAGE;
    let end = (start + PREVIEW_ROWS_PER_PAGE).min(preview.len());
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 7.0;
        egui::Grid::new("bspline_samples")
            .striped(true)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.add_sized([150.0, 32.0], egui::Label::new(RichText::new("x").strong()));
                ui.add_sized([150.0, 32.0], egui::Label::new(RichText::new("y").strong()));
                ui.end_row();
                for point in &preview[start..end] {
                    for value in [point.x, point.y] {
                        ui.add_sized(
                            [150.0, 30.0],
                            egui::Label::new(
                                RichText::new(format_number(value)).size(13.0).monospace(),
                            ),
                        );
                    }
                    ui.end_row();
                }
            });
    });
}

/// 先规划采样数量，允许末段短于步长但始终包含最大 x；拒绝不可分辨的步进。
fn planned_sample_count(minimum: f64, maximum: f64, step: f64) -> Result<usize, String> {
    if !minimum.is_finite() || !maximum.is_finite() || !step.is_finite() {
        return Err("采样范围和步进必须是有限数值".to_owned());
    }
    if minimum >= maximum || step <= 0.0 {
        return Err("采样范围无效或步进不大于 0".to_owned());
    }
    if minimum + step == minimum || maximum - step == maximum {
        return Err("步进小于当前坐标精度，请增大步进".to_owned());
    }
    let intervals = (maximum - minimum) / step;
    if !intervals.is_finite() {
        return Err("采样点最多为 5000000 个，请增大步进或缩小范围".to_owned());
    }
    let nearest = intervals.round();
    let intervals = if (intervals - nearest).abs() <= 8.0 * f64::EPSILON * intervals.max(1.0) {
        nearest
    } else {
        intervals
    };
    if intervals > (MAX_SAMPLE_COUNT - 1) as f64 {
        return Err("采样点最多为 5000000 个，请增大步进或缩小范围".to_owned());
    }
    let count = intervals.ceil() as usize + 1;
    if count > MAX_SAMPLE_COUNT {
        return Err("采样点最多为 5000000 个，请增大步进".to_owned());
    }
    Ok(count.max(2))
}

fn start_csv_export(
    context: &egui::Context,
    points: Vec<Point>,
    minimum: f64,
    maximum: f64,
    step: f64,
) -> Result<Option<ExportJob>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("导出完整插值结果")
        .add_filter("CSV 文件", &["csv"])
        .set_file_name("bspline_results.csv")
        .save_file()
    else {
        return Ok(None);
    };
    let path = if path.extension().is_none() {
        path.with_extension("csv")
    } else {
        path
    };
    let total = planned_sample_count(minimum, maximum, step)?;
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicUsize::new(0));
    let worker_cancelled = Arc::clone(&cancelled);
    let worker_progress = Arc::clone(&progress);
    let repaint_context = context.clone();
    thread::Builder::new()
        .name("bspline-csv-export".to_owned())
        .spawn(move || {
            let result = write_csv(
                &path,
                &points,
                minimum,
                maximum,
                step,
                total,
                &worker_cancelled,
                &worker_progress,
            );
            let _ = sender.send(result);
            repaint_context.request_repaint();
        })
        .map_err(|error| format!("无法启动 CSV 导出任务：{error}"))?;
    Ok(Some(ExportJob {
        receiver,
        cancelled,
        progress,
        total,
    }))
}

/// 同目录临时写入并同步，成功后再替换目标；取消或计算失败不会破坏旧文件。
fn write_csv(
    path: &Path,
    points: &[Point],
    minimum: f64,
    maximum: f64,
    step: f64,
    count: usize,
    cancelled: &AtomicBool,
    progress: &AtomicUsize,
) -> Result<PathBuf, String> {
    let spline = UniformCubicSpline::interpolate(points)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("无法建立暂存文件：{error}"))?;
    {
        let mut writer = BufWriter::with_capacity(256 * 1024, temporary.as_file_mut());
        writer
            .write_all(b"x,y\r\n")
            .map_err(|error| format!("写入 CSV 标题失败：{error}"))?;
        let mut previous_x = None;
        for index in 0..count {
            if index % 1024 == 0 {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("导出已取消".to_owned());
                }
                progress.store(index, Ordering::Relaxed);
            }
            let x = sample_x(minimum, maximum, step, count, index);
            if previous_x.is_some_and(|previous| previous >= x) {
                return Err("采样步进导致 x 重复，请增大步进".to_owned());
            }
            let y = spline.y_at_x(x)?;
            writeln!(writer, "{x},{y}").map_err(|error| format!("写入 CSV 数据失败：{error}"))?;
            previous_x = Some(x);
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err("导出已取消".to_owned());
        }
        writer
            .flush()
            .map_err(|error| format!("刷新 CSV 文件失败：{error}"))?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("同步 CSV 文件失败：{error}"))?;
    if cancelled.load(Ordering::Relaxed) {
        return Err("导出已取消".to_owned());
    }
    temporary
        .persist(path)
        .map_err(|error| format!("保存 CSV 文件失败：{}", error.error))?;
    progress.store(count, Ordering::Relaxed);
    Ok(path.to_owned())
}

fn sample_x(minimum: f64, maximum: f64, step: f64, count: usize, index: usize) -> f64 {
    if index + 1 == count {
        maximum
    } else {
        (index as f64).mul_add(step, minimum)
    }
}

impl ToolboxApp {
    pub(crate) fn render_bspline(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        render_spline(self, ui, tool);
    }
}
