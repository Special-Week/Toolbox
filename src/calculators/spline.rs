//! 自然端点条件下的三次均匀参数 B 样条；x/y 均以等距参数插值，x 可非等距。
//! 求 y(x) 必须先反求单调分支上的参数；CSV 导出使用独立线程和原子替换。
use crate::app::ToolboxApp;
use crate::calculators::common::{
    calculation_process_lazy, format_number, format_process_computed, format_process_number,
    info_note, panel, render_error_message, render_metrics, split_workspace,
};
use crate::catalog::ToolDef;
use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, RichText, ScrollArea, Vec2};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub(super) mod model;
use model::*;
mod chart;
use chart::draw_chart;

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

type SplineProcessPayload = (UniformCubicSpline, Vec<Point>, Vec<Point>, f64, f64, f64, usize);

fn render_spline(app: &mut ToolboxApp, ui: &mut egui::Ui, tool: &ToolDef) {
    let p = theme::palette(ui.ctx());
    let mut process_payload: Option<SplineProcessPayload> = None;

    split_workspace(
        ui,
        0.40,
        app,
        |ui, app| {
            let mut points = Vec::new();
            let mut input_error = None;
            let bounds = panel(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("插值参数").size(14.5).strong().color(p.ink));
                ui.add_space(8.0);
                app.input(ui, tool, "x_min", "曲线最小 x", "0", "绘图范围起点");
                app.input(ui, tool, "x_max", "曲线最大 x", "2650", "绘图范围终点");
                app.input(ui, tool, "step", "采样步进", "0.5", "必须大于 0");
                ui.add_space(6.0);
                let count =
                    app.dynamic_count(ui, tool, "q_points", "Q 型值点", 9, 2, MAX_POINT_COUNT);
                ui.label(RichText::new("x 须严格递增。").size(11.5).color(p.muted));
                ui.add_space(6.0);
                // 分页直接参与外层页面布局，避免嵌套滚动区截断 Q 点或抢占页面滚轮。
                const Q_PAGE_SIZE: usize = 10;
                let visible_range = if count <= Q_PAGE_SIZE {
                    0..count
                } else {
                    let page_id = ui.make_persistent_id("bspline_q_points_page");
                    let total_pages = count.div_ceil(Q_PAGE_SIZE);
                    let mut q_page = ui
                        .ctx()
                        .data_mut(|data| data.get_temp::<usize>(page_id).unwrap_or(0))
                        .min(total_pages - 1);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                q_page > 0,
                                egui::Button::new("上一页").min_size(Vec2::new(72.0, 30.0)),
                            )
                            .clicked()
                        {
                            q_page -= 1;
                        }
                        if ui
                            .add_enabled(
                                q_page + 1 < total_pages,
                                egui::Button::new("下一页").min_size(Vec2::new(72.0, 30.0)),
                            )
                            .clicked()
                        {
                            q_page += 1;
                        }
                        ui.label(
                            RichText::new(format!("第 {} / {total_pages} 页", q_page + 1))
                                .size(11.5)
                                .color(p.muted),
                        );
                    });
                    ui.add_space(4.0);
                    ui.ctx().data_mut(|data| data.insert_temp(page_id, q_page));
                    let start = q_page * Q_PAGE_SIZE;
                    start..(start + Q_PAGE_SIZE).min(count)
                };

                let spacing = ui.spacing().item_spacing.x;
                let col_w = ((ui.available_width() - 42.0 - 2.0 * spacing) / 2.0).max(52.0);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [42.0, 24.0],
                        egui::Label::new(RichText::new("点").size(12.0).strong().color(p.muted)),
                    );
                    ui.add_sized(
                        [col_w, 24.0],
                        egui::Label::new(RichText::new("x").size(12.0).strong().color(p.muted)),
                    );
                    ui.add_sized(
                        [col_w, 24.0],
                        egui::Label::new(RichText::new("y").size(12.0).strong().color(p.muted)),
                    );
                });
                ui.add_space(2.0);
                for index in 0..count {
                    let (default_x, default_y) = DEFAULT_POINTS
                        .get(index)
                        .copied()
                        .map(|(x, y)| (x.to_string(), y.to_string()))
                        .unwrap_or_default();
                    let label = format!("Q{index}");
                    let x_key = format!("q_{index}_x");
                    let y_key = format!("q_{index}_y");
                    if visible_range.contains(&index) {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [42.0, 36.0],
                                egui::Label::new(RichText::new(label).strong().color(p.accent)),
                            );
                            app.inline_input(ui, tool, &x_key, &default_x, col_w, "x");
                            app.inline_input(ui, tool, &y_key, &default_y, col_w, "y");
                        });
                    }
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
                (
                    app.number(tool, "x_min", "0"),
                    app.number(tool, "x_max", "2650"),
                    app.number(tool, "step", "0.5"),
                )
            })
            .inner;
            (points, input_error, bounds)
        },
        |ui, app, (points, input_error, (minimum, maximum, step))| {
            panel(ui, |ui| {
                ui.set_width(ui.available_width());
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
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("采样结果预览").size(14.0).strong().color(p.ink));
                    ui.label(
                        RichText::new(format!("前 {} / {} 行", preview.len(), sample_count))
                            .size(11.5)
                            .color(p.muted),
                    );
                    if let Some((completed, total, cancelled)) = app.spline_export_progress() {
                        let percent = completed as f32 / total.max(1) as f32 * 100.0;
                        ui.label(RichText::new(format!("导出中 {percent:.0}%")).color(p.accent));
                        if ui
                            .add_enabled(
                                !cancelled,
                                egui::Button::new("取消导出").min_size(Vec2::new(112.0, 34.0)),
                            )
                            .clicked()
                        {
                            request_cancel = true;
                        }
                    } else if ui
                        .add(
                            egui::Button::new(
                                RichText::new("导出 CSV")
                                    .size(13.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(124.0, 36.0))
                            .fill(p.accent)
                            .corner_radius(CornerRadius::same(8)),
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
                    ui.label(RichText::new(message).size(11.5).color(p.muted));
                }
                render_sample_preview(ui, &preview);
                ui.add_space(8.0);
                ui.label(RichText::new("插值曲线").size(14.0).strong().color(p.ink));
                draw_chart(ui, &spline, &points, &curve, minimum, maximum);
                process_payload = Some((spline, points, preview, minimum, maximum, step, sample_count));
            });
        },
    );

    if let Some((spline, points, preview, minimum, maximum, step, sample_count)) = process_payload {
        calculation_process_lazy(ui, "bspline_process", || {
            let mut steps = vec![format!(
                "输入 {} 个 Q 型值点，x 严格递增；以首点为基准，对 x、y 分别归一化，端点二阶导数为零。",
                points.len()
            )];
            steps.extend(spline.normalization_steps());
            steps.push(String::new());
            steps.extend([
                "内部控制点分别满足 Pᵢ₋₁ + 4Pᵢ + Pᵢ₊₁ = 6Qᵢ，程序用三对角消元求解；每段 x(u)、y(u) 均为三次多项式。".to_owned(),
                format!("采样范围 {}～{}，步进 {}；区间数按 (最大值−最小值)/步进计算，接近整数时按整数处理。最后一点固定为最大值，若与常规采样点重合则只保留一点，共 {} 个采样点。", format_process_number(minimum), format_process_number(maximum), format_process_number(step), sample_count),
                format!("以下选取一个预览点说明分段与反算；全部 {} 个采样点采用相同方法。", sample_count),
            ]);
            let is_interpolated = |point: &Point| !points.iter().any(|q| q.x == point.x);
            let sample = preview
                .iter()
                .enumerate()
                .find(|(_, point)| {
                    point.x > points[0].x
                        && point.x < points[points.len() - 1].x
                        && is_interpolated(point)
                })
                .or_else(|| {
                    preview
                        .iter()
                        .enumerate()
                        .find(|(_, point)| is_interpolated(point))
                })
                .or_else(|| preview.iter().enumerate().next());
            if let Some((index, point)) = sample {
                steps.push(String::new());
                steps.push(format!(
                    "示例：预览采样点 {}，x = {}，y = {}。",
                    index + 1,
                    format_process_computed(point.x),
                    format_process_computed(point.y)
                ));
                match spline.process_at_x(point.x) {
                    Ok(lines) => steps.extend(lines),
                    Err(error) => steps.push(format!("示例过程无法生成：{error}")),
                }
            }
            steps
        });
    }
    info_note(ui, tool.kind);
}

fn render_sample_preview(ui: &mut egui::Ui, preview: &[Point]) {
    let p = theme::palette(ui.ctx());
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
            .color(p.muted),
        );
    });
    ui.ctx().data_mut(|data| data.insert_temp(page_id, page));

    ui.add_space(6.0);
    let start = page * PREVIEW_ROWS_PER_PAGE;
    let end = (start + PREVIEW_ROWS_PER_PAGE).min(preview.len());
    let avail = ui.available_width();
    let col_w = ((avail - 16.0) / 2.0).max(110.0);
    ScrollArea::horizontal()
        .id_salt("bspline_samples_scroll")
        .max_width(avail)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            egui::Grid::new("bspline_samples")
                .striped(true)
                .min_col_width(col_w)
                .show(ui, |ui| {
                    ui.add_sized(
                        [col_w, 30.0],
                        egui::Label::new(RichText::new("x").size(12.5).strong().color(p.muted)),
                    );
                    ui.add_sized(
                        [col_w, 30.0],
                        egui::Label::new(RichText::new("y").size(12.5).strong().color(p.muted)),
                    );
                    ui.end_row();
                    for point in &preview[start..end] {
                        for value in [point.x, point.y] {
                            ui.add_sized(
                                [col_w, 30.0],
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
#[allow(clippy::too_many_arguments)]
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

impl ToolboxApp {
    pub(crate) fn render_bspline(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        render_spline(self, ui, tool);
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;

    #[test]
    fn normalization_explains_actual_scales_and_constant_y_fallback() {
        let points: Vec<_> = DEFAULT_POINTS
            .iter()
            .map(|&(x, y)| Point { x, y })
            .collect();
        let spline = UniformCubicSpline::interpolate(&points).unwrap();
        let text = spline
            .normalization_steps()
            .join("\n")
            .replace('\u{2063}', "");
        assert!(text.contains("x₀ = 250，y₀ = 2002"), "{text}");
        assert!(text.contains("sₓ = x末 − x₀ = 2650 − 250 = 2400"), "{text}");
        assert!(text.contains("sᵧ = maxᵢ|yᵢ − y₀|"), "{text}");
        assert!(
            text.contains("Q8：|48340 − 2002| = 46338，所以 sᵧ = 46338"),
            "{text}"
        );
        assert!(text.contains("Yᵢ = (yᵢ − y₀)/sᵧ"));

        let points = [
            Point { x: 0.0, y: 10.0 },
            Point { x: 1.0, y: -11.0 },
            Point { x: 2.0, y: 28.0 },
        ];
        let text = UniformCubicSpline::interpolate(&points)
            .unwrap()
            .normalization_steps()
            .join("\n")
            .replace('\u{2063}', "");
        assert!(text.contains("Q1：|-11 − 10| = 21，所以 sᵧ = 21"), "{text}");

        let points = [Point { x: 0.0, y: 7.0 }, Point { x: 1.0, y: 7.0 }];
        let text = UniformCubicSpline::interpolate(&points)
            .unwrap()
            .normalization_steps()
            .join("\n");
        assert!(text.contains("maxᵢ|yᵢ − y₀| = 0"));
        assert!(text.contains("将 sᵧ 设为 1"));
        let points = [Point { x: 0.0, y: 7.0 }, Point { x: 1.0, y: 8.0 }];
        let text = UniformCubicSpline::interpolate(&points)
            .unwrap()
            .normalization_steps()
            .join("\n");
        assert!(text.contains("最大差值来自 Q1"));
        assert!(!text.contains("避免除以零"));
    }

    #[test]
    fn process_tracks_knots_interpolation_and_both_extrapolation_branches() {
        let points = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 2.0, y: 4.0 },
        ];
        let spline = UniformCubicSpline::interpolate(&points).unwrap();
        for (x, branch) in [
            (-0.5, "左端外推"),
            (0.0, "直接返回"),
            (0.5, "Q0～Q1 插值"),
            (1.5, "Q1～Q2 插值"),
            (2.5, "右端外推"),
        ] {
            let text = spline.process_at_x(x).unwrap().join("\n");
            assert!(text.contains(branch), "{text}");
            assert!(
                text.contains(&spline.y_at_x(x).unwrap().to_string()),
                "{text}"
            );
        }
    }
}
