//! 自适应参数区、可折叠取值详情和横向结果表。表格不设内部纵向滚动，避免阻断整页滚轮。
use super::{imperial, lookup::*, pipe, report::*, smooth, specs, thread, unified};
use crate::{
    BLUE, BLUE_SOFT, INK, MUTED,
    app::ToolboxApp,
    calculators::common::*,
    catalog::{GaugeKind, ToolDef, ToolKind},
};
use eframe::egui::{self, RichText, ScrollArea, TextEdit, Vec2};

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
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    ui.spacing_mut().interact_size.y = 32.;
    ui.spacing_mut().item_spacing.y = 3.;
    ui.spacing_mut().button_padding = egui::vec2(10., 7.);
}

impl ToolboxApp {
    pub(crate) fn render_gauge(&mut self, ui: &mut egui::Ui, tool: &ToolDef, kind: GaugeKind) {
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(20, 6))
            .show(ui, |ui| {
                let response = egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .corner_radius(8.)
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        egui::Color32::from_rgb(220, 229, 241),
                    ))
                    .inner_margin(16.)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("计算参数").size(15.).strong().color(INK));
                        ui.add_space(4.);
                        match kind {
                            GaugeKind::Metric | GaugeKind::Trapezoidal => {
                                self.gauge_thread_form(ui, tool, kind == GaugeKind::Trapezoidal)
                            }
                            GaugeKind::Unified => self.gauge_unified_form(ui, tool),
                            GaugeKind::Pipe => self.gauge_pipe_form(ui, tool),
                            GaugeKind::Rebar => self.gauge_rebar_form(ui, tool),
                            GaugeKind::Smooth => self.gauge_smooth_form(ui, tool),
                        }
                    });
                match response.inner {
                    Ok(report) => render_report(ui, tool, &report),
                    Err(message) => render_error_message(ui, &message),
                }
                info_note(ui, tool.kind);
            });
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
                            let captions: Vec<_> = choices
                                .iter()
                                .map(|item| {
                                    // UN 的编号和分数规格附实际直径，选择时只写入原规格，保持解析与系列联动。
                                    let caption = if key == "size"
                                        && matches!(tool.kind, ToolKind::Gauge(GaugeKind::Unified))
                                    {
                                        imperial::un_diameter(item)
                                            .map(|inch| {
                                                format!(
                                                    "{item}（{} inch / {} mm）",
                                                    format_number(inch),
                                                    format_number(inch * imperial::INCH),
                                                )
                                            })
                                            .unwrap_or_else(|_| item.clone())
                                    } else {
                                        item.clone()
                                    };
                                    caption
                                })
                                .collect();
                            size_gauge_menu(ui, &captions);
                            for (item, caption) in choices.iter().zip(captions) {
                                ui.selectable_value(
                                    &mut value,
                                    item.clone(),
                                    RichText::new(caption).size(14.),
                                );
                            }
                        });
                }
            });
        });
        self.set_value(tool, key, value);
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
        let narrow = ui.available_width() < 560.;
        let draw_combo = |ui: &mut egui::Ui, value: &mut String| {
            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 32.;
                egui::ComboBox::from_id_salt((tool.id, key))
                    .width(ui.available_width().min(270.))
                    .height(320.)
                    .selected_text(if value.is_empty() {
                        "当前参数无可用选项"
                    } else {
                        value.as_str()
                    })
                    .show_ui(ui, |ui| {
                        size_gauge_menu(ui, options);
                        for item in options {
                            ui.selectable_value(value, item.clone(), RichText::new(item).size(14.));
                        }
                    });
            });
        };
        ui.push_id((tool.id, key, "choice_row"), |ui| {
            if narrow {
                ui.label(RichText::new(label).size(12.).color(MUTED));
                draw_combo(ui, &mut value);
            } else {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [180., 34.],
                        egui::Label::new(RichText::new(label).color(MUTED)),
                    );
                    draw_combo(ui, &mut value);
                });
            }
        });
        // 输入清空、负号等编辑中间态没有可选公差带时，不抹掉此前选择。
        if !options.is_empty() {
            self.set_value(tool, key, value.clone());
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
        let previous_series = parse_number(&previous_tpi)
            .ok()
            .and_then(|tpi| imperial::un_series_class(&previous_size, tpi));
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
                .find(|(series, _)| Some(series) == previous_series.as_ref())
                .or_else(|| variants.first())
                .map(|(_, tpi)| *tpi);
            if let Some(tpi) = default_tpi {
                self.set_value(tool, "tpi", tpi.to_string());
            }
        }
        let mut tpis = variants.iter().map(|(_, tpi)| *tpi).collect::<Vec<_>>();
        tpis.sort_by(|a, b| b.total_cmp(a));
        tpis.dedup_by(|a, b| same(*a, *b));
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
        self.set_value(tool, "series", detected.into());
        let selected = self.gauge_choice(ui, tool, "series", "螺纹系列", &series_options, detected);
        if selected != detected {
            if let Some((_, tpi)) = variants.iter().find(|(series, _)| *series == selected) {
                self.set_value(tool, "tpi", tpi.to_string());
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

fn render_report(ui: &mut egui::Ui, tool: &ToolDef, report: &Report) {
    ui.add_space(12.);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&report.title).size(16.).strong().color(INK));
        if ui
            .add(egui::Button::new("复制完整结果").min_size(Vec2::new(116., 34.)))
            .clicked()
        {
            ui.ctx().copy_text(report.text(
                crate::calculators::explanations::for_tool(tool.kind),
                &super::conventions::text(tool.kind),
            ));
        }
    });
    for note in &report.notes {
        egui::Frame::new()
            .fill(BLUE_SOFT)
            .corner_radius(5.)
            .inner_margin(10.)
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(note).size(12.).color(INK)).wrap());
            });
        ui.add_space(3.);
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
        egui::CollapsingHeader::new(format!(
            "几何与公差取值详情 · {} 项",
            report.metrics.len() - 6
        ))
        .id_salt((tool.id, "factors"))
        .show(ui, |ui| {
            egui::Grid::new((tool.id, "factor_grid"))
                .num_columns(2)
                .striped(true)
                .spacing([18., 9.])
                .show(ui, |ui| {
                    for (name, value) in report.metrics.iter().skip(6) {
                        ui.label(RichText::new(name).color(MUTED));
                        ui.label(&value.display);
                        ui.end_row();
                    }
                });
        });
    }
    for (index, table) in report.tables.iter().enumerate() {
        ui.add_space(14.);
        ui.label(RichText::new(&table.title).size(14.).strong().color(BLUE));
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
                    .map(|cell| text_width(RichText::new(&cell.display).size(12.5)))
                    .fold(heading_width.max(48.), f32::max)
                    + 2.
            })
            .collect();
        ScrollArea::horizontal()
            .id_salt((tool.id, index, "table_scroll"))
            .auto_shrink([false, true])
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
                                    RichText::new(title).size(12.).strong().color(MUTED),
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
                                    egui::Label::new(
                                        RichText::new(&cell.display).size(12.5).color(if i == 0 {
                                            INK
                                        } else {
                                            BLUE
                                        }),
                                    )
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
