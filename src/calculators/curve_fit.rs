//! 分析与测量数据的通用校准曲线页面；不绑定仪器类型、测量对象或单位。
use crate::app::ToolboxApp;
use crate::calculators::common::{format_number, info_note, render_error_message, render_metrics};
use crate::catalog::ToolDef;
use crate::{BLUE, BLUE_SOFT, INK, MUTED};
use eframe::egui::{self, RichText, Vec2};
use std::ops::Range;

mod chart;
mod model;
use model::{CurveFit, FitKind, MAX_POINTS};

const ROWS_PER_PAGE: usize = 10;
// 示例以 x/y 成对保存，界面不限定浓度、吸光度等具体物理量。
const EXAMPLE: [(&str, &str); 6] = [
    ("0", "0.197"),
    ("5", "0.208"),
    ("10", "0.231"),
    ("20", "0.263"),
    ("40", "0.338"),
    ("60", "0.439"),
];
const COEFFICIENT_NAMES: [&str; 4] = [
    "K0（常数项）",
    "K1（一次项）",
    "K2（二次项）",
    "K3（三次项）",
];

fn caption(name: &str, fallback: &str, variable: &str, unit: &str) -> String {
    let name = if name.trim().is_empty() {
        fallback
    } else {
        name.trim()
    };
    if unit.trim().is_empty() {
        format!("{name} {variable}")
    } else {
        format!("{name} {variable} ({})", unit.trim())
    }
}

/// 分页直接参与外层页面布局，避免数据表截获滚轮、挡住下方曲线。
fn page_range(ui: &mut egui::Ui, key: &str, count: usize) -> Range<usize> {
    if count <= ROWS_PER_PAGE {
        return 0..count;
    }
    let id = ui.make_persistent_id(key);
    let pages = count.div_ceil(ROWS_PER_PAGE);
    let mut page = ui
        .ctx()
        .data_mut(|data| data.get_temp::<usize>(id).unwrap_or(0))
        .min(pages - 1);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                page > 0,
                egui::Button::new("上一页").min_size(Vec2::new(80.0, 32.0)),
            )
            .clicked()
        {
            page -= 1;
        }
        if ui
            .add_enabled(
                page + 1 < pages,
                egui::Button::new("下一页").min_size(Vec2::new(80.0, 32.0)),
            )
            .clicked()
        {
            page += 1;
        }
        ui.label(
            RichText::new(format!("第 {} / {pages} 页 · 共 {count} 组", page + 1))
                .size(12.0)
                .color(MUTED),
        );
    });
    ui.ctx().data_mut(|data| data.insert_temp(id, page));
    let start = page * ROWS_PER_PAGE;
    start..(start + ROWS_PER_PAGE).min(count)
}

fn equation(fit: &CurveFit, full_precision: bool) -> String {
    let mut text = String::from("y = ");
    for power in (0..fit.coefficients.len()).rev() {
        let value = fit.coefficients[power];
        let first = power + 1 == fit.coefficients.len();
        if !first {
            text.push_str(if value < 0.0 { " - " } else { " + " });
        } else if value < 0.0 {
            text.push('-');
        }
        text.push_str(&if full_precision {
            value.abs().to_string()
        } else {
            format_number(value.abs())
        });
        match power {
            0 => (),
            1 => text.push_str(" × x"),
            _ => text.push_str(&format!(" × x^{power}")),
        }
    }
    text
}

fn optional_number(value: Option<f64>, missing: &str) -> String {
    value
        .map(format_number)
        .unwrap_or_else(|| missing.to_owned())
}

fn copy_report(fit: &CurveFit, points: &[(f64, f64)], x_label: &str, y_label: &str) -> String {
    let mut report = format!(
        "校准曲线拟合\n{}\n输入：{x_label}\n响应：{y_label}\n",
        equation(fit, true)
    );
    for (power, value) in fit.coefficients.iter().enumerate() {
        report.push_str(&format!("K{power} = {value}\n"));
    }
    report.push_str(&format!(
        "相关系数 r = {}\n决定系数 R² = {}\nRMSE = {}\n残差标准差 = {}\n\n序号\tx\ty\t拟合 y\t残差 y-拟合 y\n",
        fit.correlation.map(|v| v.to_string()).unwrap_or_else(|| "未定义".to_owned()),
        fit.r_squared.map(|v| v.to_string()).unwrap_or_else(|| "未定义".to_owned()),
        fit.rmse,
        fit.residual_sd.map(|v| v.to_string()).unwrap_or_else(|| "无剩余自由度".to_owned()),
    ));
    for (index, point) in points.iter().enumerate() {
        report.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            index + 1,
            point.0,
            point.1,
            fit.fitted[index],
            fit.residuals[index]
        ));
    }
    report
}

fn result_table(ui: &mut egui::Ui, fit: &CurveFit, points: &[(f64, f64)]) {
    ui.add_space(12.0);
    ui.label(
        RichText::new("各点拟合结果与残差")
            .size(14.0)
            .strong()
            .color(INK),
    );
    let range = page_range(ui, "curve_fit_result_page", points.len());
    // 窄窗拆成两行展示，避免五列挤压或产生横向滚动区域。
    let compact = ui.available_width() < 600.0;
    for index in range {
        egui::Frame::new()
            .fill(if index % 2 == 0 {
                BLUE_SOFT
            } else {
                egui::Color32::WHITE
            })
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                let labels = [
                    format!("{}.  x = {}", index + 1, format_number(points[index].0)),
                    format!("y = {}", format_number(points[index].1)),
                    format!("拟合 y = {}", format_number(fit.fitted[index])),
                    format!("残差 = {}", format_number(fit.residuals[index])),
                ];
                for row in labels.chunks(if compact { 2 } else { 4 }) {
                    ui.columns(row.len(), |columns| {
                        for (column, label) in columns.iter_mut().zip(row) {
                            column.add(
                                egui::Label::new(RichText::new(label).size(12.5).color(INK)).wrap(),
                            );
                        }
                    });
                }
            });
    }
}

fn render_results(
    ui: &mut egui::Ui,
    fit: &CurveFit,
    points: &[(f64, f64)],
    x_label: &str,
    y_label: &str,
) {
    ui.add_space(12.0);
    egui::Frame::new()
        .fill(BLUE_SOFT)
        .corner_radius(7.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("拟合方程").size(14.0).strong().color(INK));
                if ui
                    .add(egui::Button::new("复制拟合结果").min_size(Vec2::new(124.0, 34.0)))
                    .clicked()
                {
                    ui.ctx()
                        .copy_text(copy_report(fit, points, x_label, y_label));
                }
            });
            ui.add(
                egui::Label::new(
                    RichText::new(equation(fit, false))
                        .size(17.0)
                        .strong()
                        .color(BLUE),
                )
                .wrap(),
            );
            ui.label(
                RichText::new("系数对应输入数据的原始单位；复制结果包含完整精度系数和全部数据点。")
                    .size(11.5)
                    .color(MUTED),
            );
        });
    let coefficients: Vec<_> = fit
        .coefficients
        .iter()
        .enumerate()
        .rev()
        .map(|(power, value)| (COEFFICIENT_NAMES[power], format_number(*value)))
        .collect();
    render_metrics(ui, "方程系数", &coefficients);
    render_metrics(
        ui,
        "相关性与拟合质量",
        &[
            (
                "线性相关系数 r（x 与 y）",
                optional_number(fit.correlation, "响应量无变化，未定义"),
            ),
            (
                "决定系数 R²",
                optional_number(fit.r_squared, "响应量无变化，未定义"),
            ),
            ("均方根误差 RMSE", format_number(fit.rmse)),
            (
                "残差标准差",
                optional_number(fit.residual_sd, "无剩余自由度"),
            ),
            ("最大绝对残差", format_number(fit.maximum_residual)),
            (
                "剩余自由度 n-p",
                (points.len() - fit.kind.parameter_count()).to_string(),
            ),
        ],
    );
    if fit.kind.degree() > 1 || fit.kind == FitKind::ThroughOrigin {
        ui.label(RichText::new("r 表示原始 x、y 的线性相关性；R² 评价当前拟合模型。过原点拟合的 R² 可为负，且不一定等于 r²。").size(12.0).color(MUTED));
    }
    if fit.residual_sd.is_none() {
        ui.label(
            RichText::new("数据组数等于待估参数个数，没有剩余自由度；零残差不能用于证明曲线有效。")
                .size(12.0)
                .color(MUTED),
        );
    }
    ui.add_space(14.0);
    ui.label(RichText::new("校准曲线").size(14.0).strong().color(INK));
    if let Err(error) = chart::draw(ui, fit, points, x_label, y_label, false) {
        render_error_message(ui, &error);
    }
    egui::CollapsingHeader::new("残差图")
        .id_salt("curve_fit_residual_chart")
        .show(ui, |ui| {
            if let Err(error) =
                chart::draw(ui, fit, points, x_label, "残差（与响应量同单位）", true)
            {
                render_error_message(ui, &error);
            }
        });
    result_table(ui, fit, points);
}

/// 输入快照与绘图标签一起传递，页面重绘时始终使用同一批数据。
struct CurveInput {
    kind: FitKind,
    points: Result<Vec<(f64, f64)>, String>,
    x_label: String,
    y_label: String,
}

fn render_input(app: &mut ToolboxApp, ui: &mut egui::Ui, tool: &ToolDef) -> CurveInput {
    ui.label(
        RichText::new("拟合设置与校准数据")
            .size(15.0)
            .strong()
            .color(INK),
    );
    let kind = FitKind::from_index(app.select_index(
        ui,
        tool,
        "model",
        "拟合模型",
        &[
            "一次（含常数项）",
            "一次（过原点）",
            "二次多项式",
            "三次多项式",
        ],
        0,
    ));
    egui::CollapsingHeader::new("坐标名称与单位（可选）")
        .id_salt("curve_fit_axis_labels")
        .show(ui, |ui| {
            app.input(
                ui,
                tool,
                "x_name",
                "输入量名称",
                "输入量",
                "例如标准值、设定值",
            );
            app.input(
                ui,
                tool,
                "x_unit",
                "输入量单位",
                "",
                "可留空；全部 x 须使用相同单位",
            );
            app.input(
                ui,
                tool,
                "y_name",
                "响应量名称",
                "响应量",
                "例如测量读数、信号强度",
            );
            app.input(
                ui,
                tool,
                "y_unit",
                "响应量单位",
                "",
                "可留空；全部 y 须使用相同单位",
            );
        });
    let x_label = caption(
        &app.value(tool, "x_name", "输入量"),
        "输入量",
        "x",
        &app.value(tool, "x_unit", ""),
    );
    let y_label = caption(
        &app.value(tool, "y_name", "响应量"),
        "响应量",
        "y",
        &app.value(tool, "y_unit", ""),
    );
    let count = app.dynamic_count(ui, tool, "points", "校准数据", 6, 2, MAX_POINTS);
    ui.add(
        egui::Label::new(
            RichText::new("每行填写一对 x/y；采用等权最小二乘法，允许重复 x，不要求按大小排序。默认数据仅作示例。")
                .size(12.0)
                .color(MUTED),
        )
        .wrap(),
    );
    let range = page_range(ui, "curve_fit_input_page", count);
    let width = ((ui.available_width() - 44.0 - 2.0 * ui.spacing().item_spacing.x) / 2.0).max(52.0);
    ui.horizontal(|ui| {
        ui.add_sized(
            [44.0, 28.0],
            egui::Label::new(RichText::new("序号").strong()),
        );
        for label in [&x_label, &y_label] {
            ui.add_sized(
                [width, 32.0],
                egui::Label::new(RichText::new(label).strong()).wrap(),
            );
        }
    });
    for index in range {
        let (default_x, default_y) = EXAMPLE.get(index).copied().unwrap_or(("", ""));
        ui.horizontal(|ui| {
            ui.add_sized(
                [44.0, 36.0],
                egui::Label::new(RichText::new((index + 1).to_string()).color(BLUE)),
            );
            app.inline_input(
                ui,
                tool,
                &format!("point.{index}.x"),
                default_x,
                width,
                "输入量 x",
            );
            app.inline_input(
                ui,
                tool,
                &format!("point.{index}.y"),
                default_y,
                width,
                "响应量 y",
            );
        });
    }
    // 读取全部页的数据后再拟合；不可把当前可见页误当成完整数据集。
    let points = (0..count)
        .map(|index| {
            let (default_x, default_y) = EXAMPLE.get(index).copied().unwrap_or(("", ""));
            let x = app
                .number(tool, &format!("point.{index}.x"), default_x)
                .map_err(|error| format!("第 {} 组 x：{error}", index + 1))?;
            let y = app
                .number(tool, &format!("point.{index}.y"), default_y)
                .map_err(|error| format!("第 {} 组 y：{error}", index + 1))?;
            Ok((x, y))
        })
        .collect();
    CurveInput {
        kind,
        points,
        x_label,
        y_label,
    }
}

impl ToolboxApp {
    pub(crate) fn render_curve_fit(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.vertical(|ui| {
            ui.set_max_width(960.0);
            let input = ui.group(|ui| render_input(self, ui, tool)).inner;
            match input.points.and_then(|points| {
                CurveFit::calculate(&points, input.kind).map(|fit| (points, fit))
            }) {
                Ok((points, fit)) => {
                    render_results(ui, &fit, &points, &input.x_label, &input.y_label);
                }
                Err(error) => render_error_message(ui, &error),
            }
            info_note(ui, tool.kind);
        });
    }
}
