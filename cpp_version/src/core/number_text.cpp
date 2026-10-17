// 数值与十进制文本的唯一转换入口，避免不同 Windows CRT 的解析误差污染默认值和计算。
// 短文本保留原 double 的全部有效信息；固定小数位只用于结果展示，不修改模型中的数值。
#include "double-conversion/double-conversion.h"
#include "toolbox/core.hpp"

namespace toolbox {
using double_conversion::DoubleToStringConverter;
using double_conversion::StringBuilder;
using double_conversion::StringToDoubleConverter;

double parse(const std::string &text) {
    constexpr const char *spaces = " \t\r\n\f\v";
    auto first = text.find_first_not_of(spaces);
    require(first != std::string::npos, "请填写数值");
    auto length = text.find_last_not_of(spaces) - first + 1;
    require(length <= static_cast<std::size_t>(std::numeric_limits<int>::max()), "输入数字过长");
    // 不允许尾随字符、十六进制或按区域设置解释逗号；支持普通小数、指数和次正规数。
    static const StringToDoubleConverter converter(
        StringToDoubleConverter::NO_FLAGS, std::numeric_limits<double>::quiet_NaN(),
        std::numeric_limits<double>::quiet_NaN(), nullptr, nullptr);
    int consumed = 0;
    double value = converter.StringToDouble(text.data() + first, static_cast<int>(length), &consumed);
    require(consumed == static_cast<int>(length) && !std::isnan(value), "请输入有效的十进制数字");
    require(std::isfinite(value), "数值必须为有限数字");
    return value;
}

// 科学计数只作为内部转换的中间形式，界面和 CSV 均展开为普通十进制。
static std::string plain(std::string text) {
    auto exponentAt = text.find_first_of("eE");
    if (exponentAt == std::string::npos) {
        return text;
    }
    int exponent = std::stoi(text.substr(exponentAt + 1));
    std::string digits = text.substr(0, exponentAt), sign;
    if (digits.front() == '-') {
        sign = "-";
        digits.erase(0, 1);
    }
    auto dot = digits.find('.');
    int position = dot == std::string::npos ? static_cast<int>(digits.size()) : static_cast<int>(dot);
    if (dot != std::string::npos) {
        digits.erase(dot, 1);
    }
    position += exponent;
    while (digits.size() > 1 && digits.back() == '0') {
        digits.pop_back();
    }
    if (position <= 0) {
        return sign + "0." + std::string(-position, '0') + digits;
    }
    if (position >= static_cast<int>(digits.size())) {
        return sign + digits + std::string(position - digits.size(), '0');
    }
    digits.insert(position, ".");
    return sign + digits;
}

std::string rawNumber(double value) {
    finite({value});
    if (value == 0) {
        return "0";
    }
    char buffer[128];
    StringBuilder builder(buffer, sizeof(buffer));
    require(DoubleToStringConverter::EcmaScriptConverter().ToShortest(value, &builder), "无法格式化数字");
    return plain(builder.Finalize());
}

std::wstring inputNumber(double value) {
    // 不使用“输出一串数字再调用 strtod 比较”的循环，10.03、0.1 等默认值始终保留短写法。
    return wide(rawNumber(value));
}

static std::string displayDecimal(double value, int digits, bool exponential) {
    char buffer[192];
    StringBuilder builder(buffer, sizeof(buffer));
    const auto &converter = DoubleToStringConverter::EcmaScriptConverter();
    bool success = exponential ? converter.ToExponential(value, digits, &builder)
                               : converter.ToFixed(value, digits, &builder);
    require(success, "无法格式化计算结果");
    return plain(builder.Finalize());
}

std::wstring format(double value) {
    if (!std::isfinite(value)) {
        return L"—";
    }
    if (value == 0) {
        return L"0";
    }
    bool exponential = std::abs(value) >= 1e10 || std::abs(value) < 1e-4;
    auto text = displayDecimal(value, exponential ? 10 : 8, exponential);
    if (!exponential) {
        while (text.back() == '0') {
            text.pop_back();
        }
        if (text.back() == '.') {
            text.pop_back();
        }
    }
    return text == "-0" ? L"0" : wide(text);
}

std::wstring formatUnit(double value) {
    if (!std::isfinite(value)) {
        return L"—";
    }
    if (value == 0) {
        return L"0.0000000";
    }
    if (std::abs(value) >= 1e9 || std::abs(value) < 1e-4) {
        return format(value);
    }
    return wide(displayDecimal(value, 7, false));
}
} // namespace toolbox
