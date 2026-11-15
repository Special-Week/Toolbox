#include "toolbox/math_engine.hpp"
#include <windows.h>
#include <gdiplus.h>
#include "latex.h"
#include "platform/gdi_win/graphic_win32.h"
#include <algorithm>
#include <array>
#include <cmath>
#include <map>
#include <tuple>

namespace toolbox {
namespace {
struct Engine {
    ULONG_PTR token = 0;
    bool ready = false;
    Engine() {
        Gdiplus::GdiplusStartupInput input;
        if (Gdiplus::GdiplusStartup(&token, &input, nullptr) == Gdiplus::Ok) {
            try {
                tex::LaTeX::init();
                ready = true;
            } catch (...) {
                Gdiplus::GdiplusShutdown(token);
                token = 0;
            }
        }
    }
    ~Engine() {
        if (ready) tex::LaTeX::release();
        if (token) Gdiplus::GdiplusShutdown(token);
    }
};
}

std::shared_ptr<const MathBitmap> renderMath(const std::wstring &latex, float scale, float pixelScale) {
    static Engine engine;
    static std::map<std::tuple<std::wstring, float, float>, std::shared_ptr<const MathBitmap>> cache;
    if (!engine.ready || latex.empty() || !std::isfinite(scale) || !std::isfinite(pixelScale) ||
        scale < 1.f || scale > 6.f || pixelScale < .25f || pixelScale > 8.f) return {};
    const auto key = std::make_tuple(latex, scale, pixelScale);
    const auto existing = cache.find(key);
    if (existing != cache.end()) return existing->second;
    try {
        // 按显示器 DPI 提高采样密度，布局尺寸仍由调用方按比例还原。
        auto render = [&](float density) {
            return std::unique_ptr<tex::TeXRender>(tex::LaTeX::parse(latex, 16384, 16.f * density, 0.f, 0xff233043));
        };
        auto formula = render(scale);
        // 超长公式退回原有采样密度，避免提高分辨率后无法显示。
        if (formula && (formula->getWidth() > 8192 || formula->getHeight() > 1024) && scale > 2.f) {
            scale = 2.f;
            formula = render(scale);
        }
        if (!formula || formula->getWidth() <= 0 || formula->getHeight() <= 0 ||
            formula->getWidth() > 8192 || formula->getHeight() > 1024) return {};
        const int padding = static_cast<int>(std::ceil(scale));
        const int width = formula->getWidth() + 2 * padding, height = formula->getHeight() + 2 * padding;
        Gdiplus::Bitmap bitmap(width, height, PixelFormat32bppPARGB);
        if (bitmap.GetLastStatus() != Gdiplus::Ok) return {};
        {
            Gdiplus::Graphics graphics(&bitmap);
            graphics.Clear(Gdiplus::Color(0, 0, 0, 0));
            tex::Graphics2D_win32 canvas(&graphics);
            formula->draw(canvas, padding, padding);
        }
        // 先将高分辨率图像缩到屏幕像素，避免 Direct2D 再次缩放冲淡细线。
        const float finalScale = std::min(pixelScale, scale);
        const int pixelWidth = std::max(1, static_cast<int>(std::ceil(width * finalScale / scale)));
        const int pixelHeight = std::max(1, static_cast<int>(std::ceil(height * finalScale / scale)));
        Gdiplus::Bitmap screenBitmap(pixelWidth, pixelHeight, PixelFormat32bppPARGB);
        if (screenBitmap.GetLastStatus() != Gdiplus::Ok) return {};
        {
            Gdiplus::Graphics graphics(&screenBitmap);
            graphics.SetCompositingMode(Gdiplus::CompositingModeSourceCopy);
            graphics.SetInterpolationMode(Gdiplus::InterpolationModeHighQualityBilinear);
            graphics.SetPixelOffsetMode(Gdiplus::PixelOffsetModeHalf);
            if (graphics.DrawImage(&bitmap, Gdiplus::Rect(0, 0, pixelWidth, pixelHeight),
                                   0, 0, width, height, Gdiplus::UnitPixel) != Gdiplus::Ok) return {};
        }
        Gdiplus::Rect bounds(0, 0, pixelWidth, pixelHeight);
        Gdiplus::BitmapData data{};
        if (screenBitmap.LockBits(&bounds, Gdiplus::ImageLockModeRead, PixelFormat32bppPARGB, &data) != Gdiplus::Ok) return {};
        auto image = std::make_shared<MathBitmap>();
        image->width = static_cast<unsigned>(pixelWidth);
        image->height = static_cast<unsigned>(pixelHeight);
        image->scale = finalScale;
        image->pixels.resize(static_cast<std::size_t>(pixelWidth) * pixelHeight * 4);
        // 保留预乘透明度，在最终像素上增强细笔画的对比度。
        static const auto coverage = [] {
            std::array<unsigned char, 256> result{};
            for (int i = 1; i < 256; ++i) {
                result[i] = static_cast<unsigned char>(std::lround(255.0 * std::pow(i / 255.0, 0.7)));
            }
            return result;
        }();
        for (int y = 0; y < pixelHeight; ++y) {
            const auto *source = static_cast<const unsigned char *>(data.Scan0) + y * data.Stride;
            auto *dest = image->pixels.data() + static_cast<std::size_t>(y) * pixelWidth * 4;
            for (int x = 0; x < pixelWidth; ++x) {
                const unsigned alpha = source[x * 4 + 3], adjusted = coverage[alpha];
                for (int channel = 0; channel < 3; ++channel) {
                    const unsigned value = alpha ? (source[x * 4 + channel] * adjusted + alpha / 2) / alpha : 0;
                    dest[x * 4 + channel] = static_cast<unsigned char>(std::min(adjusted, value));
                }
                dest[x * 4 + 3] = static_cast<unsigned char>(adjusted);
            }
        }
        screenBitmap.UnlockBits(&data);
        if (cache.size() >= 128) cache.erase(cache.begin());
        cache.emplace(key, image);
        return image;
    } catch (...) {
        return {};
    }
}
}
