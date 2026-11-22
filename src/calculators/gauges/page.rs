//! 自适应参数区、可折叠取值详情和横向结果表。表格不设内部纵向滚动，避免阻断整页滚轮。
use super::{imperial, lookup::*, pipe, report::*, smooth, specs, thread, unified};
use crate::{
    BLUE, INK, MUTED,
    app::ToolboxApp,
    calculators::common::*,
    catalog::{GaugeKind, ToolDef, ToolKind},
};
use eframe::egui::{self, RichText, ScrollArea, TextEdit, Vec2};

fn menu_text_width(ui: &egui::Ui, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), egui::FontId::proportional(14.), INK)
        .size()
        .x
}

/// 展开列表按实际文字测宽，独立于收起按钮；窄屏时允许换行，不挤压规格备注。
fn size_gauge_menu(ui: &mut egui::Ui, captions: &[String]) {
    let text_width = captions
        .iter()
        .map(|text| {
            ui.painter()
                .layout_no_wrap(text.clone(), egui::FontId::proportional(14.), INK)
                .size()
                .x
        })
        .fold(0_f32, f32::max);
    let available = (ui.ctx().screen_rect().width() - 32.).max(120.);
    ui.set_width((text_width + 44.).max(180.).min(available));
    style_gauge_menu(ui);
}

fn style_gauge_menu(ui: &mut egui::Ui) {
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    ui.spacing_mut().interact_size.y = 32.;
    ui.spacing_mut().item_spacing.y = 3.;
    ui.spacing_mut().button_padding = egui::vec2(10., 7.);
}

impl ToolboxApp {
    pub(crate) fn render_gauge(&mut self, ui: &mut egui::Ui, tool: &ToolDef, kind: GaugeKind) {
        let p = crate::theme::palette(ui.ctx());
        let mut process_report: Option<Report> = None;

        split_workspace(
            ui,
            0.38,
            self,
            |ui, app| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("计算参数").size(14.5).strong().color(p.ink));
                    ui.add_space(6.0);
                    match kind {
                        GaugeKind::Metric | GaugeKind::Trapezoidal => {
                            app.gauge_thread_form(ui, tool, kind == GaugeKind::Trapezoidal)
                        }
                        GaugeKind::Unified => app.gauge_unified_form(ui, tool),
                        GaugeKind::Pipe => app.gauge_pipe_form(ui, tool),
                        GaugeKind::Rebar => app.gauge_rebar_form(ui, tool),
                        GaugeKind::Smooth => app.gauge_smooth_form(ui, tool),
                    }
                })
                .inner
            },
            |ui, app, response| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match response {
                        Ok(report) => {
                            render_report_summary(ui, tool, &report, || {
                                serde_json::json!(app.tool_inputs(tool)).to_string()
                            });
                            process_report = Some(report);
                        }
                        Err(message) => render_error_message(ui, &message),
                    }
                });
            },
        );

        if let Some(report) = process_report {
            if !report.tables.is_empty() {
                ui.add_space(12.0);
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    render_report_tables(ui, tool, &report);
                });
            }
            calculation_process_lazy(ui, "gauge_process", || {
                let mut steps = report.process.clone();
                for note in &report.notes {
                    steps.push(format!("取值说明：{note}"));
                }
                for table in report
                    .tables
                    .iter()
                    .filter(|_| matches!(kind, GaugeKind::Pipe))
                {
                    steps.push(String::new());
                    steps.push(format!(
                        "{}：按所选规格查内置表，以下数值为表中数据；结果表按五位小数显示。",
                        table.title
                    ));
                    for row in &table.rows {
                        steps.push(String::new());
                        steps.push(
                            table
                                .headings
                                .iter()
                                .zip(row)
                                .map(|(heading, cell)| {
                                    format!(
                                        "{} = {}",
                                        heading,
                                        cell.raw
                                            .split_whitespace()
                                            .map(|part| part
                                                .parse::<f64>()
                                                .ok()
                                                .map(format_process_computed)
                                                .unwrap_or_else(|| part.to_owned()))
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("；"),
                        );
                    }
                }
                steps
            });
        }
        info_note(ui, tool.kind);
    }

    /// 推荐值与手填共用一个字符串，改变下拉不会丢失单位或额外触发滚轮选项切换。
    fn gauge_edit(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
        default: &str,
        choices: &[String],
    ) {
        let mut value = self.value(tool, key, default);
        let initial = value.clone();
        ui.push_id((tool.id, key, "field"), |ui| {
            ui.label(RichText::new(label).size(12.).color(MUTED));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.;
                ui.spacing_mut().interact_size.y = 36.;
                let width = if choices.is_empty() {
                    ui.available_width()
                } else {
                    (ui.available_width() - 128.).max(80.)
                };
                let response = ui.add_sized(
                    [width, 36.],
                    TextEdit::singleline(&mut value)
                        .id(egui::Id::new((tool.id, key, "input")))
                        .vertical_align(egui::Align::Center)
                        .margin(egui::Margin::symmetric(10, 8))
                        .background_color(egui::Color32::WHITE)
                        .frame(true),
                );
                // egui 0.31 的 TextEdit 返回去掉 margin 的内部矩形，绘制外框时须加回边距。
                // 主题默认会隐藏未激活输入框边线，所有状态都保留完整 36px 控件边界。
                ui.painter().rect_stroke(
                    response.rect.expand2(Vec2::new(10., 8.)),
                    5.,
                    egui::Stroke::new(
                        1.0_f32,
                        if response.has_focus() {
                            BLUE
                        } else {
                            egui::Color32::from_rgb(202, 214, 230)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                if !choices.is_empty() {
                    egui::ComboBox::from_id_salt((tool.id, key, "suggestions"))
                        .width(120.)
                        .height(320.)
                        .selected_text("常用规格")
                        .show_ui(ui, |ui| {
                            let unified = key == "size"
                                && matches!(tool.kind, ToolKind::Gauge(GaugeKind::Unified));
                            size_gauge_menu(ui, choices);
                            if unified {
                                let columns: Vec<_> = choices
                                    .iter()
                                    .map(|item| {
                                        imperial::un_option_columns(item).unwrap_or_else(|_| {
                                            [item.clone(), String::new(), String::new()]
                                        })
                                    })
                                    .collect();
                                let widths: [f32; 3] = std::array::from_fn(|i| {
                                    columns
                                        .iter()
                                        .map(|row| menu_text_width(ui, &row[i]))
                                        .fold(0.0, f32::max)
                                });
                                let available = (ui.ctx().screen_rect().width() - 32.).max(120.);
                                ui.set_width(
                                    (widths.iter().sum::<f32>() + 76.).max(180.).min(available),
                                );
                                for (item, row) in choices.iter().zip(&columns) {
                                    let mut job = egui::text::LayoutJob::default();
                                    let format = egui::TextFormat {
                                        font_id: egui::FontId::proportional(14.),
                                        color: ui.visuals().text_color(),
                                        ..Default::default()
                                    };
                                    job.append(&row[0], 0., format.clone());
                                    job.append(
                                        &row[1],
                                        widths[0] - menu_text_width(ui, &row[0]) + 16.,
                                        format.clone(),
                                    );
                                    job.append(
                                        &row[2],
                                        widths[1] - menu_text_width(ui, &row[1]) + 16.,
                                        format,
                                    );
                                    ui.selectable_value(&mut value, item.clone(), job);
                                }
                            } else {
                                for item in choices {
                                    ui.selectable_value(
                                        &mut value,
                                        item.clone(),
                                        RichText::new(item).size(14.),
                                    );
                                }
                            }
                        });
                }
            });
        });
        if value != initial {
            self.mark_input_edited(tool);
        }
        self.set_derived_value(tool, key, value);
    }

    fn gauge_choice(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
        options: &[String],
        preferred: &str,
    ) -> String {
        let saved = self.value(tool, key, preferred);
        let mut value = if options.contains(&saved) {
            saved
        } else if options.iter().any(|s| s == preferred) {
            preferred.into()
        } else {
            options.first().cloned().unwrap_or_default()
        };
        let initial = value.clone();
        let draw_combo = |ui: &mut egui::Ui, value: &mut String| {
            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 36.;
                egui::ComboBox::from_id_salt((tool.id, key))
                    .width(ui.available_width())
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .height(320.)
                    .selected_text(if value.is_empty() {
                        "当前参数无可用选项"
                    } else {
                        value.as_str()
                    })
                    .show_ui(ui, |ui| {
                        // 列表宽度沿用下拉按钮，避免选项高亮只覆盖菜单的一小部分。
                        style_gauge_menu(ui);
                        for item in options {
                            ui.selectable_value(value, item.clone(), RichText::new(item).size(14.));
                        }
                    });
            });
        };
        ui.push_id((tool.id, key, "choice_row"), |ui| {
            ui.label(RichText::new(label).size(12.).color(MUTED));
            draw_combo(ui, &mut value);
        });
        // 输入清空、负号等编辑中间态没有可选公差带时，不抹掉此前选择。
        if value != initial {
            self.mark_input_edited(tool);
        }
        if !options.is_empty() {
            self.set_derived_value(tool, key, value.clone());
        }
        value
    }
    fn gauge_options(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
        options: &[&str],
        default: &str,
    ) -> String {
        self.gauge_choice(
            ui,
            tool,
            key,
            label,
            &options.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
            default,
        )
    }
    /// 环规只列小写外螺纹公差带，塞规只列大写内螺纹公差带；分别保存选择。
    fn gauge_side(&mut self, ui: &mut egui::Ui, tool: &ToolDef, context: &str) -> bool {
        self.gauge_options(
            ui,
            tool,
            &format!("{context}.side"),
            "量规类型",
            &["塞规（内螺纹）", "环规（外螺纹）"],
            "塞规（内螺纹）",
        )
        .starts_with("环规")
    }
    fn gauge_grade(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        context: &str,
        trap: bool,
        ring: bool,
        options: Vec<String>,
    ) -> String {
        let options = options
            .into_iter()
            .filter(|s| {
                s.chars()
                    .last()
                    .is_some_and(|c| c.is_ascii_lowercase() == ring)
            })
            .collect::<Vec<_>>();
        self.gauge_choice(
            ui,
            tool,
            &format!("{context}.grade.{}", if ring { "ring" } else { "plug" }),
            if ring {
                "外螺纹公差带"
            } else {
                "内螺纹公差带"
            },
            &options,
            match (trap, ring) {
                (false, false) => "6H",
                (false, true) => "6g",
                (true, false) => "8H",
                (true, true) => "8e",
            },
        )
    }
    fn gauge_thread_form(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        trap: bool,
    ) -> Result<Report, String> {
        let ring = self.gauge_side(ui, tool, "thread");
        self.gauge_edit(
            ui,
            tool,
            "diameter",
            "公称直径 D (mm)",
            if trap { "24" } else { "20" },
            &specs::diameters(trap),
        );
        let d = self.number(tool, "diameter", "20");
        let pitches = if trap {
            specs::trap_pitches()
        } else {
            specs::metric_pitches(d.clone().unwrap_or(20.))
        };
        self.gauge_edit(
            ui,
            tool,
            "pitch",
            "螺距 P (mm)",
            if trap { "5" } else { "2.5" },
            &pitches.iter().map(ToString::to_string).collect::<Vec<_>>(),
        );
        let p = self.number(tool, "pitch", "2.5");
        let options = if let (Ok(d), Ok(p)) = (&d, &p) {
            thread::grades(trap, *d, *p)
        } else {
            Vec::new()
        };
        let grade = self.gauge_grade(ui, tool, "thread", trap, ring, options);
        let lines = if trap {
            self.gauge_options(
                ui,
                tool,
                "lines",
                "线数 / 头数",
                &["1", "2", "3", "4", "5"],
                "1",
            )
            .parse()
            .unwrap_or(1)
        } else {
            1
        };
        thread::calculate(trap, d?, p?, &grade, lines)
    }
    /// UN 的规格与牙数双向联动，塞规和环规各自保存工件等级。
    fn gauge_unified_form(&mut self, ui: &mut egui::Ui, tool: &ToolDef) -> Result<Report, String> {
        let ring = self.gauge_side(ui, tool, "unified");
        let sizes = imperial::un_sizes();
        let previous_size = self.value(tool, "size", "1/4");
        let previous_tpi = self.value(tool, "tpi", "20");
        let detected_previous = parse_number(&previous_tpi)
            .ok()
            .and_then(|tpi| imperial::un_series_class(&previous_size, tpi));
        let preferred_series = self.value(
            tool,
            "preferred_series",
            detected_previous.as_deref().unwrap_or(""),
        );
        if !preferred_series.is_empty() {
            self.set_derived_value(tool, "preferred_series", preferred_series.clone());
        }
        self.gauge_edit(
            ui,
            tool,
            "size",
            "公称直径 (inch，可填 1/2、1 1/4)",
            "1/4",
            &sizes,
        );
        let size = self.value(tool, "size", "1/4");
        let variants = imperial::un_variants(&size);
        if size != previous_size {
            let default_tpi = variants
                .iter()
                .find(|(series, _)| *series == preferred_series)
                .or_else(|| variants.first())
                .map(|(_, tpi)| *tpi);
            if let Some(tpi) = default_tpi {
                self.set_derived_value(tool, "tpi", tpi.to_string());
            }
        }
        let mut tpis = variants.iter().map(|(_, tpi)| *tpi).collect::<Vec<_>>();
        tpis.sort_by(|a, b| b.total_cmp(a));
        tpis.dedup_by(|a, b| same(*a, *b));
        let tpi_before_edit = self.value(tool, "tpi", "20");
        self.gauge_edit(
            ui,
            tool,
            "tpi",
            "每英寸牙数 TPI",
            "20",
            &tpis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        );
        // 手改 TPI 时立即识别系列；选择系列时反向带入该直径对应的牙数。
        let custom = "非标 / 自定义";
        let detected = self
            .number(tool, "tpi", "20")
            .ok()
            .and_then(|tpi| variants.iter().find(|(_, value)| same(*value, tpi)))
            .map(|(series, _)| series.as_str())
            .unwrap_or(custom);
        let mut series_options = Vec::new();
        if detected == custom {
            series_options.push(custom.to_owned());
        }
        for (series, _) in &variants {
            if !series_options.contains(series) {
                series_options.push(series.clone());
            }
        }
        if imperial::un_diameter(&size).is_ok() {
            if detected != custom && self.value(tool, "tpi", "20") != tpi_before_edit {
                self.set_derived_value(tool, "preferred_series", detected.into());
            }
            self.set_derived_value(tool, "series", detected.into());
            let selected = self.gauge_choice(ui, tool, "series", "螺纹系列", &series_options, detected);
            if selected != detected
                && let Some((_, tpi)) = variants.iter().find(|(series, _)| *series == selected)
            {
                self.set_derived_value(tool, "preferred_series", selected);
                self.set_derived_value(tool, "tpi", tpi.to_string());
                ui.ctx().request_repaint();
            }
        }
        let class = self.gauge_options(
            ui,
            tool,
            if ring {
                "unified.class.ring"
            } else {
                "unified.class.plug"
            },
            "工件公差等级",
            if ring {
                &["1A", "2A", "3A"]
            } else {
                &["1B", "2B", "3B"]
            },
            if ring { "2A" } else { "2B" },
        );
        unified::calculate(&unified::Input {
            size: &size,
            tpi: self.number(tool, "tpi", "20")?,
            class: &class,
        })
    }
    fn gauge_pipe_form(&mut self, ui: &mut egui::Ui, tool: &ToolDef) -> Result<Report, String> {
        let kind = self.gauge_options(
            ui,
            tool,
            "system",
            "管螺纹类型",
            &["R · 55° 密封锥螺纹", "NPT · 60° 密封锥螺纹"],
            "R · 55° 密封锥螺纹",
        );
        let npt = kind.starts_with("NPT");
        // 沿用 R/NPT 原有的独立规格键，切换类型时保留各自选择。
        let code = self.gauge_choice(
            ui,
            tool,
            if npt { "spec.2" } else { "spec.0" },
            "尺寸代号",
            &pipe::sizes(npt),
            "1/2",
        );
        pipe::calculate(npt, &code)
    }
    fn gauge_rebar_form(&mut self, ui: &mut egui::Ui, tool: &ToolDef) -> Result<Report, String> {
        let angle = self.gauge_options(ui, tool, "angle", "牙型角 (°)", &["75", "60", "70"], "75");
        let table = rows(&database()["REBAR"][&angle]);
        let specs: Vec<_> = table
            .iter()
            .map(|r| {
                format!(
                    "{} · {}",
                    r[0].as_str().unwrap_or(""),
                    r[1].as_str().unwrap_or("")
                )
            })
            .collect();
        let spec = self.gauge_choice(
            ui,
            tool,
            &format!("spec.{angle}"),
            "钢筋规格 / 螺纹代号",
            &specs,
            specs.first().map(String::as_str).unwrap_or(""),
        );
        let lines = self
            .gauge_options(ui, tool, "lines", "线数 / 头数", &["1", "2", "3", "4"], "1")
            .parse()
            .unwrap_or(1);
        thread::rebar(
            angle.parse().unwrap_or(75),
            specs.iter().position(|s| *s == spec).unwrap_or(0),
            lines,
        )
    }
    fn gauge_smooth_form(&mut self, ui: &mut egui::Ui, tool: &ToolDef) -> Result<Report, String> {
        let kind = self.gauge_options(
            ui,
            tool,
            "mode",
            "量规类型",
            &["孔用塞规", "轴用环规", "轴用卡规"],
            "孔用塞规",
        );
        let mode = if kind == "孔用塞规" {
            0
        } else if kind == "轴用卡规" {
            2
        } else {
            1
        };
        self.gauge_edit(ui, tool, "diameter", "基本尺寸 D (mm)", "20", &[]);
        let letter = self.gauge_choice(
            ui,
            tool,
            if mode == 0 {
                "hole_letter"
            } else {
                "shaft_letter"
            },
            "公差带字母",
            &smooth::letters(mode == 0),
            if mode == 0 { "H" } else { "h" },
        );
        let grade = self.gauge_options(
            ui,
            tool,
            "it",
            "公差等级",
            &[
                "IT6", "IT7", "IT8", "IT9", "IT10", "IT11", "IT12", "IT13", "IT14", "IT15", "IT16",
            ],
            "IT7",
        );
        smooth::calculate(
            self.number(tool, "diameter", "20")?,
            mode,
            &letter,
            grade.trim_start_matches("IT").parse().unwrap_or(7),
        )
    }
}

fn render_report_summary(
    ui: &mut egui::Ui,
    tool: &ToolDef,
    report: &Report,
    inputs: impl FnOnce() -> String,
) {
    let p = crate::theme::palette(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&report.title).size(15.5).strong().color(p.ink));
        if ui
            .add(egui::Button::new("复制完整结果").min_size(Vec2::new(116., 32.)))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            ui.ctx().copy_text(report.text(
                tool.id,
                &inputs(),
                crate::calculators::explanations::for_tool(tool.kind),
                &super::conventions::text(tool.kind),
            ));
            crate::theme::toast_success(ui.ctx(), "已复制完整量规计算报告");
        }
    });
    if !report.notes.is_empty() {
        ui.add_space(6.);
        collapsible_note(
            ui,
            (tool.id, "result_notes"),
            &format!("⚠ 取值说明与适用限制 · {} 项", report.notes.len()),
            |ui| {
                for note in &report.notes {
                    ui.add(egui::Label::new(RichText::new(note).size(12.).color(p.ink)).wrap());
                }
            },
        );
        ui.add_space(4.);
    }
    let summary: Vec<_> = report
        .metrics
        .iter()
        .take(6)
        .map(|(k, v)| (k.as_str(), v.display.clone()))
        .collect();
    render_metrics(
        ui,
        if matches!(tool.kind, ToolKind::Gauge(GaugeKind::Unified)) {
            "工件尺寸与公差"
        } else {
            "基本尺寸与参数"
        },
        &summary,
    );
    if report.metrics.len() > 6 {
        ui.add_space(4.);
        egui::CollapsingHeader::new(format!(
            "几何与公差取值详情 · {} 项",
            report.metrics.len() - 6
        ))
        .id_salt((tool.id, "factors"))
        .show(ui, |ui| {
            ScrollArea::horizontal()
                .id_salt((tool.id, "factor_grid_scroll"))
                .max_width(ui.available_width())
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new((tool.id, "factor_grid"))
                        .num_columns(2)
                        .striped(true)
                        .spacing([18., 8.])
                        .show(ui, |ui| {
                            for (name, value) in report.metrics.iter().skip(6) {
                                ui.label(RichText::new(name).color(p.muted));
                                ui.label(
                                    RichText::new(&value.display)
                                        .font(egui::FontId::monospace(12.5))
                                        .color(p.ink),
                                );
                                ui.end_row();
                            }
                        });
                });
        });
    }
}

fn render_report_tables(ui: &mut egui::Ui, tool: &ToolDef, report: &Report) {
    let p = crate::theme::palette(ui.ctx());
    let available = ui.available_width();
    for (index, table) in report.tables.iter().enumerate() {
        if index > 0 {
            ui.add_space(14.);
        }
        // 按表头和实际内容确定列宽；列结构相同的工作规、校对规共用宽度，保持上下对齐。
        let text_width = |text: RichText| {
            egui::WidgetText::from(text)
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Body,
                )
                .size()
                .x
                .ceil()
        };
        let widths: Vec<f32> = table
            .headings
            .iter()
            .enumerate()
            .map(|(column, heading)| {
                let heading_width = text_width(RichText::new(heading).size(12.).strong());
                report
                    .tables
                    .iter()
                    .filter(|other| other.headings == table.headings)
                    .flat_map(|other| &other.rows)
                    .filter_map(|row| row.get(column))
                    .map(|cell| {
                        text_width(
                            RichText::new(&cell.display)
                                .font(egui::FontId::monospace(12.5)),
                        )
                    })
                    .fold(heading_width.max(48.), f32::max)
                    + 6.
            })
            .collect();
        let total_w: f32 = widths.iter().sum::<f32>()
            + 16.0 * widths.len().saturating_sub(1) as f32;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(&table.title)
                    .size(14.)
                    .strong()
                    .color(p.accent),
            );
            if total_w > available {
                ui.label(
                    RichText::new("← 左右滑动查看全部列 →")
                        .size(11.)
                        .color(p.muted),
                );
            }
        });
        ui.add_space(4.);
        ScrollArea::horizontal()
            .id_salt((tool.id, index, "table_scroll"))
            .max_width(available)
            .auto_shrink([false, true])
            .scroll_bar_visibility(if total_w > available {
                egui::scroll_area::ScrollBarVisibility::AlwaysVisible
            } else {
                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
            })
            .show(ui, |ui| {
                egui::Grid::new((tool.id, index, "table"))
                    .num_columns(table.headings.len())
                    .striped(true)
                    .min_row_height(34.)
                    .spacing([16., 8.])
                    .show(ui, |ui| {
                        for (title, width) in table.headings.iter().zip(&widths) {
                            ui.add_sized(
                                [*width, 34.],
                                egui::Label::new(
                                    RichText::new(title).size(12.).strong().color(p.muted),
                                )
                                .extend()
                                .halign(egui::Align::Center),
                            );
                        }
                        ui.end_row();
                        for row in &table.rows {
                            for (i, cell) in row.iter().enumerate() {
                                ui.add_sized(
                                    [widths[i], 34.],
                                    egui::Label::new(if i == 0 {
                                        RichText::new(&cell.display).size(12.5).color(p.ink)
                                    } else {
                                        RichText::new(&cell.display)
                                            .font(egui::FontId::monospace(12.5))
                                            .color(p.accent)
                                    })
                                    .extend()
                                    .halign(egui::Align::Center),
                                );
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独立计算说明中可直接解析的数值四则运算，检查等号两边一致。
    fn check_numeric_equations(report: &Report, checked: &mut usize) {
        use std::sync::OnceLock;
        static EQUATION: OnceLock<regex::Regex> = OnceLock::new();
        static TOKEN: OnceLock<regex::Regex> = OnceLock::new();
        static RANGE: OnceLock<regex::Regex> = OnceLock::new();
        let number = r"-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?";
        let equations = EQUATION.get_or_init(|| {
            regex::Regex::new(&format!(
                r"(?P<expr>{number}(?:\s*(?:\+|−|×|/)\s*{number})+)\s*=\s*(?P<result>{number})"
            ))
            .unwrap()
        });
        let token = TOKEN.get_or_init(|| regex::Regex::new(&format!(r"{number}|[+−×/]")).unwrap());
        let ranges = RANGE.get_or_init(|| regex::Regex::new(&format!(r"(?P<center>{number}(?:\s*(?:\+|−|×|/)\s*{number})*)\s*±\s*(?P<half>{number})(?:\s*/\s*(?P<div>{number}))?\s*=\s*(?P<low>{number})～(?P<high>{number})")).unwrap());
        let evaluate = |expression: &str| {
            let parts: Vec<_> = token.find_iter(expression).map(|m| m.as_str()).collect();
            let mut sum = 0.0;
            let mut term = parts[0].parse::<f64>().unwrap();
            let mut scale = term.abs().max(1.0);
            for pair in parts[1..].as_chunks::<2>().0 {
                let value = pair[1].parse::<f64>().unwrap();
                scale = scale.max(value.abs());
                match pair[0] {
                    "+" => {
                        sum += term;
                        term = value;
                    }
                    "−" => {
                        sum += term;
                        term = -value;
                    }
                    "×" => term *= value,
                    "/" => term /= value,
                    _ => unreachable!(),
                }
            }
            (sum + term, scale)
        };
        let metric = |label: &str| {
            report
                .metrics
                .iter()
                .find(|(name, _)| name == label)
                .and_then(|(_, value)| value.raw.parse::<f64>().ok())
        };
        if let (Some(p), Some(d2), Some(angle), Some(lines), Some(lead), Some(slope), Some(wire)) = (
            metric("螺距 P (mm)"),
            metric("基本中径 D₂ (mm)"),
            metric("牙型角 (°)"),
            metric("线数 / 头数"),
            metric("导程 L (mm)"),
            metric("螺旋升角 ψ (°)"),
            metric("最佳三针直径 d₀ (mm)"),
        ) {
            let calculated_lead = lines * p;
            let calculated_slope = (calculated_lead / (std::f64::consts::PI * d2))
                .atan()
                .to_degrees();
            let calculated_wire = p / (2.0 * (angle / 2.0).to_radians().cos());
            for (label, expected, actual) in [
                ("导程", calculated_lead, lead),
                ("螺旋升角", calculated_slope, slope),
                ("最佳三针直径", calculated_wire, wire),
            ] {
                assert!(
                    (expected - actual).abs() <= 1e-12 * expected.abs().max(1.0),
                    "{} 的{label}计算不符",
                    report.title
                );
                *checked += 1;
            }
        }
        for full in [true, false] {
            for line in crate::calculators::common::process_lines_for_test(&report.process, full) {
                for captures in equations.captures_iter(&line) {
                    let whole = captures.name("expr").unwrap();
                    if line[..whole.start()]
                        .trim_end()
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || ".^×/−±".contains(c))
                    {
                        continue;
                    }
                    let (expected, scale) = evaluate(whole.as_str());
                    let actual = captures
                        .name("result")
                        .unwrap()
                        .as_str()
                        .parse::<f64>()
                        .unwrap();
                    let tolerance = if full { 1e-12 } else { 2e-11 } * scale.max(actual.abs());
                    assert!(
                        (expected - actual).abs() <= tolerance,
                        "{}：完整精度={full}，算式 {} = {}，独立计算为 {expected}；原文：{line}",
                        report.title,
                        whole.as_str(),
                        actual
                    );
                    *checked += 1;
                }
                for captures in ranges.captures_iter(&line) {
                    let start = captures.name("center").unwrap().start();
                    if line[..start]
                        .trim_end()
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || ".^×/−+±".contains(c))
                    {
                        continue;
                    }
                    let (center, scale) = evaluate(&captures["center"]);
                    let half = captures["half"].parse::<f64>().unwrap()
                        / captures
                            .name("div")
                            .map_or(1.0, |m| m.as_str().parse::<f64>().unwrap());
                    let low = captures["low"].parse::<f64>().unwrap();
                    let high = captures["high"].parse::<f64>().unwrap();
                    let tolerance = if full { 1e-12 } else { 2e-11 }
                        * scale.max(center.abs()).max(half.abs()).max(1.0);
                    assert!(
                        (center - half - low).abs() <= tolerance
                            && (center + half - high).abs() <= tolerance,
                        "{}：完整精度={full}，上下限代入不符：{line}",
                        report.title
                    );
                    *checked += 2;
                }
            }
        }
    }

    fn check_calculated_results_are_explained(report: &Report) {
        let text = report.process.join("\n").replace('\u{2063}', "");
        for table in &report.tables {
            for row in &table.rows {
                for (heading, cell) in table.headings.iter().zip(row) {
                    for token in cell.raw.split_whitespace() {
                        if token.parse::<f64>().is_ok() {
                            assert!(
                                text.contains(token),
                                "{} / {} / {} 的 {} 未在计算过程中说明",
                                report.title,
                                table.title,
                                heading,
                                token
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn numeric_substitutions_match_across_gauge_modes_and_boundaries() {
        let mut checked = 0;
        let mut reports = 0;
        for trap in [false, true] {
            for diameter in [3.0, 10.0, 20.0, 24.0, 60.0, 100.0, 120.0] {
                for pitch in [0.5, 1.5, 2.5, 5.0, 12.0, 13.0] {
                    for grade in thread::grades(trap, diameter, pitch) {
                        for lines in if trap { &[1, 2, 5][..] } else { &[1][..] } {
                            if let Ok(report) =
                                thread::calculate(trap, diameter, pitch, &grade, *lines)
                            {
                                check_numeric_equations(&report, &mut checked);
                                check_calculated_results_are_explained(&report);
                                reports += 1;
                            }
                        }
                    }
                }
            }
        }
        for (size, _, tpi, _) in imperial::un_series().into_iter().step_by(7) {
            for class in ["1A", "2A", "3A", "1B", "2B", "3B"] {
                if let Ok(report) = unified::calculate(&unified::Input {
                    size: &size,
                    tpi,
                    class,
                }) {
                    check_numeric_equations(&report, &mut checked);
                    check_calculated_results_are_explained(&report);
                    reports += 1;
                }
            }
        }
        for row in rows(&super::super::un_data::data()["manufacturing"]) {
            let low = n(row, "diameter_lower_in");
            let high = n(row, "diameter_upper_in");
            let tpi = n(row, "tpi");
            for diameter in [(low + high) / 2.0, high] {
                for class in ["2A", "2B"] {
                    if let Ok(report) = unified::calculate(&unified::Input {
                        size: &diameter.to_string(),
                        tpi,
                        class,
                    }) {
                        check_numeric_equations(&report, &mut checked);
                        check_calculated_results_are_explained(&report);
                        reports += 1;
                    }
                }
            }
        }
        for angle in [60, 70, 75] {
            for index in 0..rows(&database()["REBAR"][angle.to_string()]).len() {
                for lines in [1, 4] {
                    if let Ok(report) = thread::rebar(angle, index, lines) {
                        check_numeric_equations(&report, &mut checked);
                        check_calculated_results_are_explained(&report);
                        reports += 1;
                    }
                }
            }
        }
        for mode in 0..=2 {
            for diameter in [0.5, 1.0, 1.000001, 3.0, 3.000001, 10.0, 20.0, 100.0, 500.0] {
                for letter in smooth::letters(mode == 0) {
                    for grade in [6, 7, 8, 16] {
                        if let Ok(report) = smooth::calculate(diameter, mode, &letter, grade) {
                            check_numeric_equations(&report, &mut checked);
                            check_calculated_results_are_explained(&report);
                            reports += 1;
                        }
                    }
                }
            }
        }
        for npt in [false, true] {
            for size in pipe::sizes(npt) {
                if let Ok(report) = pipe::calculate(npt, &size) {
                    check_numeric_equations(&report, &mut checked);
                    reports += 1;
                }
            }
        }
        assert!(reports > 1000, "仅检查 {reports} 个有效组合");
        assert!(checked > 10000, "仅检查 {checked} 个数值算式");
        println!("核查了 {reports} 个有效量规组合和 {checked} 条数值代入等式（含两种显示精度）");
    }

    #[test]
    fn process_uses_actual_geometry_and_lookup_branches() {
        let mut reports = vec![
            thread::calculate(false, 20.0, 2.5, "6H", 1).unwrap(),
            thread::calculate(false, 20.0, 2.5, "6g", 1).unwrap(),
            thread::calculate(true, 24.0, 5.0, "8H", 2).unwrap(),
            thread::calculate(true, 24.0, 5.0, "8e", 2).unwrap(),
            unified::calculate(&unified::Input {
                size: "1/2",
                tpi: 20.0,
                class: "2A",
            })
            .unwrap(),
            unified::calculate(&unified::Input {
                size: "1/2",
                tpi: 20.0,
                class: "2B",
            })
            .unwrap(),
            pipe::calculate(false, "1/2").unwrap(),
            pipe::calculate(true, "1/2").unwrap(),
            thread::rebar(75, 0, 1).unwrap(),
            thread::rebar(60, 0, 2).unwrap(),
        ];
        for mode in 0..=2 {
            reports
                .push(smooth::calculate(20.0, mode, if mode == 0 { "H" } else { "h" }, 7).unwrap());
        }
        for report in reports {
            let text = report.process.join("\n");
            assert!(!text.is_empty(), "{}", report.title);
            assert!(
                text.contains("查表") || text.contains("内置表") || text.contains("查询内置"),
                "{}: {text}",
                report.title
            );
            for (label, value) in &report.metrics {
                if label == "基本中径 D₂ (mm)" || label == "工件尺寸下限 (mm)" {
                    assert!(
                        text.contains(&value.raw),
                        "{}: {label}={} {text}",
                        report.title,
                        value.raw
                    );
                }
            }
        }
    }

    #[test]
    fn ring_process_matches_actual_stop_center_for_metric_and_trapezoidal() {
        for (trap, diameter, pitch, grade, lines) in [
            (false, 20.0, 2.5, "6g", 1),
            (true, 24.0, 5.0, "8e", 1),
            (true, 24.0, 5.0, "8e", 3),
        ] {
            let report = thread::calculate(trap, diameter, pitch, grade, lines).unwrap();
            let text = report.process.join("\n");
            assert!(text.contains("止端中心 = D₂ + es − Td − TR/2"), "{text}");
            assert!(!text.contains("− Td/2 − TR/2"), "{text}");
            let actual_center = &report.tables[0].rows[1][3].raw;
            assert!(
                text.contains(&format!("\u{2063}{actual_center}\u{2063} mm")),
                "{text}"
            );
        }
    }

    #[test]
    fn unified_process_converts_inch_tolerance_for_every_grade() {
        for class in ["1A", "2A", "3A", "1B", "2B", "3B"] {
            let report = unified::calculate(&unified::Input {
                size: "1/2",
                tpi: 20.0,
                class,
            })
            .unwrap();
            let text = report.process.join("\n");
            assert!(text.contains("P 均以 inch 代入"), "{class}: {text}");
            assert!(text.contains("乘以 25.4 后为"), "{class}: {text}");
            let tolerance = &report
                .metrics
                .iter()
                .find(|(name, _)| name == "2A 基础中径公差 (mm)")
                .unwrap()
                .1
                .raw;
            assert!(
                text.contains(&format!("\u{2063}{tolerance}\u{2063} mm")),
                "{class}: {text}"
            );
        }
    }

    #[test]
    fn detailed_gauge_steps_cover_the_values_in_each_calculated_row() {
        let mut reports = vec![
            thread::calculate(false, 20.0, 2.5, "6H", 1).unwrap(),
            thread::calculate(false, 20.0, 2.5, "6g", 1).unwrap(),
            thread::calculate(true, 24.0, 5.0, "8H", 2).unwrap(),
            thread::calculate(true, 24.0, 5.0, "8e", 2).unwrap(),
            thread::rebar(75, 0, 1).unwrap(),
            thread::rebar(60, 0, 2).unwrap(),
        ];
        for class in ["1A", "2A", "3A", "1B", "2B", "3B"] {
            reports.push(
                unified::calculate(&unified::Input {
                    size: "1/2",
                    tpi: 20.0,
                    class,
                })
                .unwrap(),
            );
        }
        for mode in 0..=2 {
            reports
                .push(smooth::calculate(20.0, mode, if mode == 0 { "H" } else { "h" }, 7).unwrap());
        }
        for report in reports {
            let text = report.process.join("\n").replace('\u{2063}', "");
            for table in &report.tables {
                for row in &table.rows {
                    for (heading, cell) in table.headings.iter().zip(row) {
                        for token in cell.raw.split_whitespace() {
                            if token.parse::<f64>().is_ok() {
                                assert!(
                                    text.contains(token),
                                    "{} / {} / {} 的结果 {} 没有对应计算步骤：{text}",
                                    report.title,
                                    table.title,
                                    heading,
                                    token
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn metric_plug_and_minor_reference_show_substitution_instead_of_only_results() {
        let report = thread::calculate(false, 20.0, 2.5, "6H", 1).unwrap();
        let text = report.process.join("\n").replace('\u{2063}', "");
        assert!(text.contains("中径名义 = 工件中径下限 + ZPL ="));
        assert!(text.contains("中径下限 = 名义 − TPL/2 ="));
        assert!(text.contains("大径下限 = 大径基准 + ZPL − TPL ="));
        assert!(text.contains("磨损极限 = 通端中径下限 − 通端磨损储备 ="));
        assert!(text.contains("小径参考界限（非独立制造公差）"));
        assert!(text.contains("参考上界 ="));
        assert!(text.contains("止端中径名义按本模型"));
        assert!(!text.contains("以下为本次计算或查表得到的结果"));
        let d2 = 20.0 - 3.0 * 3.0_f64.sqrt() / 8.0 * 2.5;
        let factors = factors(false, 224.0, false).unwrap();
        let center = d2 + factors.zpl;
        let lower = center - factors.tpl / 2.0;
        let upper = center + factors.tpl / 2.0;
        let table = &report.tables[0];
        assert_eq!(table.rows[0][1].raw.parse::<f64>().unwrap(), lower);
        assert_eq!(table.rows[0][2].raw.parse::<f64>().unwrap(), upper);
        assert_eq!(table.rows[0][3].raw.parse::<f64>().unwrap(), center);
        assert_eq!(
            table.rows[0][7].raw.parse::<f64>().unwrap(),
            lower - factors.sgo
        );
        assert!(text.contains(&format!("{} + {} = {} mm", d2, factors.zpl, center)));
        let d1 = 20.0 - 5.0 * 3.0_f64.sqrt() / 8.0 * 2.5;
        assert_eq!(report.tables[1].rows[0][1].raw.parse::<f64>().unwrap(), d1);
        assert_eq!(
            report.tables[1].rows[0][2].raw.parse::<f64>().unwrap(),
            d1 + 450.0 / 1000.0
        );
    }

    #[test]
    fn unavailable_minor_limits_are_not_presented_as_computed_values() {
        let report = thread::calculate(true, 24.0, 5.0, "8e", 2).unwrap();
        let table = report
            .tables
            .iter()
            .find(|table| table.title.contains("小径参考界限"))
            .unwrap();
        assert_eq!(table.rows[0][1].raw, "—");
        assert_eq!(table.rows[0][2].raw, "—");
        let text = report.process.join("\n");
        assert!(text.contains("该组合未收录公差，参考上下界不计算"));
    }

    #[test]
    fn questionable_npt_six_inch_value_remains_unavailable() {
        let report = pipe::calculate(true, "6").unwrap();
        let ring = report
            .tables
            .iter()
            .find(|table| table.title.contains("NPT 工作环规"))
            .unwrap();
        let row = ring
            .rows
            .iter()
            .find(|row| row[0].raw == "基面中径 D₂")
            .unwrap();
        assert_eq!(row[1].raw, "—");
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("NPT 6 环规中径原表值存在疑点"))
        );
    }

    #[test]
    fn result_notes_are_collapsed_but_remain_visibly_flagged() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let tool = crate::catalog::TOOLS
            .iter()
            .find(|tool| tool.id == "gauge_metric")
            .unwrap();
        let mut report = Report::new("测试量规");
        report.note("采用最近标准螺距，仅作参考。");
        let render = |events, time| {
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
                    egui::CentralPanel::default().show(ctx, |ui| {
                        render_report_summary(ui, tool, &report, || "{}".into());
                        render_report_tables(ui, tool, &report);
                    });
                },
            )
        };
        let output = render(vec![], 0.0);
        let header = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "⚠ 取值说明与适用限制 · 1 项" => {
                    Some(text.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .expect("结果说明标题应提示取值限制");
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.text() == "采用最近标准螺距，仅作参考。"
        )));
        for (pressed, time) in [(true, 0.1), (false, 0.2)] {
            render(
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
        assert!(
            render(vec![], 0.5)
                .shapes
                .iter()
                .any(|shape| matches!(&shape.shape,
                    egui::Shape::Text(text) if text.galley.text() == "采用最近标准螺距，仅作参考。"
                )),
            "展开后应显示完整的取值限制"
        );
    }
}
