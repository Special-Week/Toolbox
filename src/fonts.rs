//! 保留 egui 默认拉丁字体，并依次追加系统中文、符号与数学字体作为回退。
//! 字体文件缺失时跳过该项，不让特定系统字体成为启动的硬性依赖。
use eframe::egui::{self, FontData, FontDefinitions};
use std::path::Path;

pub(crate) fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    #[cfg(target_os = "windows")]
    {
        let directory = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| r"C:\Windows".into())
            .join("Fonts");
        for filename in ["msyh.ttc", "Deng.ttf", "simhei.ttf"] {
            if append_font(&mut fonts, "system-cjk", &directory.join(filename), 0) {
                break;
            }
        }
        // 中文字体通常不覆盖全部数学下标和特殊符号，分别添加完整的回退字体。
        append_font(
            &mut fonts,
            "system-symbols",
            &directory.join("seguisym.ttf"),
            0,
        );
        append_font(&mut fonts, "system-math", &directory.join("cambria.ttc"), 1);
        append_font(
            &mut fonts,
            "system-latin",
            &directory.join("segoeui.ttf"),
            0,
        );
    }

    #[cfg(target_os = "linux")]
    {
        for path in [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ] {
            if append_font(&mut fonts, "system-cjk", Path::new(path), 0) {
                break;
            }
        }
        append_font(
            &mut fonts,
            "system-symbols",
            Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            0,
        );
    }

    #[cfg(target_os = "macos")]
    {
        for path in [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ] {
            if append_font(&mut fonts, "system-cjk", Path::new(path), 0) {
                break;
            }
        }
        append_font(
            &mut fonts,
            "system-symbols",
            Path::new("/System/Library/Fonts/Apple Symbols.ttf"),
            0,
        );
    }

    ctx.set_fonts(fonts);
}

fn append_font(fonts: &mut FontDefinitions, name: &str, path: &Path, index: u32) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let mut data = FontData::from_owned(bytes);
    data.index = index;
    fonts.font_data.insert(name.to_owned(), data.into());
    for family in fonts.families.values_mut() {
        family.push(name.to_owned());
    }
    true
}
