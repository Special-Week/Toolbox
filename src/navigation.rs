//! 侧栏分类、工具项与折叠态按钮的现代清爽绘制；统一接入深浅主题与 Phosphor 矢量图标。
use crate::catalog::Group;
use crate::theme::{self, group_accent};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Response, RichText, Sense, Stroke, TextStyle,
    TextWrapMode, Vec2, WidgetInfo, WidgetText, WidgetType,
};
use egui_phosphor::regular as icons;

pub(crate) fn category(
    ui: &mut egui::Ui,
    group: Group,
    count: usize,
    collapsed: bool,
    active: bool,
) -> Response {
    let p = theme::palette(ui.ctx());
    let accent = group_accent(group, p.dark);
    let title = group.title();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), title));

    let fill = if active {
        accent.bg
    } else if response.hovered() {
        p.surface_hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    }

    // 分类专属色圆角图标徽章
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        Vec2::splat(24.0),
    );
    ui.painter()
        .rect_filled(icon_rect, CornerRadius::same(6), accent.bg);
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        accent.icon,
        FontId::proportional(14.0),
        accent.fg,
    );

    let foreground = if active { accent.fg } else { p.ink };
    let galley = WidgetText::from(RichText::new(title).size(13.5).strong().color(foreground))
        .into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (rect.width() - 86.0).max(1.0),
            TextStyle::Body,
        );
    let position = egui::pos2(rect.left() + 38.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(position, galley, foreground);

    // 现代轻量数字胶囊
    let count_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 34.0, rect.center().y),
        egui::vec2(24.0, 19.0),
    );
    ui.painter().rect_filled(
        count_rect,
        CornerRadius::same(10),
        if active { p.surface } else { p.surface_subtle },
    );
    ui.painter().text(
        count_rect.center(),
        Align2::CENTER_CENTER,
        count.to_string(),
        FontId::monospace(11.0),
        if active { accent.fg } else { p.muted },
    );

    // 折叠/展开箭头
    let arrow = if collapsed {
        icons::CARET_RIGHT
    } else {
        icons::CARET_DOWN
    };
    ui.painter().text(
        egui::pos2(rect.right() - 12.0, rect.center().y),
        Align2::CENTER_CENTER,
        arrow,
        FontId::proportional(12.0),
        if active { accent.fg } else { p.muted },
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) struct ToolItemOutcome {
    pub(crate) clicked: bool,
    pub(crate) toggled_favorite: bool,
    pub(crate) response: Response,
}

pub(crate) fn tool(
    ui: &mut egui::Ui,
    group: Group,
    name: &str,
    selected: bool,
    favorite: bool,
) -> ToolItemOutcome {
    let p = theme::palette(ui.ctx());
    let accent = group_accent(group, p.dark);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), name));

    let star_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 14.0, rect.center().y),
        Vec2::splat(22.0),
    );
    let pointer_pos = ui.ctx().pointer_latest_pos();
    let star_hovered = pointer_pos.is_some_and(|pos| star_rect.contains(pos));
    let row_hovered = response.hovered();

    if selected || row_hovered {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(7),
            if selected { accent.bg } else { p.surface_hover },
        );
    }

    // 左侧细腻层级引导线与选中高亮竖条
    let rail_x = rect.left() + 18.0;
    ui.painter().line_segment(
        [
            egui::pos2(rail_x, rect.top() - 2.0),
            egui::pos2(rail_x, rect.bottom() + 2.0),
        ],
        Stroke::new(1.0_f32, p.border),
    );
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(rail_x, rect.center().y),
                egui::vec2(3.0, 18.0),
            ),
            CornerRadius::same(2),
            accent.fg,
        );
    }

    let foreground = if selected { accent.fg } else { p.ink_secondary };
    let text = RichText::new(name).size(13.0).color(foreground);
    let galley = WidgetText::from(if selected { text.strong() } else { text }).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        (rect.width() - 56.0).max(1.0),
        TextStyle::Body,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 30.0, rect.center().y - galley.size().y / 2.0),
        galley,
        foreground,
    );

    // 收藏星标：已收藏常显，未收藏悬停时显现
    if favorite || row_hovered {
        let star_color = if favorite {
            Color32::from_rgb(245, 158, 11)
        } else if star_hovered {
            p.ink
        } else {
            p.muted
        };
        ui.painter().text(
            star_rect.center(),
            Align2::CENTER_CENTER,
            icons::STAR,
            FontId::proportional(13.0),
            star_color,
        );
    }

    let mut clicked = false;
    let mut toggled_favorite = false;
    if response.clicked() {
        if response
            .interact_pointer_pos()
            .is_some_and(|pos| star_rect.contains(pos))
        {
            toggled_favorite = true;
        } else {
            clicked = true;
        }
    }

    ToolItemOutcome {
        clicked,
        toggled_favorite,
        response: response.on_hover_cursor(egui::CursorIcon::PointingHand),
    }
}

pub(crate) fn collapsed_group_button(
    ui: &mut egui::Ui,
    group: Group,
    selected: bool,
) -> Response {
    let p = theme::palette(ui.ctx());
    let accent = group_accent(group, p.dark);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(56.0, 48.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), group.title()));

    let fill = if selected {
        accent.bg
    } else if response.hovered() {
        p.surface_hover
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
    }
    if selected {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(1.0_f32, accent.fg.gamma_multiply(0.35)),
            egui::StrokeKind::Inside,
        );
    }

    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 18.0),
        Align2::CENTER_CENTER,
        accent.icon,
        FontId::proportional(18.0),
        if selected || response.hovered() {
            accent.fg
        } else {
            p.ink_secondary
        },
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 10.0),
        Align2::CENTER_CENTER,
        group.short_title(),
        FontId::proportional(10.5),
        if selected { accent.fg } else { p.muted },
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
