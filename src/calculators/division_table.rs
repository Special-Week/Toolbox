//! 传感器分度表、分页预览及 CSV 导出；所有温度曲线回调均接收摄氏温度。
use super::{calibration::TemperatureUnit, common::*};
use crate::{BLUE, INK, MUTED, app::ToolboxApp, catalog::ToolDef};
use eframe::egui::{self, RichText};
use std::io::Write;

/// 写出完整数值和 UTF-8 BOM；任一行无效时返回错误，由调用者放弃临时文件。
pub(crate) fn write_rows(
    output: &mut impl Write,
    points: &[f64],
    unit: &str,
    signal_unit: &str,
    values: impl Fn(f64) -> Result<(f64, f64), String>,
) -> Result<(), String> {
    output
        .write_all(b"\xEF\xBB\xBF")
        .map_err(|e| e.to_string())?;
    writeln!(
        output,
        "温度 ({unit}),信号 ({signal_unit}),变化率 ({signal_unit}/{unit})"
    )
    .map_err(|e| e.to_string())?;
    for &x in points {
        let (y, d) = values(x)?;
        super::numerics::finite(&[x, y, d])?;
        writeln!(output, "{x},{y},{d}").map_err(|e| e.to_string())?;
    }
    output.flush().map_err(|e| e.to_string())
}

/// 由索引计算采样点，避免逐次累加误差；末点包含终值，总行数最多 50,000。
pub(crate) fn sample_points(start: f64, end: f64, step: f64) -> Result<Vec<f64>, String> {
    super::numerics::finite(&[start, end, step])?;
    if start >= end || step <= 0.0 {
        return Err("起点须小于终点，步长须大于 0".into());
    }
    let count = ((end - start) / step).floor();
    if !count.is_finite() || count > 49999.0 {
        return Err("分度表最多 50000 行，请增大步长".into());
    }
    let mut points = Vec::new();
    for i in 0..=count as usize {
        let x = start + i as f64 * step;
        if points.last().is_some_and(|v| *v >= x) {
            return Err("步长小于当前数值的可分辨精度".into());
        }
        if x <= end {
            points.push(x);
        }
    }
    if points
        .last()
        .is_none_or(|x| (x - end).abs() > 1e-10 * end.abs().max(1.0))
    {
        points.push(end);
    } else {
        *points.last_mut().unwrap() = end;
    }
    if points.len() > 50000 {
        return Err("分度表最多 50000 行".into());
    }
    Ok(points)
}

impl ToolboxApp {
    pub(crate) fn render_division_table(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        bounds: (f64, f64),
        unit: TemperatureUnit,
        signal_unit: &str,
        curve: impl Fn(f64) -> Result<f64, String>,
    ) {
        egui::CollapsingHeader::new("分度表与特性曲线")
            .id_salt((tool.id, key))
            .show(ui, |ui| {
                let result = (|| {
                    let (min, max) = (unit.display(bounds.0), unit.display(bounds.1));
                    let start_key = format!("table.{key}.{}.start", unit.0);
                    let end_key = format!("table.{key}.{}.end", unit.0);
                    let step_key = format!("table.{key}.{}.step", unit.0);
                    self.input(
                        ui,
                        tool,
                        &start_key,
                        &format!("起始温度 ({})", unit.symbol()),
                        &min.to_string(),
                        "位于所选分区内",
                    );
                    self.input(
                        ui,
                        tool,
                        &end_key,
                        &format!("终止温度 ({})", unit.symbol()),
                        &max.to_string(),
                        "位于所选分区内",
                    );
                    self.input(
                        ui,
                        tool,
                        &step_key,
                        &format!("温度步长 ({})", unit.symbol()),
                        &unit.span().to_string(),
                        "最多 50000 行",
                    );
                    let start = self.number(tool, &start_key, &min.to_string())?;
                    let end = self.number(tool, &end_key, &max.to_string())?;
                    if start < min || end > max {
                        return Err("分度表范围超出所选曲线或校准分区".into());
                    }
                    let points = sample_points(
                        start,
                        end,
                        self.number(tool, &step_key, &unit.span().to_string())?,
                    )?;
                    let values = |x: f64| -> Result<(f64, f64), String> {
                        let t = unit.celsius(x);
                        let delta = (t.abs() * f64::EPSILON * 8.0).max(0.01);
                        // 曲线边界使用单侧差分；显示为 °F 时再按温差比例换算导数。
                        let lo = (t - delta).max(bounds.0);
                        let hi = (t + delta).min(bounds.1);
                        Ok((
                            curve(t)?,
                            (curve(hi)? - curve(lo)?) / (hi - lo) / unit.span(),
                        ))
                    };
                    if ui
                        .add_sized([154.0, 36.0], egui::Button::new("导出完整 CSV"))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV", &["csv"])
                            .set_file_name(format!("{}_division.csv", tool.id))
                            .save_file()
                        {
                            let output = (|| -> Result<(), String> {
                                let parent = path.parent().ok_or("无效保存目录")?;
                                // 与目标同目录写临时文件，全部成功后再替换目标。
                                let mut file = tempfile::NamedTempFile::new_in(parent)
                                    .map_err(|e| e.to_string())?;
                                write_rows(&mut file, &points, unit.symbol(), signal_unit, values)?;
                                file.persist(&path).map_err(|e| e.to_string())?;
                                Ok(())
                            })();
                            match output {
                                Ok(()) => {
                                    ui.label(format!(
                                        "已保存 {} 行：{}",
                                        points.len(),
                                        path.display()
                                    ));
                                }
                                Err(e) => render_error_message(ui, &e),
                            }
                        }
                    }
                    let page_key = format!("table.{key}.page");
                    let page_count = points.len().div_ceil(10);
                    let mut page = self
                        .value(tool, &page_key, "1")
                        .parse::<usize>()
                        .unwrap_or(1)
                        .clamp(1, page_count);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(page > 1, egui::Button::new("上一页"))
                            .clicked()
                        {
                            page -= 1;
                        }
                        ui.label(format!("{page} / {page_count}"));
                        if ui
                            .add_enabled(page < page_count, egui::Button::new("下一页"))
                            .clicked()
                        {
                            page += 1;
                        }
                    });
                    self.set_value(tool, &page_key, page.to_string());
                    let offset = (page - 1) * 10;
                    ui.label(
                        RichText::new(format!("共 {} 行 · 当前第 {} 页", points.len(), page))
                            .color(MUTED),
                    );
                    egui::Grid::new((tool.id, key, "table-grid"))
                        .striped(true)
                        .spacing([24.0, 9.0])
                        .show(ui, |ui| -> Result<(), String> {
                            ui.strong(format!("温度 ({})", unit.symbol()));
                            ui.strong(format!("信号 ({signal_unit})"));
                            ui.strong(format!("变化率 ({signal_unit}/{})", unit.symbol()));
                            ui.end_row();
                            for &x in points.iter().skip(offset).take(10) {
                                let (y, d) = values(x)?;
                                ui.label(format_number(x));
                                ui.label(format_number(y));
                                ui.label(format_number(d));
                                ui.end_row();
                            }
                            Ok(())
                        })
                        .inner?;
                    let mut plot = Vec::new();
                    for i in 0..=100 {
                        let x = start + (end - start) * i as f64 / 100.;
                        plot.push((x, curve(unit.celsius(x))?));
                    }
                    let y0 = plot.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
                    let y1 = plot.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                    super::numerics::finite(&[y0, y1])?;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().max(150.), 250.),
                        egui::Sense::hover(),
                    );
                    let area = egui::Rect::from_min_max(
                        rect.min + egui::vec2(85., 22.),
                        rect.max - egui::vec2(30., 36.),
                    );
                    let painter = ui.painter_at(rect);
                    let font = egui::FontId::proportional(11.0);
                    let x_axis = super::axis::Axis::new(
                        unit.symbol(),
                        start,
                        end,
                        (area.width() / 95.0).floor().max(2.0) as usize,
                        false,
                    );
                    let y_axis = super::axis::Axis::new(signal_unit, y0, y1, 4, true);
                    for tick in &x_axis.ticks {
                        let x =
                            area.left() + x_axis.tick_fraction(tick.value) as f32 * area.width();
                        painter.text(
                            egui::pos2(x, area.bottom() + 10.0),
                            egui::Align2::CENTER_TOP,
                            &tick.label,
                            font.clone(),
                            MUTED,
                        );
                    }
                    for tick in &y_axis.ticks {
                        let y =
                            area.bottom() - y_axis.tick_fraction(tick.value) as f32 * area.height();
                        painter.line_segment(
                            [egui::pos2(area.left(), y), egui::pos2(area.right(), y)],
                            egui::Stroke::new(0.5_f32, MUTED.gamma_multiply(0.3)),
                        );
                        painter.text(
                            egui::pos2(area.left() - 8.0, y),
                            egui::Align2::RIGHT_CENTER,
                            &tick.label,
                            font.clone(),
                            MUTED,
                        );
                    }
                    painter.text(
                        rect.min,
                        egui::Align2::LEFT_TOP,
                        &y_axis.caption,
                        font.clone(),
                        INK,
                    );
                    painter.text(
                        egui::pos2(area.right(), rect.bottom()),
                        egui::Align2::RIGHT_BOTTOM,
                        &x_axis.caption,
                        font,
                        INK,
                    );
                    let xy: Vec<_> = plot
                        .iter()
                        .map(|&(x, y)| {
                            egui::pos2(
                                area.left() + x_axis.fraction(x) as f32 * area.width(),
                                area.bottom() - y_axis.fraction(y) as f32 * area.height(),
                            )
                        })
                        .collect();
                    painter.add(egui::Shape::line(xy, egui::Stroke::new(1.8_f32, BLUE)));
                    Ok::<_, String>(())
                })();
                if let Err(e) = result {
                    render_error_message(ui, &e);
                }
            });
    }
}
