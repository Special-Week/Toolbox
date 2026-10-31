//! 应用状态与页面调度；输入以“工具 ID + 字段名”隔离，避免切换工具后串值。
//! 复杂计算自动续接本地输入；快捷换算每次打开使用示例。
use crate::calculators::common::*;
use crate::calculators::spline::ExportJob;
use crate::catalog::{Dimension, Group, TOOLS, ToolDef, ToolKind, units_for};
use crate::navigation;
use crate::{BLUE, BLUE_SOFT, INK, MUTED, PAGE};
use eframe::egui::{self, Align, Color32, Layout, RichText, ScrollArea, TextEdit, Vec2};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;
mod dashboard;
mod input_history;
use input_history::InputHistory;

pub(crate) struct ToolboxApp {
    selected: Option<&'static str>,
    selected_group: Option<Group>,
    search: String,
    // 保留用户输入的原始文本，允许编辑中的空串、负号等暂时无效状态。
    values: HashMap<String, String>,
    history: InputHistory,
    opened: Option<&'static str>,
    example_revision: u64,
    sidebar_collapsed: bool,
    collapsed_groups: HashSet<Group>,
    spline_export: Option<ExportJob>,
    spline_export_message: Option<String>,
    // 页眉纹理由 egui 持有，保持高分辨率源图以适应窗口 DPI 缩放。
    brand_icon: Option<egui::TextureHandle>,
}

impl Default for ToolboxApp {
    fn default() -> Self {
        let history = InputHistory::new();
        Self {
            selected: None,
            selected_group: None,
            search: String::new(),
            values: history.values.clone(),
            history,
            opened: None,
            example_revision: 0,
            sidebar_collapsed: false,
            collapsed_groups: Group::ALL.into_iter().collect(),
            spline_export: None,
            spline_export_message: None,
            brand_icon: None,
        }
    }
}

impl eframe::App for ToolboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_spline_export(ctx);
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(22, 10)),
            )
            .show(ctx, |ui| self.render_header(ui));

        if ctx.screen_rect().width() >= 880.0 {
            egui::SidePanel::left("navigation")
                .exact_width(if self.sidebar_collapsed { 92.0 } else { 245.0 })
                .resizable(false)
                .frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(
                    egui::Margin::symmetric(if self.sidebar_collapsed { 8 } else { 14 }, 13),
                ))
                .show(ctx, |ui| {
                    if self.sidebar_collapsed {
                        self.render_collapsed_sidebar(ui);
                    } else {
                        self.render_sidebar(ui);
                    }
                });
        } else {
            egui::TopBottomPanel::top("compact_navigation")
                .frame(
                    egui::Frame::new()
                        .fill(Color32::WHITE)
                        .inner_margin(egui::Margin::symmetric(10, 5)),
                )
                .show(ctx, |ui| self.render_compact_navigation(ui));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(PAGE)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ctx, |ui| self.render_main(ui));
        if let Some(remaining) = self.history.remaining() {
            if remaining.is_zero() {
                self.history.flush();
            }
            if let Some(remaining) = self.history.remaining() {
                ctx.request_repaint_after(remaining);
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.history.flush();
    }
}

impl ToolboxApp {
    fn poll_spline_export(&mut self, ctx: &egui::Context) {
        // UI 线程只轮询消息，文件写入交给后台线程，避免大规模采样冻结窗口。
        let Some(job) = self.spline_export.as_ref() else {
            return;
        };
        match job.receiver.try_recv() {
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(80));
            }
            Err(TryRecvError::Disconnected) => {
                self.spline_export = None;
                self.spline_export_message = Some("CSV 导出线程异常退出".to_owned());
            }
            Ok(result) => {
                let cancelled = job.cancelled.load(Ordering::Relaxed);
                self.spline_export = None;
                self.spline_export_message = Some(match result {
                    Ok(path) => format!("CSV 已导出：{}", path.display()),
                    Err(_) if cancelled => "CSV 导出已取消".to_owned(),
                    Err(error) => format!("CSV 导出失败：{error}"),
                });
            }
        }
    }

    pub(crate) fn start_spline_export(&mut self, job: ExportJob) {
        self.spline_export_message = None;
        self.spline_export = Some(job);
    }

    pub(crate) fn spline_export_progress(&self) -> Option<(usize, usize, bool)> {
        self.spline_export.as_ref().map(|job| {
            (
                job.progress.load(Ordering::Relaxed),
                job.total,
                job.cancelled.load(Ordering::Relaxed),
            )
        })
    }

    pub(crate) fn cancel_spline_export(&self) {
        if let Some(job) = &self.spline_export {
            job.cancelled.store(true, Ordering::Relaxed);
        }
    }

    pub(crate) fn spline_export_message(&self) -> Option<&str> {
        self.spline_export_message.as_deref()
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        let icon = self.brand_icon.get_or_insert_with(|| {
            let data = crate::icon::data();
            ui.ctx().load_texture(
                "toolbox-brand",
                egui::ColorImage::from_rgba_unmultiplied(
                    [data.width as usize, data.height as usize],
                    &data.rgba,
                ),
                egui::TextureOptions::LINEAR,
            )
        });
        ui.horizontal(|ui| {
            ui.image((icon.id(), Vec2::splat(42.0)));
            ui.label(RichText::new("工具箱").size(17.0).strong().color(INK));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} 项工具", TOOLS.len()))
                        .size(12.0)
                        .color(MUTED),
                );
            });
        });
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("工具导航").size(14.0).strong().color(INK));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_sized([34.0, 34.0], egui::Button::new("«").frame(false))
                    .on_hover_text("折叠工具导航")
                    .clicked()
                {
                    self.sidebar_collapsed = true;
                }
            });
        });
        ui.add_space(6.0);
        ui.add(
            TextEdit::singleline(&mut self.search)
                .hint_text("搜索工具")
                .desired_width(ui.available_width()),
        );
        ui.add_space(10.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                for group in Group::ALL {
                    let tools: Vec<&ToolDef> = TOOLS
                        .iter()
                        .filter(|tool| tool.group == group && self.matches(tool))
                        .collect();
                    if tools.is_empty() {
                        continue;
                    }
                    ui.add_space(12.0);
                    let is_collapsed = self.collapsed_groups.contains(&group);
                    if navigation::category(
                        ui,
                        group.title(),
                        tools.len(),
                        is_collapsed,
                        self.selected_group == Some(group),
                    )
                    .on_hover_text("展开或折叠此分类")
                    .clicked()
                    {
                        if is_collapsed {
                            self.collapsed_groups.remove(&group);
                        } else {
                            self.collapsed_groups.insert(group);
                        }
                    }
                    if is_collapsed {
                        continue;
                    }
                    for tool in tools {
                        let selected = self.selected == Some(tool.id);
                        if navigation::tool(ui, tool.name, selected)
                            .on_hover_text(format!("{}\n{}", tool.name, tool.description))
                            .clicked()
                        {
                            self.selected = Some(tool.id);
                            self.selected_group = Some(group);
                        }
                    }
                }
            });
    }

    fn render_collapsed_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            if ui
                .add_sized([72.0, 42.0], egui::Button::new("»").frame(false))
                .on_hover_text("展开工具导航")
                .clicked()
            {
                self.sidebar_collapsed = false;
            }
            ui.add_space(10.0);
            for group in Group::ALL {
                if ui
                    .add_sized(
                        [72.0, 44.0],
                        egui::Button::new(RichText::new(group.short_title()).size(13.5))
                            .selected(self.selected_group == Some(group)),
                    )
                    .on_hover_text(group.title())
                    .clicked()
                {
                    self.selected_group = Some(group);
                    self.selected = None;
                }
            }
        });
    }

    fn render_compact_navigation(&mut self, ui: &mut egui::Ui) {
        ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.selected_group.is_none() && self.selected.is_none(),
                            "全部",
                        )
                        .clicked()
                    {
                        self.selected_group = None;
                        self.selected = None;
                    }
                    for group in Group::ALL {
                        if ui
                            .selectable_label(self.selected_group == Some(group), group.title())
                            .clicked()
                        {
                            self.selected_group = Some(group);
                            self.selected = None;
                        }
                    }
                });
            });
    }

    fn render_main(&mut self, ui: &mut egui::Ui) {
        if let Some(id) = self.selected {
            self.render_selected(ui, id);
        } else {
            self.opened = None;
            self.render_dashboard(ui);
        }
    }

    fn render_selected(&mut self, ui: &mut egui::Ui, id: &'static str) {
        let Some(tool) = TOOLS.iter().find(|tool| tool.id == id) else {
            return;
        };
        if self.opened != Some(id) {
            if !tool.remembers_inputs() {
                self.clear_tool_inputs(tool);
            }
            self.opened = Some(id);
        }
        let mut back = false;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(22.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(RichText::new("← 工具箱").color(BLUE)).clicked() {
                        back = true;
                    }
                    ui.label(
                        RichText::new(format!("/ {} / {}", tool.group.title(), tool.name))
                            .color(MUTED),
                    );
                    if tool.remembers_inputs() && self.history.error {
                        ui.label(RichText::new("输入暂未保存").size(11.0).color(MUTED))
                            .on_hover_text("本地保存暂时不可用，当前输入仍可使用，稍后自动重试");
                    }
                });
                ui.add_space(13.0);
                ui.horizontal(|ui| {
                    // 操作与标题同排，右侧留白；先预留按钮宽度，长标题可在左侧换行。
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(24.0);
                        if ui
                            .add_enabled(
                                self.spline_export.is_none(),
                                egui::Button::new(RichText::new("恢复示例").size(14.0))
                                    .min_size(Vec2::new(104.0, 40.0)),
                            )
                            .on_hover_text("将当前工具的全部输入恢复为示例值")
                            .clicked()
                        {
                            self.clear_tool_inputs(tool);
                            self.example_revision += 1;
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if !tool.icon.is_empty() {
                                ui.add_sized(
                                    [46.0, 46.0],
                                    egui::Button::new(
                                        RichText::new(tool.icon).size(18.0).strong().color(BLUE),
                                    )
                                    .fill(BLUE_SOFT),
                                );
                            }
                            ui.vertical(|ui| {
                                ui.label(RichText::new(tool.name).size(26.0).strong().color(INK));
                                ui.label(RichText::new(tool.description).size(12.5).color(MUTED));
                            });
                        });
                    });
                });
                ui.add_space(17.0);
                ui.push_id((tool.id, self.example_revision), |ui| match tool.kind {
                    ToolKind::Unit(dimension) => self.render_unit_page(ui, tool, dimension),
                    kind => self.render_calculator(ui, tool, kind),
                });
            });
        self.history.observe(tool, &self.values);
        if back {
            self.selected = None;
        }
    }

    fn clear_tool_inputs(&mut self, tool: &ToolDef) {
        let prefix = format!("{}.", tool.id);
        self.values.retain(|key, _| !key.starts_with(&prefix));
    }

    fn matches(&self, tool: &ToolDef) -> bool {
        let query = self.search.trim().to_lowercase();
        query.is_empty()
            || tool.name.to_lowercase().contains(&query)
            || tool.description.to_lowercase().contains(&query)
    }

    fn storage_key(tool: &ToolDef, key: &str) -> String {
        format!("{}.{}", tool.id, key)
    }

    pub(crate) fn value(&self, tool: &ToolDef, key: &str, default: &str) -> String {
        self.values
            .get(&Self::storage_key(tool, key))
            .cloned()
            .unwrap_or_else(|| default.to_owned())
    }

    pub(crate) fn number(&self, tool: &ToolDef, key: &str, default: &str) -> Result<f64, String> {
        parse_number(&self.value(tool, key, default))
    }
    pub(crate) fn set_value(&mut self, tool: &ToolDef, key: &str, value: String) {
        self.values.insert(Self::storage_key(tool, key), value);
    }

    pub(crate) fn input(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
        default: &str,
        hint: &str,
    ) {
        let storage_key = Self::storage_key(tool, key);
        let value = self
            .values
            .entry(storage_key)
            .or_insert_with(|| default.to_owned());
        input_row(ui, label, value, hint);
    }

    pub(crate) fn inline_input(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        default: &str,
        width: f32,
        hint: &str,
    ) -> bool {
        let storage_key = Self::storage_key(tool, key);
        let value = self
            .values
            .entry(storage_key)
            .or_insert_with(|| default.to_owned());
        ui.add_sized(
            [width.max(52.0), 36.0],
            TextEdit::singleline(value).hint_text(hint),
        )
        .changed()
    }

    /// 调整项目数量时保留已有字段值；重新增加项目可恢复本次运行中的输入。
    pub(crate) fn dynamic_count(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
        default: usize,
        min: usize,
        max: usize,
    ) -> usize {
        let count_key = Self::storage_key(tool, &format!("{key}.count"));
        let stored = self
            .values
            .entry(count_key.clone())
            .or_insert_with(|| default.to_string());
        let mut count = stored.parse::<usize>().unwrap_or(default).clamp(min, max);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{label}  ·  {count} 项"))
                    .size(13.0)
                    .strong()
                    .color(INK),
            );
            let remove = ui.add_enabled(
                count > min,
                egui::Button::new("− 删除一项").min_size(Vec2::new(96.0, 34.0)),
            );
            if remove.clicked() {
                count -= 1;
            }
            let add = ui.add_enabled(
                count < max,
                egui::Button::new("＋ 添加一项").min_size(Vec2::new(96.0, 34.0)),
            );
            if add.clicked() {
                count += 1;
            }
        });
        self.values.insert(count_key, count.to_string());
        count
    }

    /// 收集全部控件后再返回首个错误，防止无效值使后续输入框从界面上消失。
    pub(crate) fn dynamic_values(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        count: usize,
        label: &str,
        defaults: &[&str],
        hint: &str,
    ) -> Result<Vec<f64>, String> {
        let mut values = Vec::with_capacity(count);
        let mut first_error = None;
        for index in 0..count {
            let field_key = format!("{key}.{index}");
            let storage_key = Self::storage_key(tool, &field_key);
            let value = self
                .values
                .entry(storage_key)
                .or_insert_with(|| defaults.get(index).copied().unwrap_or("0").to_owned());
            input_row(ui, &format!("{label} {}", index + 1), value, hint);
            match parse_number(value) {
                Ok(number) => values.push(number),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(format!("第 {} 项：{error}", index + 1));
                    }
                    values.push(0.0);
                }
            }
        }
        first_error.map_or(Ok(values), Err)
    }

    /// labels 必须非空；调用方提供静态或已确认非空的目录，返回值始终在目录内。
    pub(crate) fn select_index(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
        key: &str,
        label: &str,
        labels: &[&str],
        default: usize,
    ) -> usize {
        let storage_key = Self::storage_key(tool, key);
        let value = self
            .values
            .entry(storage_key)
            .or_insert_with(|| default.min(labels.len() - 1).to_string());
        let mut index = value
            .parse::<usize>()
            .unwrap_or(default)
            .min(labels.len() - 1);
        ui.horizontal(|ui| {
            ui.add_sized(
                [132.0, 29.0],
                egui::Label::new(RichText::new(label).color(MUTED)),
            );
            egui::ComboBox::from_id_salt(format!("{}-{key}", tool.id))
                .selected_text(labels[index])
                .width(ui.available_width().min(270.0))
                .show_ui(ui, |ui| {
                    for (i, item) in labels.iter().enumerate() {
                        ui.selectable_value(&mut index, i, *item);
                    }
                });
        });
        *value = index.to_string();
        index
    }

    fn render_unit_page(&mut self, ui: &mut egui::Ui, tool: &ToolDef, dimension: Dimension) {
        let units = units_for(dimension);
        let source_labels = units
            .iter()
            .map(|unit| format!("{} · {}", unit.name, unit.code))
            .collect::<Vec<_>>();
        ui.group(|ui| {
            ui.set_max_width(ui.available_width().min(830.0));
            ui.label(RichText::new("换算参数").size(14.0).strong().color(INK));
            ui.add_space(8.0);
            self.input(ui, tool, "value", "输入数值", "1", "例如 1");
            let selected_unit = self.select_index(
                ui,
                tool,
                "source_unit",
                "输入单位",
                &source_labels.iter().map(String::as_str).collect::<Vec<_>>(),
                0,
            );
            ui.add_space(6.0);
            if matches!(dimension, Dimension::Concentration) {
                ui.label(
                    RichText::new("换算基准：等密度 1000 kg/m³；20 滴/mL")
                        .size(12.0)
                        .color(MUTED),
                );
            }
            let input_value = self.number(tool, "value", "1");
            let results = input_value.and_then(|value| {
                units
                    .iter()
                    .enumerate()
                    .map(|(target, _)| {
                        crate::catalog::convert_unit(dimension, value, selected_unit, target)
                    })
                    .collect::<Result<Vec<_>, _>>()
            });
            match results {
                Ok(results) => {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                format_number(self.number(tool, "value", "1").unwrap_or(1.0)),
                                units[selected_unit].code
                            ))
                            .size(13.0)
                            .strong()
                            .color(INK),
                        );
                        ui.label(
                            RichText::new(format!("换算结果 · 全部 {} 项", units.len()))
                                .size(12.0)
                                .color(MUTED),
                        );
                    });
                    ui.add_space(4.0);
                    for (unit, result) in units.iter().zip(results) {
                        ui.horizontal(|ui| {
                            ui.set_min_height(43.0);
                            ui.add_sized(
                                [ui.available_width() * 0.62, 38.0],
                                egui::Label::new(
                                    RichText::new(format_unit(result))
                                        .size(15.0)
                                        .color(Color32::from_rgb(166, 174, 186)),
                                ),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(unit.name).size(11.5).color(INK));
                                    ui.label(
                                        RichText::new(unit.code).size(13.5).strong().color(BLUE),
                                    );
                                });
                            });
                        });
                        ui.separator();
                    }
                }
                Err(error) => render_error_message(ui, &error),
            }
        });
        info_note(ui, tool.kind);
    }

    fn render_calculator(&mut self, ui: &mut egui::Ui, tool: &ToolDef, kind: ToolKind) {
        match kind {
            ToolKind::Gauge(kind) => self.render_gauge(ui, tool, kind),
            ToolKind::WorkingThermocouple => self.render_thermocouple(ui, tool, false),
            ToolKind::StandardThermocouple => self.render_thermocouple(ui, tool, true),
            ToolKind::IndustrialResistance => self.render_industrial_rtd(ui, tool),
            ToolKind::StandardPlatinumResistor => self.render_standard_resistance(ui, tool),
            ToolKind::Thermistor => self.render_thermistor(ui, tool),
            ToolKind::PressureReference => self.render_pressure_reference(ui, tool),
            ToolKind::TransmitterReference => self.render_transmitter_reference(ui, tool),
            ToolKind::HeightCorrection => self.render_height_correction(ui, tool),
            ToolKind::AltitudePressure => self.render_altitude_pressure(ui, tool),
            ToolKind::HumidityConversion => self.render_humidity(ui, tool),
            ToolKind::EarBlackbody => self.render_radiation(ui, tool, 0),
            ToolKind::BlackbodyError => self.render_radiation(ui, tool, 1),
            ToolKind::RadiationCorrection => self.render_radiation(ui, tool, 2),
            ToolKind::Repeatability => self.render_repeatability(ui, tool),
            ToolKind::RangeMethod => self.render_range_method(ui, tool),
            ToolKind::ResultantError => self.render_resultant_error(ui, tool),
            ToolKind::ErrorAnalysis => self.render_error_analysis(ui, tool),
            ToolKind::Uncertainty => self.render_uncertainty(ui, tool),
            ToolKind::Transmitter => self.render_transmitter(ui, tool),
            ToolKind::BSplineInterpolation => self.render_bspline(ui, tool),
            ToolKind::CalibrationCurveFit => self.render_curve_fit(ui, tool),
            ToolKind::Unit(_) => {}
        }
    }
}
