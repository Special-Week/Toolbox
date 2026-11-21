//! 传感器分度表（虚拟滚动表格 + 一键复制 TSV + CSV 导出）与 egui_plot 交互式特性曲线。
use super::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef, theme};
use eframe::egui::{self, Align, Color32, CornerRadius, FontId, RichText, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use egui_phosphor::regular as icons;
use egui_plot::{Line, Plot, PlotPoints};
use std::io::Write;

/// 写出完整数值和 UTF-8 BOM；任一行无效时返回错误，由调用者放弃临时文件。
pub(crate) fn write_rows(
    output: &mut impl Write,
    points: &[f64],
    unit: &str,
    signal_unit: &str,
    values: impl Fn(f64) -> Result<(f64, Option<f64>), String>,
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
        super::numerics::finite(&[x, y])?;
        let row = if let Some(rate) = d {
            super::numerics::finite(&[rate])?;
            writeln!(output, "{x},{y},{rate}")
        } else {
            writeln!(output, "{x},{y},")
        };
        row.map_err(|e| e.to_string())?;
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
    if points.last().is_none_or(|x| *x < end) {
        points.push(end);
    }
    if points.len() > 50000 {
        return Err("分度表最多 50000 行".into());
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_division_range_keeps_both_endpoints_and_allows_undefined_slope() {
        let points = sample_points(100.0, 100.000000001, 1.0).unwrap();
        assert_eq!(points, [100.0, 100.000000001]);
        let mut csv = Vec::new();
        write_rows(&mut csv, &points, "°C", "mV", |temperature| Ok((temperature, None))).unwrap();
        assert!(String::from_utf8(csv).unwrap().ends_with("100,100,\n100.000000001,100.000000001,\n"));
    }
}

impl ToolboxApp {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_division_table(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        bounds: (f64, f64),
        unit: TemperatureUnit,
        signal_unit: &str,
        curve: impl Fn(f64) -> Result<f64, String>,
        slope: Option<&dyn Fn(f64) -> Result<Option<f64>, String>>,
    ) {
        let p = theme::palette(ui.ctx());
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(p.surface)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(CornerRadius::same(12))
            .shadow(p.shadow)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::CollapsingHeader::new(
                    RichText::new(format!("{} 分度表与特性曲线", icons::TABLE))
                        .size(14.0)
                        .strong()
                        .color(p.ink),
                )
                .id_salt((tool.id, key))
                .show(ui, |ui| {
                    let result = (|| {
                        let (min, max) = (unit.display(bounds.0), unit.display(bounds.1));
                        let start_key = format!("table.{key}.{}.start", unit.0);
                        let end_key = format!("table.{key}.{}.end", unit.0);
                        let step_key = format!("table.{key}.{}.step", unit.0);
                        ui.add_space(6.0);
                        if ui.available_width() >= 680.0 {
                            ui.columns(3, |cols| {
                                self.input(
                                    &mut cols[0],
                                    tool,
                                    &start_key,
                                    &format!("起始温度 ({})", unit.symbol()),
                                    &min.to_string(),
                                    "分区内",
                                );
                                self.input(
                                    &mut cols[1],
                                    tool,
                                    &end_key,
                                    &format!("终止温度 ({})", unit.symbol()),
                                    &max.to_string(),
                                    "分区内",
                                );
                                self.input(
                                    &mut cols[2],
                                    tool,
                                    &step_key,
                                    &format!("温度步长 ({})", unit.symbol()),
                                    &unit.span().to_string(),
                                    "最多 50000 行",
                                );
                            });
                        } else {
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
                        }
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
                        let values = |x: f64| -> Result<(f64, Option<f64>), String> {
                            let t = unit.celsius(x).clamp(bounds.0, bounds.1);
                            let rate = if let Some(slope) = slope {
                                slope(t)?
                            } else {
                                let delta = (t.abs() * f64::EPSILON * 8.0).max(0.01);
                                let lo = (t - delta).max(bounds.0);
                                let hi = (t + delta).min(bounds.1);
                                Some((curve(hi)? - curve(lo)?) / (hi - lo))
                            };
                            Ok((curve(t)?, rate.map(|rate| rate / unit.span())))
                        };

                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!("共 {} 行 · 支持丝滑滚动浏览", points.len()))
                                    .size(12.0)
                                    .color(p.muted),
                            );
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(format!(
                                                    "{} 导出完整 CSV",
                                                    icons::DOWNLOAD_SIMPLE
                                                ))
                                                .size(12.5)
                                                .color(Color32::WHITE),
                                            )
                                            .fill(p.accent)
                                            .stroke(Stroke::NONE)
                                            .corner_radius(CornerRadius::same(8))
                                            .min_size(Vec2::new(132.0, 32.0)),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                        && let Some(path) = rfd::FileDialog::new()
                                            .add_filter("CSV", &["csv"])
                                            .set_file_name(format!("{}_division.csv", tool.id))
                                            .save_file()
                                    {
                                        let output = (|| -> Result<(), String> {
                                            let parent = path.parent().ok_or("无效保存目录")?;
                                            let mut file = tempfile::NamedTempFile::new_in(parent)
                                                .map_err(|e| e.to_string())?;
                                            write_rows(
                                                &mut file,
                                                &points,
                                                unit.symbol(),
                                                signal_unit,
                                                values,
                                            )?;
                                            file.persist(&path).map_err(|e| e.to_string())?;
                                            Ok(())
                                        })();
                                        match output {
                                            Ok(()) => theme::toast_success(
                                                ui.ctx(),
                                                format!(
                                                    "已导出 {} 行至 {}",
                                                    points.len(),
                                                    path.display()
                                                ),
                                            ),
                                            Err(e) => {
                                                theme::toast_error(
                                                    ui.ctx(),
                                                    format!("导出失败：{e}"),
                                                );
                                            }
                                        }
                                    }

                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(format!("{} 复制表格 (Excel)", icons::COPY))
                                                .size(12.5)
                                                .color(p.ink),
                                        )
                                        .fill(p.surface_subtle)
                                        .stroke(Stroke::new(1.0_f32, p.border_strong))
                                        .corner_radius(CornerRadius::same(8))
                                        .min_size(Vec2::new(136.0, 32.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text("以制表符分隔格式复制，可直接粘贴到 Excel")
                                    .clicked()
                                {
                                    let limit = points.len().min(2000);
                                    let mut tsv = format!(
                                        "温度 ({})\t信号 ({signal_unit})\t变化率 ({signal_unit}/{})\n",
                                        unit.symbol(),
                                        unit.symbol()
                                    );
                                    let mut ok = true;
                                    for &x in points.iter().take(limit) {
                                        if let Ok((y, d)) = values(x) {
                                            tsv.push_str(&format!(
                                                "{}\t{}\t{}\n",
                                                format_number(x),
                                                format_number(y),
                                                d.map(format_number).unwrap_or_default()
                                            ));
                                        } else {
                                            ok = false;
                                            break;
                                        }
                                    }
                                    if ok {
                                        ui.ctx().copy_text(tsv);
                                        if points.len() > limit {
                                            theme::toast_info(
                                                ui.ctx(),
                                                format!(
                                                    "已复制前 {limit} 行（全表 {} 行请使用 CSV 导出）",
                                                    points.len()
                                                ),
                                            );
                                        } else {
                                            theme::toast_success(
                                                ui.ctx(),
                                                format!("已复制全部 {limit} 行分度表，可直接粘贴至 Excel"),
                                            );
                                        }
                                    }
                                }
                            });
                        });

                        ui.add_space(8.0);
                        // 使用 egui_extras::TableBuilder 实现虚拟滚动与吸顶表头
                        egui::Frame::new()
                            .fill(p.surface_subtle)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                let table_height = (points.len() as f32 * 26.0 + 34.0).clamp(130.0, 300.0);
                                let mut row_error: Option<String> = None;
                                TableBuilder::new(ui)
                                    .id_salt((tool.id, key, "virtual_table"))
                                    .striped(true)
                                    .max_scroll_height(table_height)
                                    .column(Column::remainder().at_least(110.0))
                                    .column(Column::remainder().at_least(120.0))
                                    .column(Column::remainder().at_least(130.0))
                                    .header(28.0, |mut header| {
                                        header.col(|ui| {
                                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                ui.label(
                                                    RichText::new(format!("温度 ({})", unit.symbol()))
                                                        .size(12.5)
                                                        .strong()
                                                        .color(p.ink),
                                                );
                                            });
                                        });
                                        header.col(|ui| {
                                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                ui.label(
                                                    RichText::new(format!("信号 ({signal_unit})"))
                                                        .size(12.5)
                                                        .strong()
                                                        .color(p.ink),
                                                );
                                            });
                                        });
                                        header.col(|ui| {
                                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "变化率 ({signal_unit}/{})",
                                                        unit.symbol()
                                                    ))
                                                    .size(12.5)
                                                    .strong()
                                                    .color(p.ink),
                                                );
                                            });
                                        });
                                    })
                                    .body(|body| {
                                        body.rows(25.0, points.len(), |mut row| {
                                            let x = points[row.index()];
                                            match values(x) {
                                                Ok((y, d)) => {
                                                    row.col(|ui| {
                                                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                            ui.label(
                                                                RichText::new(format_number(x))
                                                                    .font(FontId::monospace(12.5))
                                                                    .color(p.ink),
                                                            );
                                                        });
                                                    });
                                                    row.col(|ui| {
                                                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                            ui.label(
                                                                RichText::new(format_number(y))
                                                                    .font(FontId::monospace(12.5))
                                                                    .color(p.accent),
                                                            );
                                                        });
                                                    });
                                                    row.col(|ui| {
                                                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                                            ui.label(
                                                                RichText::new(d.map(format_number).unwrap_or_else(|| "—".to_owned()))
                                                                    .font(FontId::monospace(12.5))
                                                                    .color(p.muted),
                                                            );
                                                        });
                                                    });
                                                }
                                                Err(e) => {
                                                    if row_error.is_none() {
                                                        row_error = Some(e);
                                                    }
                                                }
                                            }
                                        });
                                    });
                                if let Some(err) = row_error {
                                    render_error_message(ui, &err);
                                }
                            });

                        ui.add_space(12.0);
                        let mut plot_points = Vec::with_capacity(181);
                        for i in 0..=180 {
                            let x = start + (end - start) * i as f64 / 180.0;
                            let y = curve(unit.celsius(x).clamp(bounds.0, bounds.1))?;
                            super::numerics::finite(&[x, y])?;
                            plot_points.push([x, y]);
                        }

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("特性曲线")
                                    .size(13.0)
                                    .strong()
                                    .color(p.ink),
                            );
                            ui.label(
                                RichText::new("悬停读数 · 拖拽平移 · 右键框选放大 · 双击复位")
                                    .size(11.0)
                                    .color(p.muted),
                            );
                        });
                        ui.add_space(4.0);
                        let x_sym = unit.symbol().to_owned();
                        let y_sym = signal_unit.to_owned();
                        egui::Frame::new()
                            .fill(p.surface_subtle)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(8)
                            .show(ui, |ui| {
                                Plot::new((tool.id, key, "characteristic_plot"))
                                    .height(250.0)
                                    .allow_scroll(false)
                                    .allow_zoom(false)
                                    .allow_boxed_zoom(true)
                                    .allow_drag(true)
                                    .x_axis_label(format!("温度 ({x_sym})"))
                                    .y_axis_label(format!("信号 ({y_sym})"))
                                    .label_formatter(move |_name, value| {
                                        format!(
                                            "温度 = {} {x_sym}\n信号 = {} {y_sym}",
                                            format_number(value.x),
                                            format_number(value.y)
                                        )
                                    })
                                    .show(ui, |plot_ui| {
                                        plot_ui.line(
                                            Line::new("特性曲线", PlotPoints::from(plot_points))
                                                .color(p.accent)
                                                .width(2.2_f32),
                                        );
                                    });
                            });
                        Ok::<_, String>(())
                    })();
                    if let Err(e) = result {
                        render_error_message(ui, &e);
                    }
                });
            });
    }
}
