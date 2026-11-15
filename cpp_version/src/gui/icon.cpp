// 从 EXE 图标资源生成页眉位图。WIC 与 Direct2D 均使用 Windows 7 支持的接口。
#include "toolbox/gui.hpp"
#include "toolbox/resource_ids.h"
#include "toolbox/math_engine.hpp"
#include <wincodec.h>

namespace toolbox {
ID2D1Bitmap *App::loadDynamicMathBitmap(const std::wstring &formula) {
    if (!target_) return nullptr;
    const auto existing = dynamicMathBitmaps_.find(formula);
    if (existing != dynamicMathBitmaps_.end()) return existing->second;
    const auto image = renderMath(formula);
    if (!image) return nullptr;
    ID2D1Bitmap *bitmap = nullptr;
    const auto properties = D2D1::BitmapProperties(
        D2D1::PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM, D2D1_ALPHA_MODE_PREMULTIPLIED));
    if (FAILED(target_->CreateBitmap(D2D1::SizeU(image->width, image->height), image->pixels.data(),
                                     image->width * 4, properties, &bitmap))) return nullptr;
    if (dynamicMathBitmaps_.size() >= 128) {
        for (auto &entry : dynamicMathBitmaps_) entry.second->Release();
        dynamicMathBitmaps_.clear();
    }
    dynamicMathBitmaps_.emplace(formula, bitmap);
    return bitmap;
}
void App::loadIconBitmap() {
    // 位图属于当前渲染目标；设备丢失或 DPI 改变后由窗口代码释放并重新创建。
    if (!target_ || iconBitmap_) {
        return;
    }
    auto icon = static_cast<HICON>(
        LoadImageW(GetModuleHandleW(nullptr), MAKEINTRESOURCEW(TOOLBOX_ICON), IMAGE_ICON, 256, 256, 0));
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
} // 命名空间 toolbox
