#include "toolbox/math_engine.hpp"
#include <windows.h>
#include <gdiplus.h>
#include "latex.h"
#include "platform/gdi_win/graphic_win32.h"
#include <algorithm>
#include <cstring>
#include <map>

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

std::shared_ptr<const MathBitmap> renderMath(const std::wstring &latex) {
    static Engine engine;
    static std::map<std::wstring, std::shared_ptr<const MathBitmap>> cache;
    if (!engine.ready || latex.empty()) return {};
    const auto existing = cache.find(latex);
    if (existing != cache.end()) return existing->second;
    try {
        // 两倍分辨率供 Direct2D 在常见 DPI 下缩放。
        std::unique_ptr<tex::TeXRender> formula(tex::LaTeX::parse(latex, 16384, 28.f, 0.f, 0xff233043));
        if (!formula || formula->getWidth() <= 0 || formula->getHeight() <= 0 ||
            formula->getWidth() > 8192 || formula->getHeight() > 1024) return {};
        const int width = formula->getWidth() + 4, height = formula->getHeight() + 4;
        Gdiplus::Bitmap bitmap(width, height, PixelFormat32bppPARGB);
        if (bitmap.GetLastStatus() != Gdiplus::Ok) return {};
        {
            Gdiplus::Graphics graphics(&bitmap);
            graphics.Clear(Gdiplus::Color(0, 0, 0, 0));
            tex::Graphics2D_win32 canvas(&graphics);
            formula->draw(canvas, 2, 2);
        }
        Gdiplus::Rect bounds(0, 0, width, height);
        Gdiplus::BitmapData data{};
        if (bitmap.LockBits(&bounds, Gdiplus::ImageLockModeRead, PixelFormat32bppPARGB, &data) != Gdiplus::Ok) return {};
        auto image = std::make_shared<MathBitmap>();
        image->width = static_cast<unsigned>(width);
        image->height = static_cast<unsigned>(height);
        image->pixels.resize(static_cast<std::size_t>(width) * height * 4);
        for (int y = 0; y < height; ++y) {
            const auto *source = static_cast<const unsigned char *>(data.Scan0) + y * data.Stride;
            std::memcpy(image->pixels.data() + static_cast<std::size_t>(y) * width * 4, source, width * 4);
        }
        bitmap.UnlockBits(&data);
        if (cache.size() >= 128) cache.erase(cache.begin());
        cache.emplace(latex, image);
        return image;
    } catch (...) {
        return {};
    }
}
}
