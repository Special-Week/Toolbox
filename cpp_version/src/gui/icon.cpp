// 从 EXE 图标资源生成页眉位图。WIC 与 Direct2D 均使用 Windows 7 支持的接口。
#include "toolbox/gui.hpp"
#include "toolbox/resource_ids.h"
#include <wincodec.h>

namespace toolbox {
void App::loadIconBitmap() {
    // 位图属于当前渲染目标；设备丢失或 DPI 改变后由窗口代码释放并重新创建。
    if (!target_ || iconBitmap_) {
        return;
    }
    auto icon = static_cast<HICON>(LoadImageW(GetModuleHandleW(nullptr), MAKEINTRESOURCEW(TOOLBOX_ICON),
                                             IMAGE_ICON, 256, 256, 0));
    if (!icon) {
        return;
    }
    IWICImagingFactory *imaging = nullptr;
    IWICBitmap *source = nullptr;
    IWICFormatConverter *converter = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_WICImagingFactory, nullptr, CLSCTX_INPROC_SERVER,
                                    IID_IWICImagingFactory, reinterpret_cast<void **>(&imaging))) &&
        SUCCEEDED(imaging->CreateBitmapFromHICON(icon, &source)) &&
        SUCCEEDED(imaging->CreateFormatConverter(&converter)) &&
        SUCCEEDED(converter->Initialize(source, GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone,
                                         nullptr, 0, WICBitmapPaletteTypeCustom))) {
        // 预乘透明度可避免透明圆角在浅色背景上出现黑边。
        target_->CreateBitmapFromWicBitmap(converter, nullptr, &iconBitmap_);
    }
    if (converter) {
        converter->Release();
    }
    if (source) {
        source->Release();
    }
    if (imaging) {
        imaging->Release();
    }
    DestroyIcon(icon);
}
} // namespace toolbox
