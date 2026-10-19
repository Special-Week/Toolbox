//! 桌面入口：配置窗口、字体和主题，再交由 ToolboxApp 管理页面状态。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod calculators;
mod catalog;
mod fonts;
mod icon;
mod navigation;
mod units;

use crate::app::ToolboxApp;
use crate::fonts::configure_fonts;
use eframe::egui::{self, Color32, Vec2};

pub(crate) const BLUE: Color32 = Color32::from_rgb(39, 105, 220);
pub(crate) const BLUE_SOFT: Color32 = Color32::from_rgb(234, 242, 255);
pub(crate) const INK: Color32 = Color32::from_rgb(35, 48, 67);
pub(crate) const MUTED: Color32 = Color32::from_rgb(111, 126, 146);
pub(crate) const PAGE: Color32 = Color32::from_rgb(246, 248, 251);
pub(crate) const GREEN: Color32 = Color32::from_rgb(29, 139, 97);
pub(crate) const RED: Color32 = Color32::from_rgb(183, 72, 70);

fn main() -> eframe::Result<()> {
    // 发布版在 Windows 上使用 GUI 子系统，启动时不额外弹出控制台窗口。
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon::data().clone())
            .with_inner_size([1220.0, 820.0])
            .with_min_inner_size([620.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "工具箱 · 桌面版",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            configure_style(&cc.egui_ctx);
            Ok(Box::<ToolboxApp>::default())
        }),
    )
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.visuals = egui::Visuals::light();
    style.visuals.widgets.noninteractive.bg_fill = Color32::WHITE;
    style.visuals.widgets.noninteractive.fg_stroke.color = INK;
    style.visuals.widgets.inactive.bg_fill = Color32::WHITE;
    style.visuals.widgets.inactive.fg_stroke.color = INK;
    style.visuals.widgets.hovered.bg_fill = BLUE_SOFT;
    style.visuals.widgets.hovered.fg_stroke.color = BLUE;
    style.visuals.widgets.active.bg_fill = BLUE;
    style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    style.visuals.selection.bg_fill = BLUE;
    style.visuals.selection.stroke.color = Color32::WHITE;
    ctx.set_style(style);
}
