#pragma once
#include <memory>
#include <string>
#include <vector>

namespace toolbox {
struct MathBitmap {
    unsigned width = 0, height = 0;
    std::vector<unsigned char> pixels;
};

// 返回预乘透明度的 BGRA 图像，排版失败时返回空指针。
std::shared_ptr<const MathBitmap> renderMath(const std::wstring &latex);
}
