//! 应用状态、现代清爽工作台布局（顶栏、多标签、侧栏、全局 Ctrl+K 命令面板、Toast）与页面调度。
use crate::calculators::common::*;
use crate::calculators::spline::ExportJob;
use crate::catalog::{Dimension, Group, TOOLS, ToolDef, ToolKind, units_for};
use crate::navigation;
use crate::theme::{self, ThemeMode, group_accent, tool_icon};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Key, Layout, Modifiers, RichText,
    ScrollArea, Sense, Stroke, Vec2,
};
use egui_notify::{Anchor, Toasts};
use egui_phosphor::regular as icons;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;
pub(crate) mod calibration_archive;
mod dashboard;
mod input_history;
mod pdf_download;
mod ui_prefs;
use calibration_archive::CalibrationArchives;
use input_history::InputHistory;
use ui_prefs::UiPrefs;

const ZOOM_STEPS: &[f32] = &[0.9, 1.0, 1.1, 1.2];

pub(crate) struct ToolboxApp {
    selected: Option<&'static str>,
    selected_group: Option<Group>,
    // 保留用户输入的原始文本，允许编辑中的空串、负号等暂时无效状态。
    values: HashMap<String, String>,
    history: InputHistory,
    archives: CalibrationArchives,
    prefs: UiPrefs,
    pub(super) favorites: Vec<String>,
    pub(super) recent_tools: Vec<&'static str>,
    open_tabs: Vec<&'static str>,
    opened: Option<&'static str>,
    example_revision: u64,
    sidebar_collapsed: bool,
    sidebar_query: String,
    collapsed_groups: HashSet<Group>,
    dashboard_query: String,
    unit_filter: String,
    palette_open: bool,
    palette_query: String,
    palette_index: usize,
    palette_just_opened: bool,
    theme_mode: ThemeMode,
    theme_synced: bool,
    zoom_factor: f32,
    toasts: Toasts,
    spline_export: Option<ExportJob>,
    spline_export_message: Option<String>,
    pdf_download: pdf_download::PdfDownloadPage,
    // 页眉纹理由 egui 持有，保持高分辨率源图以适应窗口 DPI 缩放。
    brand_icon: Option<egui::TextureHandle>,
}

impl Default for ToolboxApp {
    fn default() -> Self {
        let history = InputHistory::new();
        let archives = CalibrationArchives::load(history.sibling_path("calibrations-v1.json"));
        let prefs = UiPrefs::load(history.sibling_path("ui-prefs-v1.json"));
        let favorites = prefs.favorites.clone();
        let recent_tools: Vec<&'static str> = prefs
            .recent
            .iter()
            .filter_map(|id| TOOLS.iter().find(|t| t.id == id.as_str()).map(|t| t.id))
            .collect();
        let theme_mode = prefs.theme;
        let zoom_factor = prefs.zoom;
        // 启动时侧栏所有分类默认全部收起，界面更清爽聚焦。
        let collapsed_groups: HashSet<Group> = Group::ALL.into_iter().collect();
        Self {
            selected: None,
            selected_group: None,
            values: history.values.clone(),
            history,
            archives,
            prefs,
            favorites,
            recent_tools,
            open_tabs: Vec::new(),
            opened: None,
            example_revision: 0,
            sidebar_collapsed: false,
            sidebar_query: String::new(),
            collapsed_groups,
            dashboard_query: String::new(),
            unit_filter: String::new(),
            palette_open: false,
            palette_query: String::new(),
            palette_index: 0,
            palette_just_opened: false,
            theme_mode,
            theme_synced: false,
            zoom_factor,
            toasts: Toasts::default().with_anchor(Anchor::BottomRight),
            spline_export: None,
            spline_export_message: None,
            pdf_download: Default::default(),
            brand_icon: None,
        }
    }
}

impl eframe::App for ToolboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_synced {
            theme::apply_theme(ctx, self.theme_mode);
            ctx.set_zoom_factor(self.zoom_factor);
            self.theme_synced = true;
        }
        self.handle_shortcuts(ctx);
        self.poll_spline_export(ctx);
        self.pdf_download.poll(ctx);

        let p = theme::palette(ctx);

        // 顶部现代功能栏
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .inner_margin(egui::Margin::symmetric(18, 8)),
            )
            .show(ctx, |ui| self.render_header(ui));

        // 顶部多工具标签栏（有打开的工具时显示）
        if !self.open_tabs.is_empty() {
            egui::TopBottomPanel::top("workspace_tabs")
                .frame(
                    egui::Frame::new()
                        .fill(p.surface_subtle)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .inner_margin(egui::Margin::symmetric(14, 5)),
                )
                .show(ctx, |ui| self.render_tab_bar(ui));
        }

        if ctx.screen_rect().width() >= 880.0 {
            egui::SidePanel::left("navigation")
                .exact_width(if self.sidebar_collapsed { 74.0 } else { 244.0 })
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .inner_margin(egui::Margin::symmetric(
                            if self.sidebar_collapsed { 8 } else { 12 },
                            12,
                        )),
                )
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
                        .fill(p.surface)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .inner_margin(egui::Margin::symmetric(12, 6)),
                )
                .show(ctx, |ui| self.render_compact_navigation(ui));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.page)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ctx, |ui| self.render_main(ui));

        // 全局 Ctrl+K 命令面板
        if self.palette_open {
            self.render_command_palette(ctx);
        }

        // 统一消费并展示 Toast 通知
        theme::drain_toasts(ctx, &mut self.toasts);
        self.toasts.show(ctx);

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
        self.save_prefs();
    }
}

impl ToolboxApp {
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
            self.palette_open = !self.palette_open;
            if self.palette_open {
                self.palette_query.clear();
                self.palette_index = 0;
                self.palette_just_opened = true;
            }
        }
        if self.palette_open && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.palette_open = false;
        }
    }

    fn save_prefs(&mut self) {
        self.prefs.theme = self.theme_mode;
        self.prefs.zoom = self.zoom_factor;
        self.prefs.favorites = self.favorites.clone();
        self.prefs.recent = self.recent_tools.iter().map(|s| (*s).to_owned()).collect();
        self.prefs.save();
    }

    pub(super) fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|item| item == id)
    }

    pub(super) fn toggle_favorite(&mut self, ctx: &egui::Context, id: &'static str) {
        let name = TOOLS
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name)
            .unwrap_or(id);
        if let Some(pos) = self.favorites.iter().position(|item| item == id) {
            self.favorites.remove(pos);
            theme::toast_info(ctx, format!("已取消收藏：{name}"));
        } else {
            self.favorites.push(id.to_owned());
            theme::toast_success(ctx, format!("已加入收藏：{name}"));
        }
        self.save_prefs();
    }

    pub(super) fn open_tool(&mut self, id: &'static str) {
        self.selected = Some(id);
        if let Some(tool) = TOOLS.iter().find(|t| t.id == id) {
            self.selected_group = Some(tool.group);
            self.collapsed_groups.remove(&tool.group);
        }
        if !self.open_tabs.contains(&id) {
            self.open_tabs.push(id);
            if self.open_tabs.len() > 10 {
                self.open_tabs.remove(0);
            }
        }
        self.recent_tools.retain(|&existing| existing != id);
        self.recent_tools.insert(0, id);
        self.recent_tools.truncate(8);
        self.unit_filter.clear();
        self.save_prefs();
    }

    fn close_tab(&mut self, id: &'static str) {
        if let Some(pos) = self.open_tabs.iter().position(|&item| item == id) {
            self.open_tabs.remove(pos);
            if self.selected == Some(id) {
                self.selected = if self.open_tabs.is_empty() {
                    None
                } else {
                    Some(self.open_tabs[pos.min(self.open_tabs.len() - 1)])
                };
            }
        }
    }

    pub(crate) fn calibration_archive_controls(
        &mut self,
        ui: &mut egui::Ui,
        tool: &ToolDef,
    ) -> Option<calibration_archive::SaveRequest> {
        self.archives.show(ui, tool.id, &mut self.values)
    }

    pub(crate) fn finish_calibration_save(
        &mut self,
        tool: &ToolDef,
        request: Option<calibration_archive::SaveRequest>,
        required: Option<Vec<String>>,
    ) {
        self.archives
            .finish_save(tool.id, &self.values, request, required);
    }

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
                let msg = "CSV 导出线程异常退出".to_owned();
                theme::toast_error(ctx, msg.clone());
                self.spline_export_message = Some(msg);
            }
            Ok(result) => {
                let cancelled = job.cancelled.load(Ordering::Relaxed);
                self.spline_export = None;
                let msg = match result {
                    Ok(path) => {
                        let m = format!("CSV 已导出：{}", path.display());
                        theme::toast_success(ctx, m.clone());
                        m
                    }
                    Err(_) if cancelled => {
                        let m = "CSV 导出已取消".to_owned();
                        theme::toast_info(ctx, m.clone());
                        m
                    }
                    Err(error) => {
                        let m = format!("CSV 导出失败：{error}");
                        theme::toast_error(ctx, m.clone());
                        m
                    }
                };
                self.spline_export_message = Some(msg);
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
        let p = theme::palette(ui.ctx());
        let icon_id = self
            .brand_icon
            .get_or_insert_with(|| {
                let data = crate::icon::data();
                ui.ctx().load_texture(
                    "toolbox-brand",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [data.width as usize, data.height as usize],
                        &data.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        let compact = ui.available_width() < 760.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;

            // 左侧可点击品牌标识（回到首页）
            let brand_resp = ui
                .horizontal(|ui| {
                    ui.image((icon_id, Vec2::splat(32.0)));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new("工具箱").size(15.5).strong().color(p.ink));
                        if !compact {
                            ui.label(
                                RichText::new(format!("计量工程工作台 · {} 项工具", TOOLS.len()))
                                    .size(10.5)
                                    .color(p.muted),
                            );
                        }
                    });
                })
                .response
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("返回工具箱首页");
            if brand_resp.clicked() {
                self.selected = None;
            }

            // 右侧工具控件 + 中间全局搜索触发栏
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // 深浅主题切换按钮
                let theme_icon = if self.theme_mode == ThemeMode::Dark {
                    icons::SUN
                } else {
                    icons::MOON
                };
                let theme_tip = if self.theme_mode == ThemeMode::Dark {
                    "切换到现代浅色主题"
                } else {
                    "切换到护眼深色主题"
                };
                if ui
                    .add_sized(
                        [34.0, 32.0],
                        egui::Button::new(RichText::new(theme_icon).size(16.0).color(p.ink_secondary))
                            .fill(p.surface_subtle)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .corner_radius(CornerRadius::same(8)),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(theme_tip)
                    .clicked()
                {
                    self.theme_mode = self.theme_mode.toggle();
                    theme::apply_theme(ui.ctx(), self.theme_mode);
                    self.save_prefs();
                }

                // 界面缩放切换按钮
                let zoom_label = format!("{}%", (self.zoom_factor * 100.0).round() as i32);
                if ui
                    .add_sized(
                        [52.0, 32.0],
                        egui::Button::new(
                            RichText::new(zoom_label)
                                .font(FontId::monospace(11.5))
                                .color(p.ink_secondary),
                        )
                        .fill(p.surface_subtle)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .corner_radius(CornerRadius::same(8)),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("切换界面缩放比例（90% / 100% / 110% / 120%）")
                    .clicked()
                {
                    let next_idx = ZOOM_STEPS
                        .iter()
                        .position(|&z| (z - self.zoom_factor).abs() < 0.03)
                        .map(|i| (i + 1) % ZOOM_STEPS.len())
                        .unwrap_or(1);
                    self.zoom_factor = ZOOM_STEPS[next_idx];
                    ui.ctx().set_zoom_factor(self.zoom_factor);
                    self.save_prefs();
                    theme::toast_info(
                        ui.ctx(),
                        format!("界面缩放已设为 {}%", (self.zoom_factor * 100.0).round() as i32),
                    );
                }

                // 全局快捷搜索胶囊（点击或按 Ctrl+K 呼出命令面板）
                let search_w = if compact {
                    148.0
                } else {
                    (ui.available_width() - 24.0).clamp(220.0, 340.0)
                };
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(search_w, 32.0), Sense::click());
                let hovered = resp.hovered();
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(9),
                    if hovered { p.surface_hover } else { p.surface_subtle },
                );
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(9),
                    Stroke::new(1.0_f32, if hovered { p.accent } else { p.border }),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    egui::pos2(rect.left() + 12.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    format!(
                        "{}  {}",
                        icons::MAGNIFYING_GLASS,
                        if compact { "快搜工具..." } else { "快速搜索任意工具..." }
                    ),
                    FontId::proportional(12.5),
                    p.muted,
                );
                if !compact {
                    let kbd_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.right() - 32.0, rect.center().y),
                        Vec2::new(48.0, 20.0),
                    );
                    ui.painter()
                        .rect_filled(kbd_rect, CornerRadius::same(5), p.surface);
                    ui.painter().rect_stroke(
                        kbd_rect,
                        CornerRadius::same(5),
                        Stroke::new(1.0_f32, p.border),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        kbd_rect.center(),
                        Align2::CENTER_CENTER,
                        "Ctrl K",
                        FontId::monospace(10.0),
                        p.muted,
                    );
                }
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("打开快捷命令面板 (Ctrl+K)")
                    .clicked()
                {
                    self.palette_open = true;
                    self.palette_query.clear();
                    self.palette_index = 0;
                    self.palette_just_opened = true;
                }
            });
        });
    }

    fn render_tab_bar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui.ctx());
        let mut switch_to: Option<Option<&'static str>> = None;
        let mut close_id: Option<&'static str> = None;

        ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // 固定首页标签
                    let home_active = self.selected.is_none();
                    let home_btn = egui::Button::new(
                        RichText::new(format!("{} 首页", icons::HOUSE))
                            .size(12.0)
                            .color(if home_active { p.accent } else { p.ink_secondary }),
                    )
                    .fill(if home_active { p.surface } else { Color32::TRANSPARENT })
                    .stroke(if home_active {
                        Stroke::new(1.0_f32, p.accent_border)
                    } else {
                        Stroke::NONE
                    })
                    .corner_radius(CornerRadius::same(7))
                    .min_size(Vec2::new(0.0, 26.0));
                    if ui
                        .add(home_btn)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        switch_to = Some(None);
                    }

                    for &tab_id in &self.open_tabs {
                        let Some(tool) = TOOLS.iter().find(|t| t.id == tab_id) else {
                            continue;
                        };
                        let active = self.selected == Some(tab_id);
                        let accent = group_accent(tool.group, p.dark);
                        let label_galley = ui.painter().layout_no_wrap(
                            format!("{} {}", tool_icon(tool), tool.name),
                            FontId::proportional(12.0),
                            if active { accent.fg } else { p.ink_secondary },
                        );
                        let tab_w = (label_galley.size().x + 36.0).clamp(84.0, 200.0);
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(tab_w, 26.0), Sense::click());
                        let close_rect = egui::Rect::from_center_size(
                            egui::pos2(rect.right() - 12.0, rect.center().y),
                            Vec2::splat(16.0),
                        );
                        let close_hovered = ui
                            .ctx()
                            .pointer_latest_pos()
                            .is_some_and(|pos| close_rect.contains(pos));

                        let bg = if active {
                            p.surface
                        } else if resp.hovered() {
                            p.surface_hover
                        } else {
                            Color32::TRANSPARENT
                        };
                        if bg != Color32::TRANSPARENT {
                            ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
                        }
                        if active {
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(7),
                                Stroke::new(1.0_f32, accent.fg.gamma_multiply(0.45)),
                                egui::StrokeKind::Inside,
                            );
                        }

                        ui.painter().galley(
                            egui::pos2(
                                rect.left() + 9.0,
                                rect.center().y - label_galley.size().y / 2.0,
                            ),
                            label_galley,
                            if active { accent.fg } else { p.ink_secondary },
                        );

                        if close_hovered {
                            ui.painter()
                                .rect_filled(close_rect, CornerRadius::same(4), p.surface_hover);
                        }
                        ui.painter().text(
                            close_rect.center(),
                            Align2::CENTER_CENTER,
                            icons::X,
                            FontId::proportional(10.5),
                            if close_hovered { p.danger } else { p.muted },
                        );

                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.middle_clicked() {
                            close_id = Some(tab_id);
                        } else if resp.clicked() {
                            if resp
                                .interact_pointer_pos()
                                .is_some_and(|pos| close_rect.contains(pos))
                            {
                                close_id = Some(tab_id);
                            } else {
                                switch_to = Some(Some(tab_id));
                            }
                        }
                    }
                });
            });

        if let Some(target) = switch_to {
            match target {
                Some(id) => self.open_tool(id),
                None => self.selected = None,
            }
        }
        if let Some(id) = close_id {
            self.close_tab(id);
        }
    }

    fn render_command_palette(&mut self, ctx: &egui::Context) {
        let p = theme::palette(ctx);
        let screen = ctx.screen_rect();
        // 半透明遮罩，点击空白关闭
        egui::Area::new(egui::Id::new("cmd_palette_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::ZERO,
                    Color32::from_rgba_premultiplied(15, 23, 42, if p.dark { 150 } else { 80 }),
                );
                if resp.clicked() {
                    self.palette_open = false;
                }
            });

        let query = self.palette_query.trim().to_lowercase();
        let matches: Vec<&ToolDef> = TOOLS
            .iter()
            .filter(|tool| dashboard::matches_query(tool, &query))
            .take(12)
            .collect();

        if !matches.is_empty() {
            self.palette_index = self.palette_index.min(matches.len() - 1);
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                self.palette_index = (self.palette_index + 1) % matches.len();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                self.palette_index = (self.palette_index + matches.len() - 1) % matches.len();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                let id = matches[self.palette_index].id;
                self.palette_open = false;
                self.open_tool(id);
                return;
            }
        }

        let panel_w = (screen.width() - 48.0).clamp(320.0, 560.0);
        let mut chosen: Option<&'static str> = None;

        egui::Window::new("command_palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .order(egui::Order::Tooltip)
            .fixed_pos(egui::pos2(
                screen.center().x - panel_w / 2.0,
                screen.top() + 72.0,
            ))
            .fixed_size(Vec2::new(panel_w, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0_f32, p.border_strong))
                    .corner_radius(CornerRadius::same(14))
                    .shadow(p.popover_shadow)
                    .inner_margin(14),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(icons::MAGNIFYING_GLASS)
                            .size(18.0)
                            .color(p.accent),
                    );
                    let edit = ui.add_sized(
                        [ui.available_width() - 36.0, 32.0],
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .frame(false)
                            .font(FontId::proportional(15.0))
                            .vertical_align(Align::Center)
                            .hint_text("输入工具名称、分类或关键词，回车打开..."),
                    );
                    if self.palette_just_opened {
                        edit.request_focus();
                        self.palette_just_opened = false;
                    }
                    if edit.changed() {
                        self.palette_index = 0;
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Esc").size(11.0).color(p.muted))
                                .fill(p.surface_subtle)
                                .stroke(Stroke::new(1.0_f32, p.border))
                                .corner_radius(CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        self.palette_open = false;
                    }
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                if matches.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("未找到匹配工具，换个关键词试试")
                                .size(13.0)
                                .color(p.muted),
                        );
                        ui.add_space(16.0);
                    });
                } else {
                    ScrollArea::vertical()
                        .max_height(360.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            for (idx, tool) in matches.iter().enumerate() {
                                let selected = idx == self.palette_index;
                                let accent = group_accent(tool.group, p.dark);
                                let (rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 44.0),
                                    Sense::click(),
                                );
                                if resp.hovered() {
                                    self.palette_index = idx;
                                }
                                let highlight = selected || resp.hovered();
                                if highlight {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(8),
                                        accent.bg,
                                    );
                                }
                                let icon_rect = egui::Rect::from_center_size(
                                    egui::pos2(rect.left() + 22.0, rect.center().y),
                                    Vec2::splat(28.0),
                                );
                                ui.painter().rect_filled(
                                    icon_rect,
                                    CornerRadius::same(7),
                                    if highlight { p.surface } else { accent.bg },
                                );
                                ui.painter().text(
                                    icon_rect.center(),
                                    Align2::CENTER_CENTER,
                                    tool_icon(tool),
                                    FontId::proportional(15.0),
                                    accent.fg,
                                );
                                ui.painter().text(
                                    egui::pos2(rect.left() + 44.0, rect.top() + 14.0),
                                    Align2::LEFT_CENTER,
                                    tool.name,
                                    FontId::proportional(13.5),
                                    if highlight { accent.fg } else { p.ink },
                                );
                                ui.painter().text(
                                    egui::pos2(rect.left() + 44.0, rect.bottom() - 12.0),
                                    Align2::LEFT_CENTER,
                                    tool.description,
                                    FontId::proportional(11.0),
                                    p.muted,
                                );
                                ui.painter().text(
                                    egui::pos2(rect.right() - 10.0, rect.center().y),
                                    Align2::RIGHT_CENTER,
                                    tool.group.title(),
                                    FontId::proportional(11.0),
                                    p.muted,
                                );
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    chosen = Some(tool.id);
                                }
                            }
                        });
                }
            });

        if let Some(id) = chosen {
            self.palette_open = false;
            self.open_tool(id);
        }
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui.ctx());
        let mut open_id: Option<&'static str> = None;
        let mut toggle_fav: Option<&'static str> = None;

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} 工具导航", icons::COMPASS))
                    .size(13.5)
                    .strong()
                    .color(p.ink),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_sized(
                        [28.0, 28.0],
                        egui::Button::new(
                            RichText::new(icons::SIDEBAR_SIMPLE)
                                .size(16.0)
                                .color(p.muted),
                        )
                        .frame(false),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("折叠侧栏导航")
                    .clicked()
                {
                    self.sidebar_collapsed = true;
                }
            });
        });

        ui.add_space(6.0);
        // 侧栏快速过滤框
        egui::Frame::new()
            .fill(p.surface_subtle)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(icons::MAGNIFYING_GLASS)
                            .size(13.0)
                            .color(p.muted),
                    );
                    ui.add_sized(
                        [ui.available_width() - 22.0, 24.0],
                        egui::TextEdit::singleline(&mut self.sidebar_query)
                            .frame(false)
                            .font(FontId::proportional(12.5))
                            .vertical_align(Align::Center)
                            .hint_text("筛选导航..."),
                    );
                    if !self.sidebar_query.is_empty()
                        && ui
                            .add(
                                egui::Button::new(
                                    RichText::new(icons::X).size(12.0).color(p.muted),
                                )
                                .frame(false),
                            )
                            .clicked()
                    {
                        self.sidebar_query.clear();
                    }
                });
            });

        ui.add_space(8.0);
        let filter = self.sidebar_query.trim().to_lowercase();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;

                // 收藏夹分组（无搜索词且有收藏时置顶）
                if filter.is_empty() && !self.favorites.is_empty() {
                    ui.label(
                        RichText::new(format!("{} 我的收藏", icons::STAR))
                            .size(11.5)
                            .strong()
                            .color(Color32::from_rgb(217, 119, 6)),
                    );
                    ui.add_space(2.0);
                    let fav_tools: Vec<&ToolDef> = self
                        .favorites
                        .iter()
                        .filter_map(|id| TOOLS.iter().find(|t| t.id == id.as_str()))
                        .collect();
                    for tool in fav_tools {
                        let outcome = navigation::tool(
                            ui,
                            tool.group,
                            tool.name,
                            self.selected == Some(tool.id),
                            true,
                        );
                        if outcome.toggled_favorite {
                            toggle_fav = Some(tool.id);
                        } else if outcome.clicked {
                            open_id = Some(tool.id);
                        }
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);
                }

                for group in Group::ALL {
                    let tools: Vec<&ToolDef> = TOOLS
                        .iter()
                        .filter(|tool| {
                            tool.group == group && dashboard::matches_query(tool, &filter)
                        })
                        .collect();
                    if tools.is_empty() {
                        continue;
                    }
                    let is_collapsed =
                        filter.is_empty() && self.collapsed_groups.contains(&group);
                    if navigation::category(
                        ui,
                        group,
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
                        ui.add_space(4.0);
                        continue;
                    }
                    for tool in tools {
                        let selected = self.selected == Some(tool.id);
                        let fav = self.is_favorite(tool.id);
                        let outcome = navigation::tool(ui, group, tool.name, selected, fav);
                        outcome
                            .response
                            .on_hover_text(format!("{}\n{}", tool.name, tool.description));
                        if outcome.toggled_favorite {
                            toggle_fav = Some(tool.id);
                        } else if outcome.clicked {
                            open_id = Some(tool.id);
                        }
                    }
                    ui.add_space(6.0);
                }
            });

        if let Some(id) = toggle_fav {
            self.toggle_favorite(ui.ctx(), id);
        }
        if let Some(id) = open_id {
            self.open_tool(id);
        }
    }

    fn render_collapsed_sidebar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui.ctx());
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            if ui
                .add_sized(
                    [44.0, 36.0],
                    egui::Button::new(
                        RichText::new(icons::SIDEBAR_SIMPLE)
                            .size(18.0)
                            .color(p.ink_secondary),
                    )
                    .fill(p.surface_subtle)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .corner_radius(CornerRadius::same(8)),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("展开侧栏导航")
                .clicked()
            {
                self.sidebar_collapsed = false;
            }
            ui.add_space(4.0);
            for group in Group::ALL {
                if navigation::collapsed_group_button(
                    ui,
                    group,
                    self.selected_group == Some(group),
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
        let p = theme::palette(ui.ctx());
        ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let all_active = self.selected_group.is_none() && self.selected.is_none();
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("全部")
                                    .size(12.0)
                                    .color(if all_active { p.accent } else { p.ink_secondary }),
                            )
                            .fill(if all_active {
                                p.accent_soft
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(99)),
                        )
                        .clicked()
                    {
                        self.selected_group = None;
                        self.selected = None;
                    }
                    for group in Group::ALL {
                        let active = self.selected_group == Some(group);
                        let accent = group_accent(group, p.dark);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("{} {}", accent.icon, group.short_title()))
                                        .size(12.0)
                                        .color(if active { accent.fg } else { p.ink_secondary }),
                                )
                                .fill(if active {
                                    accent.bg
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(99)),
                            )
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
        let p = theme::palette(ui.ctx());
        let accent = group_accent(tool.group, p.dark);
        let mut back = false;
        let mut toggle_fav = false;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let page_w = ui.available_width();
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(20, 0))
                    .show(ui, |ui| {
                        let content_w = (page_w - 40.0).max(200.0);
                        ui.set_min_width(content_w);
                        ui.set_max_width(content_w);
                        ui.add_space(18.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("{} 工具箱", icons::ARROW_LEFT))
                                            .size(12.5)
                                            .color(p.accent),
                                    )
                                    .fill(p.accent_soft)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(CornerRadius::same(7))
                                    .min_size(Vec2::new(0.0, 26.0)),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                back = true;
                            }
                            ui.label(
                                RichText::new(format!("/ {} / {}", tool.group.title(), tool.name))
                                    .size(12.5)
                                    .color(p.muted),
                            );
                            if tool.remembers_inputs() && self.history.error {
                                ui.label(RichText::new("输入暂未保存").size(11.0).color(p.muted))
                                    .on_hover_text(
                                        "本地保存暂时不可用，当前输入仍可使用，稍后自动重试",
                                    );
                            }
                        });
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            // 操作与标题同排，右侧放置收藏与恢复示例按钮。
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                if ui
                                    .add_enabled(
                                        self.spline_export.is_none()
                                            && !(matches!(tool.kind, ToolKind::PdfDownload)
                                                && self.pdf_download.busy()),
                                        egui::Button::new(
                                            RichText::new(
                                                if matches!(tool.kind, ToolKind::PdfDownload) {
                                                    "清空"
                                                } else {
                                                    "恢复示例"
                                                },
                                            )
                                            .size(13.0)
                                            .color(p.ink),
                                        )
                                        .fill(p.surface)
                                        .stroke(Stroke::new(1.0_f32, p.border_strong))
                                        .corner_radius(CornerRadius::same(8))
                                        .min_size(Vec2::new(96.0, 36.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text(if matches!(tool.kind, ToolKind::PdfDownload) {
                                        "清空链接和本次下载结果"
                                    } else {
                                        "将当前工具的全部输入恢复为示例值"
                                    })
                                    .clicked()
                                {
                                    self.clear_tool_inputs(tool);
                                    if matches!(tool.kind, ToolKind::PdfDownload) {
                                        self.pdf_download.clear();
                                    }
                                    self.example_revision += 1;
                                    theme::toast_info(ui.ctx(), "已恢复默认示例参数");
                                }

                                let is_fav = self.is_favorite(tool.id);
                                if ui
                                    .add_sized(
                                        [36.0, 36.0],
                                        egui::Button::new(
                                            RichText::new(icons::STAR).size(16.0).color(if is_fav {
                                                Color32::from_rgb(245, 158, 11)
                                            } else {
                                                p.muted
                                            }),
                                        )
                                        .fill(p.surface)
                                        .stroke(Stroke::new(1.0_f32, p.border_strong))
                                        .corner_radius(CornerRadius::same(8)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text(if is_fav { "取消收藏" } else { "加入收藏" })
                                    .clicked()
                                {
                                    toggle_fav = true;
                                }

                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    let (icon_rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                                    ui.painter().rect_filled(
                                        icon_rect,
                                        CornerRadius::same(11),
                                        accent.bg,
                                    );
                                    ui.painter().text(
                                        icon_rect.center(),
                                        Align2::CENTER_CENTER,
                                        tool_icon(tool),
                                        FontId::proportional(22.0),
                                        accent.fg,
                                    );
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.label(
                                            RichText::new(tool.name).size(22.0).strong().color(p.ink),
                                        );
                                        ui.label(
                                            RichText::new(tool.description)
                                                .size(12.5)
                                                .color(p.muted),
                                        );
                                    });
                                });
                            });
                        });
                        ui.add_space(16.0);
                        ui.push_id((tool.id, self.example_revision), |ui| match tool.kind {
                            ToolKind::Unit(dimension) => self.render_unit_page(ui, tool, dimension),
                            kind => self.render_calculator(ui, tool, kind),
                        });
                        ui.add_space(28.0);
                    });
            });
        self.history.observe(tool, &self.values);
        if toggle_fav {
            self.toggle_favorite(ui.ctx(), tool.id);
        }
        if back {
            self.selected = None;
        }
    }

    fn clear_tool_inputs(&mut self, tool: &ToolDef) {
        let prefix = format!("{}.", tool.id);
        self.values.retain(|key, _| !key.starts_with(&prefix));
        self.archives.clear_selection(tool.id);
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

    pub(crate) fn tool_inputs(&self, tool: &ToolDef) -> std::collections::BTreeMap<&str, &str> {
        let prefix = format!("{}.", tool.id);
        self.values
            .iter()
            .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|key| (key, value.as_str())))
            .collect()
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
        text_input(ui, value, width.max(52.0), hint).changed()
    }

    /// 调整项目数量时保留已有字段值；重新增加项目可恢复本次运行中的输入。
    #[allow(clippy::too_many_arguments)]
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
        let p = theme::palette(ui.ctx());
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
                    .color(p.ink),
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
    #[allow(clippy::too_many_arguments)]
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
        form_row(ui, label, |ui| {
            ui.spacing_mut().interact_size.y = 36.0;
            egui::ComboBox::from_id_salt(format!("{}-{key}", tool.id))
                .selected_text(labels[index])
                .width(ui.available_width())
                .wrap_mode(egui::TextWrapMode::Truncate)
                .show_ui(ui, |ui| {
                    // 展开项按菜单宽度换行，长名称不延伸到窗口外。
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    for (i, item) in labels.iter().enumerate() {
                        ui.selectable_value(&mut index, i, *item);
                    }
                });
        });
        *value = index.to_string();
        index
    }

    /// 现代胶囊分段选择：将状态同样保存在 values 中，适合 2~4 个短选项。
    pub(crate) fn select_segmented(
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
        form_row(ui, label, |ui| {
            ui.push_id((tool.id, key), |ui| {
                index = segmented_control(ui, labels, index);
            });
        });
        *value = index.to_string();
        index
    }

    fn render_unit_page(&mut self, ui: &mut egui::Ui, tool: &ToolDef, dimension: Dimension) {
        let p = theme::palette(ui.ctx());
        let units = units_for(dimension);
        let source_labels = units
            .iter()
            .map(|unit| format!("{} · {}", unit.name, unit.code))
            .collect::<Vec<_>>();
        let mut process = None;
        let mut switch_source_to: Option<(usize, String)> = None;
        let mut copied_text: Option<String> = None;

        // 宽屏左输右显双栏工作台，窄屏自动回退为上下单栏。
        split_workspace(
            ui,
            0.36,
            self,
            |ui, app| {
                let mut selected_unit = 0usize;
                let mut results_snapshot = None;
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("换算参数").size(14.5).strong().color(p.ink));
                    ui.add_space(10.0);
                    app.input(ui, tool, "value", "输入数值", "1", "例如 1");
                    selected_unit = app.select_index(
                        ui,
                        tool,
                        "source_unit",
                        "输入单位",
                        &source_labels.iter().map(String::as_str).collect::<Vec<_>>(),
                        0,
                    );
                    ui.add_space(6.0);
                    // 快捷筛选目标单位
                    let mut filter = app.unit_filter.clone();
                    form_row(ui, "筛选结果", |ui| {
                        text_input(ui, &mut filter, ui.available_width(), "按单位名称或符号筛选");
                    });
                    app.unit_filter = filter;

                    if matches!(dimension, Dimension::Concentration) {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("换算基准：等密度 1000 kg/m³；20 滴/mL")
                                .size(12.0)
                                .color(p.muted),
                        );
                    }

                    let input_value = app.number(tool, "value", "1");
                    results_snapshot = Some(input_value.and_then(|value| {
                        units
                            .iter()
                            .enumerate()
                            .map(|(target, _)| {
                                crate::catalog::convert_unit(dimension, value, selected_unit, target)
                            })
                            .collect::<Result<Vec<_>, _>>()
                            .map(|results| (value, results))
                    }));
                });
                (selected_unit, results_snapshot.unwrap())
            },
            |ui, app, (selected_unit, results_snapshot)| {
                panel(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match results_snapshot {
                        Ok((value, results)) => {
                            let (source_factor, source_offset) =
                                units[selected_unit].conversion_coefficients();
                            let mut steps = vec![format!(
                                "输入 {} {}；源单位系数 fₛ = {}，偏移 oₛ = {}。",
                                format_process_number(value),
                                units[selected_unit].code,
                                format_process_number(source_factor),
                                format_process_number(source_offset)
                            )];
                            if matches!(dimension, Dimension::Temperature) {
                                steps.push("温度值先使用融合乘加得到开尔文 K = max(value × fₛ + oₛ, 0)，再按目标单位计算 (K − oₜ) / fₜ；同单位直接返回输入值。".to_owned());
                            } else {
                                steps.push("线性单位直接计算目标值 = 输入值 × (fₛ / fₜ)，避免基准单位中间值溢出；同单位直接返回输入值。".to_owned());
                            }
                            for (target, (unit, &result)) in units.iter().zip(&results).enumerate() {
                                steps.push(String::new());
                                let (factor, offset) = unit.conversion_coefficients();
                                let formula = if target == selected_unit {
                                    "同单位，结果等于输入值".to_owned()
                                } else if matches!(dimension, Dimension::Temperature) {
                                    let kelvin =
                                        value.mul_add(source_factor, source_offset).max(0.0);
                                    format!(
                                        "K = max({} × {} + {}, 0) = {}，目标值 = ({} − {}) / {}",
                                        format_process_number(value),
                                        format_process_number(source_factor),
                                        format_process_number(source_offset),
                                        format_process_computed(kelvin),
                                        format_process_computed(kelvin),
                                        format_process_number(offset),
                                        format_process_number(factor)
                                    )
                                } else {
                                    format!(
                                        "目标值 = {} × ({} / {})",
                                        format_process_number(value),
                                        format_process_number(source_factor),
                                        format_process_number(factor)
                                    )
                                };
                                steps.push(format!(
                                    "{}：目标系数 fₜ = {}，偏移 oₜ = {}；{} = {} {}。",
                                    unit.name,
                                    format_process_number(factor),
                                    format_process_number(offset),
                                    formula,
                                    if target == selected_unit {
                                        format_process_number(result)
                                    } else {
                                        format_process_computed(result)
                                    },
                                    unit.code
                                ));
                            }
                            process = Some(steps);

                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        format_number(value),
                                        units[selected_unit].code
                                    ))
                                    .font(FontId::monospace(14.0))
                                    .strong()
                                    .color(p.accent),
                                );
                                ui.label(
                                    RichText::new(format!("换算结果 · 全部 {} 项", units.len()))
                                        .size(12.5)
                                        .color(p.muted),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new("点击卡片复制 · 点击 ⇄ 设为源单位")
                                            .size(11.0)
                                            .color(p.muted),
                                    );
                                });
                            });
                            ui.add_space(10.0);

                            let filter = app.unit_filter.trim().to_lowercase();
                            let visible: Vec<(usize, &crate::units::UnitDef, f64)> = units
                                .iter()
                                .zip(results)
                                .enumerate()
                                .filter(|(_, (u, _))| {
                                    filter.is_empty()
                                        || u.name.to_lowercase().contains(&filter)
                                        || u.code.to_lowercase().contains(&filter)
                                })
                                .map(|(idx, (u, r))| (idx, u, r))
                                .collect();

                            if visible.is_empty() {
                                ui.label(
                                    RichText::new("没有匹配的单位").size(12.5).color(p.muted),
                                );
                            } else {
                                let cols = if ui.available_width() >= 520.0 { 2 } else { 1 };
                                for row in visible.chunks(cols) {
                                    ui.columns(cols, |columns| {
                                        for (col_idx, &(unit_idx, unit, result)) in
                                            row.iter().enumerate()
                                        {
                                            let is_source = unit_idx == selected_unit;
                                            let formatted = format_unit(result);
                                            let col_ui = &mut columns[col_idx];
                                            let w = col_ui.available_width();
                                            let (rect, resp) = col_ui.allocate_exact_size(
                                                Vec2::new(w, 60.0),
                                                Sense::click(),
                                            );
                                            let swap_rect = egui::Rect::from_center_size(
                                                egui::pos2(rect.right() - 18.0, rect.top() + 16.0),
                                                Vec2::splat(22.0),
                                            );
                                            let swap_hovered = col_ui
                                                .ctx()
                                                .pointer_latest_pos()
                                                .is_some_and(|pos| swap_rect.contains(pos));
                                            let hovered = resp.hovered();

                                            col_ui.painter().rect_filled(
                                                rect,
                                                CornerRadius::same(10),
                                                if is_source {
                                                    p.accent_soft
                                                } else if hovered {
                                                    p.surface_hover
                                                } else {
                                                    p.surface_subtle
                                                },
                                            );
                                            col_ui.painter().rect_stroke(
                                                rect,
                                                CornerRadius::same(10),
                                                Stroke::new(
                                                    if is_source || hovered {
                                                        1.2_f32
                                                    } else {
                                                        1.0_f32
                                                    },
                                                    if is_source || hovered {
                                                        p.accent
                                                    } else {
                                                        p.border
                                                    },
                                                ),
                                                egui::StrokeKind::Inside,
                                            );

                                            // 顶部行：单位名称 + 单位代码胶囊
                                            col_ui.painter().text(
                                                egui::pos2(rect.left() + 12.0, rect.top() + 16.0),
                                                Align2::LEFT_CENTER,
                                                format!("{} ({})", unit.name, unit.code),
                                                FontId::proportional(11.5),
                                                if is_source { p.accent } else { p.muted },
                                            );

                                            // 底部行：高对比度等宽结果数值
                                            let val_galley = col_ui.painter().layout(
                                                formatted.clone(),
                                                FontId::monospace(15.5),
                                                if is_source { p.accent } else { p.ink },
                                                (rect.width() - 24.0).max(60.0),
                                            );
                                            col_ui.painter().galley(
                                                egui::pos2(rect.left() + 12.0, rect.top() + 31.0),
                                                val_galley,
                                                if is_source { p.accent } else { p.ink },
                                            );

                                            // 右上角交互提示：非源单位悬停显示 ⇄ 切换为源单位
                                            if hovered && !is_source {
                                                if swap_hovered {
                                                    col_ui.painter().rect_filled(
                                                        swap_rect,
                                                        CornerRadius::same(5),
                                                        p.surface,
                                                    );
                                                }
                                                col_ui.painter().text(
                                                    swap_rect.center(),
                                                    Align2::CENTER_CENTER,
                                                    icons::ARROWS_LEFT_RIGHT,
                                                    FontId::proportional(13.0),
                                                    if swap_hovered { p.accent } else { p.muted },
                                                );
                                            } else if hovered {
                                                col_ui.painter().text(
                                                    swap_rect.center(),
                                                    Align2::CENTER_CENTER,
                                                    icons::COPY,
                                                    FontId::proportional(13.0),
                                                    p.accent,
                                                );
                                            }

                                            let resp = resp
                                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                .on_hover_text(if is_source {
                                                    "点击复制数值"
                                                } else {
                                                    "点击复制数值；点右上角 ⇄ 设为新源单位"
                                                });
                                            if resp.clicked() {
                                                if !is_source
                                                    && resp
                                                        .interact_pointer_pos()
                                                        .is_some_and(|pos| swap_rect.contains(pos))
                                                {
                                                    switch_source_to = Some((unit_idx, formatted));
                                                } else {
                                                    copied_text = Some(formatted);
                                                }
                                            }
                                        }
                                    });
                                    ui.add_space(6.0);
                                }
                            }
                        }
                        Err(error) => render_error_message(ui, &error),
                    }
                });
            },
        );

        if let Some((next_unit, next_val)) = switch_source_to {
            self.set_value(tool, "source_unit", next_unit.to_string());
            self.set_value(tool, "value", next_val);
            theme::toast_info(
                ui.ctx(),
                format!("已切换输入单位为 {}", units[next_unit].name),
            );
        }
        if let Some(val) = copied_text {
            ui.ctx().copy_text(val.clone());
            theme::toast_success(ui.ctx(), format!("已复制：{val}"));
        }

        if let Some(lines) = &process {
            calculation_process(ui, "unit_conversion_process", lines);
        }
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
            ToolKind::PdfDownload => self.pdf_download.render(ui),
            ToolKind::Unit(_) => {}
        }
    }
}

#[cfg(test)]
mod layout_overflow_tests {
    use super::*;

    #[test]
    fn all_50_tools_fit_viewport_without_right_overflow() {
        for width in [620.0_f32, 1000.0, 1220.0] {
            for tool in TOOLS {
                let mut app = ToolboxApp::default();
                let ctx = egui::Context::default();
                crate::fonts::configure_fonts(&ctx);
                crate::configure_style(&ctx);
                let run = |app: &mut ToolboxApp| {
                    ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                Vec2::new(width, 2200.0),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                app.render_selected(ui, tool.id);
                            });
                        },
                    )
                };
                run(&mut app);
                let output = run(&mut app);
                for clipped in &output.shapes {
                    if let egui::Shape::Rect(rect) = &clipped.shape
                        && rect.stroke.width > 0.0
                    {
                        let visible_right = rect.rect.right().min(clipped.clip_rect.right());
                        assert!(
                            visible_right <= width + 2.0,
                            "工具 {} 在宽度 {width} 下卡片右边界溢出：rect={:?}, clip={:?}",
                            tool.id,
                            rect.rect,
                            clipped.clip_rect
                        );
                        // 非滚动裁剪区内的面板边框本身不能超出窗口右边
                        if clipped.clip_rect.right() >= width - 1.0 && rect.rect.width() > 260.0 {
                            assert!(
                                rect.rect.right() <= width + 2.0,
                                "工具 {} 在宽度 {width} 下面板撑出视口：rect={:?}",
                                tool.id,
                                rect.rect
                            );
                        }
                    }
                }
            }
        }
    }
}
