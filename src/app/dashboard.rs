//! 首页布局：居中说明、分类筛选、自适应卡片及工具入口。
//! 先测量同一行的文本高度，再布置卡片，防止换行或可选图标造成按钮错位。
use super::ToolboxApp;
use crate::catalog::{Group, TOOLS, ToolDef};
use crate::{BLUE, BLUE_SOFT, INK, MUTED};
use eframe::egui::{
    self, Align, Color32, FontId, Galley, Layout, RichText, ScrollArea, Sense, Stroke, TextEdit,
    TextStyle, TextWrapMode, Vec2, WidgetText,
};
use std::sync::Arc;

const CARD_GAP: f32 = 14.0;
const CARD_PADDING: i8 = 14;

impl ToolboxApp {
    pub(super) fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        let mut open_tool = None;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let margin = if ui.available_width() < 720.0 { 16 } else { 24 };
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(margin, 24))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new("计量与工程计算，一处完成")
                                        .size(if ui.available_width() < 600.0 {
                                            24.0
                                        } else {
                                            28.0
                                        })
                                        .strong()
                                        .color(INK),
                                )
                                .wrap()
                                .halign(Align::Center),
                            );
                            ui.add_space(2.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new("按工具分类快速查找。")
                                        .size(13.0)
                                        .color(MUTED),
                                )
                                .wrap()
                                .halign(Align::Center),
                            );
                        });
                        ui.add_space(22.0);
                        self.render_dashboard_filters(ui);
                        ui.add_space(14.0);

                        let tools: Vec<&ToolDef> = TOOLS
                            .iter()
                            .filter(|tool| {
                                self.matches(tool)
                                    && self.selected_group.is_none_or(|group| tool.group == group)
                            })
                            .collect();
                        if tools.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                ui.label(RichText::new("没有匹配的工具").color(MUTED));
                            });
                        }

                        let columns = ((ui.available_width() + CARD_GAP) / (270.0 + CARD_GAP))
                            .floor()
                            .clamp(1.0, 4.0) as usize;
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        let card_width = (ui.available_width() - CARD_GAP * (columns - 1) as f32)
                            / columns as f32;
                        let inner_width = card_width - 2.0 * CARD_PADDING as f32 - 2.0;
                        for row in tools.chunks(columns) {
                            let text: Vec<_> = row
                                .iter()
                                .map(|tool| CardText::new(ui, tool, inner_width))
                                .collect();
                            // Measure the complete row first, so optional icons and wrapped
                            // descriptions cannot move one card's action above another's.
                            let header_height = text
                                .iter()
                                .map(CardText::header_height)
                                .fold(42.0, f32::max);
                            let description_height = text
                                .iter()
                                .map(|text| text.description.size().y)
                                .fold(34.0, f32::max);
                            ui.columns(columns, |cols| {
                                for (index, tool) in row.iter().enumerate() {
                                    if tool_card(
                                        &mut cols[index],
                                        tool,
                                        &text[index],
                                        header_height,
                                        description_height,
                                    ) {
                                        open_tool = Some(tool.id);
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }
                    });
            });
        if let Some(id) = open_tool {
            self.selected = Some(id);
            self.selected_group = TOOLS
                .iter()
                .find(|tool| tool.id == id)
                .map(|tool| tool.group);
        }
    }

    fn render_dashboard_filters(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(7.0, 8.0);
            let all_selected = self.selected_group.is_none();
            if ui.add(filter_button("全部工具", all_selected)).clicked() {
                self.selected_group = None;
            }
            for group in Group::ALL {
                let count = TOOLS.iter().filter(|tool| tool.group == group).count();
                if ui
                    .add(filter_button(
                        format!("{}  {}", group.title(), count),
                        self.selected_group == Some(group),
                    ))
                    .clicked()
                {
                    self.selected_group = Some(group);
                }
            }
            ui.add_sized(
                [210.0_f32.min(ui.max_rect().width()), 32.0],
                TextEdit::singleline(&mut self.search).hint_text("搜索名称或功能"),
            );
        });
    }
}

fn filter_button(text: impl Into<WidgetText>, selected: bool) -> egui::Button<'static> {
    egui::Button::new(text)
        .selected(selected)
        .min_size(Vec2::new(0.0, 32.0))
        .fill(if selected { BLUE_SOFT } else { Color32::WHITE })
        .corner_radius(5)
}

struct CardText {
    title: Arc<Galley>,
    category: Arc<Galley>,
    description: Arc<Galley>,
    has_icon: bool,
}

impl CardText {
    fn new(ui: &egui::Ui, tool: &ToolDef, width: f32) -> Self {
        let has_icon = !tool.icon.is_empty();
        let title_width = (width - if has_icon { 52.0 } else { 0.0 }).max(1.0);
        let layout = |text: RichText, width| {
            WidgetText::from(text).into_galley(ui, Some(TextWrapMode::Wrap), width, TextStyle::Body)
        };
        Self {
            title: layout(
                RichText::new(tool.name).size(14.5).strong().color(INK),
                title_width,
            ),
            category: layout(
                RichText::new(tool.group.title()).size(11.0).color(MUTED),
                title_width,
            ),
            description: layout(
                RichText::new(tool.description).size(12.0).color(MUTED),
                width,
            ),
            has_icon,
        }
    }

    fn header_height(&self) -> f32 {
        self.title.size().y + 4.0 + self.category.size().y
    }
}

fn tool_card(
    ui: &mut egui::Ui,
    tool: &ToolDef,
    text: &CardText,
    header_height: f32,
    description_height: f32,
) -> bool {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(221, 228, 238)))
        .corner_radius(8)
        .inner_margin(CARD_PADDING)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            let width = ui.available_width();
            let (header, _) =
                ui.allocate_exact_size(Vec2::new(width, header_height), Sense::hover());
            let mut x = header.left();
            if text.has_icon {
                let icon = egui::Rect::from_center_size(
                    egui::pos2(x + 20.0, header.center().y),
                    Vec2::splat(40.0),
                );
                ui.painter().rect_filled(icon, 6, BLUE_SOFT);
                ui.painter().text(
                    icon.center(),
                    egui::Align2::CENTER_CENTER,
                    tool.icon,
                    FontId::proportional(16.0),
                    BLUE,
                );
                x += 52.0;
            }
            let title_y = header.center().y - text.header_height() / 2.0;
            ui.painter()
                .galley(egui::pos2(x, title_y), text.title.clone(), INK);
            ui.painter().galley(
                egui::pos2(x, title_y + text.title.size().y + 4.0),
                text.category.clone(),
                MUTED,
            );
            let (description, _) =
                ui.allocate_exact_size(Vec2::new(width, description_height), Sense::hover());
            ui.painter()
                .galley(description.min, text.description.clone(), MUTED);
            ui.allocate_ui_with_layout(
                Vec2::new(width, 34.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.add_sized(
                        [116.0, 34.0],
                        egui::Button::new(RichText::new("打开工具  →").color(BLUE))
                            .fill(BLUE_SOFT)
                            .stroke(Stroke::NONE)
                            .corner_radius(5),
                    )
                    .on_hover_text(tool.name)
                    .clicked()
                },
            )
            .inner
        })
        .inner
}
