//! 侧栏分类与工具项的统一绘制；展开箭头由线段构成，不依赖特殊字体字形。
use crate::{BLUE, BLUE_SOFT, INK};
use eframe::egui::{
    self, Align2, Color32, FontId, Response, RichText, Sense, Stroke, TextStyle, TextWrapMode,
    WidgetInfo, WidgetText, WidgetType,
};

pub(crate) fn category(
    ui: &mut egui::Ui,
    title: &str,
    count: usize,
    collapsed: bool,
    active: bool,
) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), title));
    let foreground = if active { BLUE } else { INK };
    ui.painter().rect_filled(
        rect,
        6,
        if active || response.hovered() {
            BLUE_SOFT
        } else {
            Color32::from_rgb(240, 244, 249)
        },
    );
    let galley = WidgetText::from(RichText::new(title).size(15.5).strong().color(foreground))
        .into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (rect.width() - 76.0).max(1.0),
            TextStyle::Body,
        );
    let position = egui::pos2(rect.left() + 11.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(position, galley, foreground);

    let count_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 42.0, rect.center().y),
        egui::vec2(26.0, 22.0),
    );
    ui.painter().rect_filled(count_rect, 11, Color32::WHITE);
    ui.painter().text(
        count_rect.center(),
        Align2::CENTER_CENTER,
        count.to_string(),
        FontId::proportional(11.5),
        foreground,
    );
    let center = egui::pos2(rect.right() - 14.0, rect.center().y);
    let vertices = if collapsed {
        [
            center + egui::vec2(-2.0, -4.0),
            center + egui::vec2(2.0, 0.0),
            center + egui::vec2(-2.0, 4.0),
        ]
    } else {
        [
            center + egui::vec2(-4.0, -2.0),
            center + egui::vec2(0.0, 2.0),
            center + egui::vec2(4.0, -2.0),
        ]
    };
    ui.painter().add(egui::Shape::line(
        vertices.to_vec(),
        Stroke::new(1.6_f32, foreground),
    ));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn tool(ui: &mut egui::Ui, name: &str, selected: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 37.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), name));
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            5,
            if selected {
                BLUE_SOFT
            } else {
                Color32::from_rgb(246, 248, 252)
            },
        );
    }
    let rail_x = rect.left() + 12.0;
    ui.painter().line_segment(
        [
            egui::pos2(rail_x, rect.top() - 2.0),
            egui::pos2(rail_x, rect.bottom() + 2.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(221, 229, 239)),
    );
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(rail_x, rect.center().y),
                egui::vec2(3.0, 22.0),
            ),
            1,
            BLUE,
        );
    }
    let foreground = if selected {
        BLUE
    } else {
        Color32::from_rgb(66, 82, 104)
    };
    let text = RichText::new(name).size(13.0).color(foreground);
    let galley = WidgetText::from(if selected { text.strong() } else { text }).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        (rect.width() - 38.0).max(1.0),
        TextStyle::Body,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0),
        galley,
        foreground,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
