// 将多尺寸图标与版本信息嵌入 Windows EXE，不依赖运行目录中的外部图片。
fn main() {
    println!("cargo:rerun-if-changed=assets/toolbox.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/toolbox.ico")
            .set("FileDescription", "工具箱")
            .set("ProductName", "Toolbox")
            .set("OriginalFilename", "Toolbox.exe")
            .compile()
            .expect("无法编译程序图标，请确认 Windows SDK 或 MinGW 资源编译器已安装");
    }
}
