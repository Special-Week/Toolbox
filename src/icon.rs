//! 窗口与页眉共用内嵌图标；只解码一次，避免每帧读取或解析图片。
use eframe::egui::IconData;
use std::sync::OnceLock;

pub(crate) fn data() -> &'static IconData {
    static ICON: OnceLock<IconData> = OnceLock::new();
    ICON.get_or_init(|| {
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/toolbox.png"))
            .expect("内嵌工具箱图标应为有效 PNG")
    })
}
