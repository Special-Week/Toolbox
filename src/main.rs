//! 桌面入口：配置窗口、字体和主题，再交由 ToolboxApp 管理页面状态。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod calculators;
pub(crate) use toolbox_core::{catalog, units};
mod fonts;
mod icon;
mod navigation;
mod pdf_download;
mod theme;

use crate::app::ToolboxApp;
use crate::fonts::configure_fonts;
use eframe::egui::{self, Color32};

pub(crate) const BLUE: Color32 = theme::LIGHT.accent;
pub(crate) const BLUE_SOFT: Color32 = theme::LIGHT.accent_soft;
pub(crate) const INK: Color32 = theme::LIGHT.ink;
pub(crate) const MUTED: Color32 = theme::LIGHT.muted;
pub(crate) const RED: Color32 = theme::LIGHT.danger;

fn main() -> eframe::Result<()> {
    // 发布版在 Windows 上使用 GUI 子系统，启动时不额外弹出控制台窗口。
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(concat!("工具箱 · 桌面版 v", env!("CARGO_PKG_VERSION")))
            .with_icon(icon::data().clone())
            .with_inner_size([1280.0, 840.0])
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

pub(crate) fn configure_style(ctx: &egui::Context) {
    theme::apply_theme(ctx, theme::ThemeMode::Light);
}
