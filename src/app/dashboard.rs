//! 现代清爽首页：居中 Hero 引导、快捷访问（收藏/最近）、分类胶囊筛选与整卡可点击工具网格。
use super::ToolboxApp;
use crate::catalog::{Group, TOOLS, ToolDef};
use crate::theme::{self, group_accent, tool_icon};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Galley, RichText, ScrollArea, Sense,
    Stroke, TextStyle, TextWrapMode, Vec2, WidgetText,
};
use egui_phosphor::regular as icons;
use std::sync::Arc;

const CARD_GAP: f32 = 14.0;
const CARD_PADDING: i8 = 16;

impl ToolboxApp {
    pub(super) fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui.ctx());
        let mut open_tool = None;
        let mut toggle_fav = None;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let margin = if ui.available_width() < 720.0 { 16 } else { 28 };
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(margin, 24))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        // 清爽 Hero 头部
                        ui.vertical_centered(|ui| {
                            egui::Frame::new()
                                .fill(p.accent_soft)
                                .stroke(Stroke::new(1.0_f32, p.accent_border))
                                .corner_radius(CornerRadius::same(99))
                                .inner_margin(egui::Margin::symmetric(12, 4))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{}  7 大分类 · {} 项专业工具 · 按 Ctrl+K 随时快搜",
                                            icons::SPARKLE,
                                            TOOLS.len()
                                        ))
                                        .size(11.5)
                                        .color(p.accent),
                                    );
                                });
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new("计量与工程计算，一处完成")
                                        .size(if ui.available_width() < 600.0 {
                                            23.0
                                        } else {
                                            28.0
                                        })
                                        .strong()
                                        .color(p.ink),
                                )
                                .wrap()
                                .halign(Align::Center),
                            );
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        "按名称、说明或分类实时筛选；点击任意卡片即刻进入计算。",
                                    )
                                    .size(13.0)
                                    .color(p.muted),
                                )
                                .wrap()
                                .halign(Align::Center),
                            );
                        });

                        // 快捷访问区（收藏 + 最近使用）
                        if let Some(id) = self.render_quick_access(ui) {
                            open_tool = Some(id);
                        }

                        ui.add_space(20.0);
                        self.render_dashboard_filters(ui);
                        ui.add_space(16.0);

                        let query = self.dashboard_query.trim().to_lowercase();
                        let tools: Vec<&ToolDef> = TOOLS
                            .iter()
                            .filter(|tool| {
                                self.selected_group.is_none_or(|group| tool.group == group)
                                    && matches_query(tool, &query)
                            })
                            .collect();

                        if tools.is_empty() {
                            ui.add_space(24.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(icons::MAGNIFYING_GLASS)
                                        .size(28.0)
                                        .color(p.muted),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("没有找到匹配的工具，请换个关键词或切换分类。")
                                        .size(13.5)
                                        .color(p.muted),
                                );
                            });
                        }

                        let columns = ((ui.available_width() + CARD_GAP) / (264.0 + CARD_GAP))
                            .floor()
                            .clamp(1.0, 4.0) as usize;
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        let card_width = (ui.available_width() - CARD_GAP * (columns - 1) as f32)
                            / columns as f32;
                        let inner_width = (card_width - 2.0 * CARD_PADDING as f32 - 2.0).max(80.0);
                        for row in tools.chunks(columns) {
                            let text: Vec<_> = row
                                .iter()
                                .map(|tool| CardText::new(ui, tool, inner_width, &p))
                                .collect();
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
                                    let is_fav = self.is_favorite(tool.id);
                                    let outcome = tool_card(
                                        &mut cols[index],
                                        tool,
                                        &text[index],
                                        header_height,
                                        description_height,
                                        is_fav,
                                        &p,
                                    );
                                    if outcome.opened {
                                        open_tool = Some(tool.id);
                                    }
                                    if outcome.toggled_favorite {
                                        toggle_fav = Some(tool.id);
                                    }
                                }
                            });
                            ui.add_space(6.0);
                        }
                        ui.add_space(12.0);
                    });
            });

        if let Some(id) = toggle_fav {
            self.toggle_favorite(ui.ctx(), id);
        }
        if let Some(id) = open_tool {
            self.open_tool(id);
        }
    }

    fn render_quick_access(&mut self, ui: &mut egui::Ui) -> Option<&'static str> {
        let p = theme::palette(ui.ctx());
        let mut quick_items: Vec<(&ToolDef, bool)> = Vec::new();
        for id in &self.favorites {
            if let Some(tool) = TOOLS.iter().find(|t| t.id == id.as_str()) {
                quick_items.push((tool, true));
            }
        }
        for id in &self.recent_tools {
            if quick_items.len() >= 8 {
                break;
            }
            if quick_items.iter().any(|(t, _)| t.id == *id) {
                continue;
            }
            if let Some(tool) = TOOLS.iter().find(|t| t.id == *id) {
                quick_items.push((tool, false));
            }
        }
        if quick_items.is_empty() {
            return None;
        }

        let mut picked = None;
        ui.add_space(18.0);
        egui::Frame::new()
            .fill(p.surface)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    ui.label(
                        RichText::new(format!("{} 快捷访问", icons::LIGHTNING))
                            .size(12.0)
                            .strong()
                            .color(p.muted),
                    );
                    for (tool, is_fav) in quick_items {
                        let accent = group_accent(tool.group, p.dark);
                        let prefix = if is_fav {
                            icons::STAR
                        } else {
                            icons::CLOCK_COUNTER_CLOCKWISE
                        };
                        let label_color = if is_fav {
                            Color32::from_rgb(245, 158, 11)
                        } else {
                            accent.fg
                        };
                        let mut job = egui::text::LayoutJob::default();
                        job.append(
                            prefix,
                            0.0,
                            egui::TextFormat::simple(FontId::proportional(12.0), label_color),
                        );
                        job.append(
                            &format!(" {}", tool.name),
                            0.0,
                            egui::TextFormat::simple(FontId::proportional(12.5), p.ink),
                        );
                        let resp = ui.add(
                            egui::Button::new(job)
                                .fill(p.surface_subtle)
                                .stroke(Stroke::new(1.0_f32, p.border))
                                .corner_radius(CornerRadius::same(99))
                                .min_size(Vec2::new(0.0, 28.0)),
                        );
                        if resp.clicked() {
                            picked = Some(tool.id);
                        }
                    }
                });
            });
        picked
    }

    fn render_dashboard_filters(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            let all_selected = self.selected_group.is_none();
            if ui
                .add(filter_pill(
                    format!("{} 全部  {}", icons::SQUARES_FOUR, TOOLS.len()),
                    all_selected,
                    p.accent,
                    p.accent_soft,
                    &p,
                ))
                .clicked()
            {
                self.selected_group = None;
            }
            for group in Group::ALL {
                let count = TOOLS.iter().filter(|tool| tool.group == group).count();
                let accent = group_accent(group, p.dark);
                if ui
                    .add(filter_pill(
                        format!("{} {}  {}", accent.icon, group.title(), count),
                        self.selected_group == Some(group),
                        accent.fg,
                        accent.bg,
                        &p,
                    ))
                    .clicked()
                {
                    self.selected_group = Some(group);
                }
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 68.0).clamp(200.0, 460.0);
            egui::Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0_f32, p.border_strong))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(12, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(icons::MAGNIFYING_GLASS)
                                .size(15.0)
                                .color(p.muted),
                        );
                        ui.add_sized(
                            [width - 42.0, 28.0],
                            egui::TextEdit::singleline(&mut self.dashboard_query)
                                .frame(false)
                                .vertical_align(Align::Center)
                                .hint_text("搜索工具名称、分类或说明..."),
                        );
                        if !self.dashboard_query.is_empty()
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(icons::X).size(13.0).color(p.muted),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("清空搜索")
                                .clicked()
                        {
                            self.dashboard_query.clear();
                        }
                    });
                });
        });
    }
}

pub(crate) fn matches_query(tool: &ToolDef, query: &str) -> bool {
    query.is_empty()
        || [tool.name, tool.group.title(), tool.description, tool.icon]
            .iter()
            .any(|text| text.to_lowercase().contains(query))
}

fn filter_pill(
    text: String,
    selected: bool,
    fg: Color32,
    bg: Color32,
    p: &theme::Palette,
) -> egui::Button<'static> {
    let text_color = if selected { fg } else { p.ink_secondary };
    egui::Button::new(RichText::new(text).size(12.5).color(text_color))
        .selected(selected)
        .min_size(Vec2::new(0.0, 32.0))
        .fill(if selected { bg } else { p.surface })
        .stroke(Stroke::new(
            1.0_f32,
            if selected {
                fg.gamma_multiply(0.45)
            } else {
                p.border
            },
        ))
        .corner_radius(CornerRadius::same(99))
}

struct CardText {
    title: Arc<Galley>,
    category: Arc<Galley>,
    description: Arc<Galley>,
}

impl CardText {
    fn new(ui: &egui::Ui, tool: &ToolDef, width: f32, p: &theme::Palette) -> Self {
        let title_width = (width - 54.0 - 26.0).max(1.0);
        let layout = |text: RichText, width| {
            WidgetText::from(text).into_galley(ui, Some(TextWrapMode::Wrap), width, TextStyle::Body)
        };
        Self {
            title: layout(
                RichText::new(tool.name).size(14.5).strong().color(p.ink),
                title_width,
            ),
            category: layout(
                RichText::new(tool.group.title()).size(11.0).color(p.muted),
                title_width,
            ),
            description: layout(
                RichText::new(tool.description).size(12.0).color(p.muted),
                width,
            ),
        }
    }

    fn header_height(&self) -> f32 {
        (self.title.size().y + 3.0 + self.category.size().y).max(40.0)
    }
}

struct CardOutcome {
    opened: bool,
    toggled_favorite: bool,
}

fn tool_card(
    ui: &mut egui::Ui,
    tool: &ToolDef,
    text: &CardText,
    header_height: f32,
    description_height: f32,
    is_favorite: bool,
    p: &theme::Palette,
) -> CardOutcome {
    let accent = group_accent(tool.group, p.dark);
    let total_height =
        CARD_PADDING as f32 * 2.0 + header_height + 10.0 + description_height + 12.0 + 22.0;
    let width = ui.available_width();
    let (card_rect, response) = ui.allocate_exact_size(Vec2::new(width, total_height), Sense::click());
    let hovered = response.hovered();

    let inner = card_rect.shrink(CARD_PADDING as f32);
    let star_rect = egui::Rect::from_center_size(
        egui::pos2(inner.right() - 10.0, inner.top() + 12.0),
        Vec2::splat(26.0),
    );
    let star_hovered = ui
        .ctx()
        .pointer_latest_pos()
        .is_some_and(|pos| star_rect.contains(pos));

    // 悬停轻阴影与卡片边框高亮
    if hovered {
        ui.painter()
            .add(p.shadow.as_shape(card_rect, CornerRadius::same(12)));
    }
    ui.painter().rect_filled(
        card_rect,
        CornerRadius::same(12),
        if hovered {
            if p.dark {
                p.surface_hover
            } else {
                Color32::WHITE
            }
        } else {
            p.surface
        },
    );
    ui.painter().rect_stroke(
        card_rect,
        CornerRadius::same(12),
        Stroke::new(
            if hovered { 1.3_f32 } else { 1.0_f32 },
            if hovered { accent.fg } else { p.border },
        ),
        egui::StrokeKind::Inside,
    );

    // 左上角分类专属色矢量图标块
    let icon_rect = egui::Rect::from_min_size(inner.min, Vec2::splat(40.0));
    ui.painter()
        .rect_filled(icon_rect, CornerRadius::same(10), accent.bg);
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        tool_icon(tool),
        FontId::proportional(20.0),
        accent.fg,
    );

    // 标题与分类名
    let title_x = icon_rect.right() + 12.0;
    let title_y =
        inner.top() + (header_height - (text.title.size().y + 3.0 + text.category.size().y)) / 2.0;
    ui.painter()
        .galley(egui::pos2(title_x, title_y), text.title.clone(), p.ink);
    ui.painter().galley(
        egui::pos2(title_x, title_y + text.title.size().y + 3.0),
        text.category.clone(),
        p.muted,
    );

    // 右上角星标收藏按钮
    if is_favorite || hovered {
        if star_hovered {
            ui.painter()
                .rect_filled(star_rect, CornerRadius::same(6), p.surface_hover);
        }
        ui.painter().text(
            star_rect.center(),
            Align2::CENTER_CENTER,
            icons::STAR,
            FontId::proportional(14.5),
            if is_favorite {
                Color32::from_rgb(245, 158, 11)
            } else if star_hovered {
                p.ink
            } else {
                p.muted
            },
        );
    }

    // 简介文本
    let desc_y = inner.top() + header_height + 10.0;
    ui.painter()
        .galley(egui::pos2(inner.left(), desc_y), text.description.clone(), p.muted);

    // 底部清爽元信息行：左侧符号小徽章 + 右侧柔和箭头
    let footer_y = inner.bottom() - 10.0;
    if !tool.icon.is_empty() {
        let badge_text = format!("符号 {}", tool.icon);
        let badge_galley = ui.painter().layout_no_wrap(
            badge_text,
            FontId::proportional(11.0),
            accent.fg,
        );
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(inner.left(), footer_y - 10.0),
            Vec2::new(badge_galley.size().x + 12.0, 20.0),
        );
        ui.painter()
            .rect_filled(badge_rect, CornerRadius::same(6), accent.bg);
        ui.painter().galley(
            egui::pos2(
                badge_rect.left() + 6.0,
                badge_rect.center().y - badge_galley.size().y / 2.0,
            ),
            badge_galley,
            accent.fg,
        );
    }
    let arrow_text = if hovered {
        format!("打开 {}", icons::ARROW_RIGHT)
    } else {
        icons::ARROW_RIGHT.to_owned()
    };
    ui.painter().text(
        egui::pos2(inner.right(), footer_y),
        Align2::RIGHT_CENTER,
        arrow_text,
        FontId::proportional(12.0),
        if hovered { accent.fg } else { p.muted.gamma_multiply(0.75) },
    );

    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut opened = false;
    let mut toggled_favorite = false;
    if response.clicked() {
        if response
            .interact_pointer_pos()
            .is_some_and(|pos| star_rect.contains(pos))
        {
            toggled_favorite = true;
        } else {
            opened = true;
        }
    }

    CardOutcome {
        opened,
        toggled_favorite,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_search_matches_name_group_and_description() {
        let pressure = TOOLS
            .iter()
            .find(|tool| tool.id == "unit_pressure")
            .unwrap();
        assert!(matches_query(pressure, ""));
        assert!(matches_query(pressure, "单位换算"));
        assert!(matches_query(pressure, "压力"));
        assert!(matches_query(pressure, "常用单位"));
        assert!(!matches_query(pressure, "辐射"));
    }
}
