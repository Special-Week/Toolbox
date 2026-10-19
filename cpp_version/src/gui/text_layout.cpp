// 用 DirectWrite 的实际换行结果测量段落，避免按字符数估高造成大块空白或文字重叠。
#include "toolbox/gui.hpp"

namespace toolbox {
float App::textWidth(const std::wstring &text, float size) const {
    if (text.empty()) {
        return 0;
    }
    IDWriteTextFormat *format = nullptr;
    IDWriteTextLayout *layout = nullptr;
    HRESULT status = textFactory_->CreateTextFormat(L"Microsoft YaHei", nullptr, DWRITE_FONT_WEIGHT_NORMAL,
                                                    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                                                    size, L"zh-cn", &format);
    if (SUCCEEDED(status)) {
        format->SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
        status = textFactory_->CreateTextLayout(text.c_str(), UINT32(text.size()), format,
                                                100000.f, 100000.f, &layout);
    }
    DWRITE_TEXT_METRICS metrics{};
    if (SUCCEEDED(status)) {
        status = layout->GetMetrics(&metrics);
    }
    if (layout) {
        layout->Release();
    }
    if (format) {
        format->Release();
    }
    require(SUCCEEDED(status), "无法计算图表文字的排版宽度");
    return std::ceil(metrics.widthIncludingTrailingWhitespace);
}

float App::paragraphHeight(const std::wstring &text, float availableWidth, float size) const {
    IDWriteTextFormat *format = nullptr;
    IDWriteTextLayout *layout = nullptr;
    HRESULT status = textFactory_->CreateTextFormat(L"Microsoft YaHei", nullptr, DWRITE_FONT_WEIGHT_NORMAL,
                                                    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                                                    size, L"zh-cn", &format);
    if (SUCCEEDED(status)) {
        format->SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, paragraphLineSpacing(size), size * 1.25f);
        status = textFactory_->CreateTextLayout(text.c_str(), UINT32(text.size()), format,
                                                std::max(1.f, availableWidth), 100000.f, &layout);
    }
    DWRITE_TEXT_METRICS metrics{};
    if (SUCCEEDED(status)) {
        status = layout->GetMetrics(&metrics);
    }
    if (layout) {
        layout->Release();
    }
    if (format) {
        format->Release();
    }
    require(SUCCEEDED(status), "无法计算说明文字的排版高度");
    return std::ceil(std::max(paragraphLineSpacing(size), metrics.height)) + 2;
}
} // namespace toolbox
