// 数值与十进制文本的唯一转换入口，避免不同 Windows CRT 的解析误差污染默认值和计算。
// 短文本保留原 double 的全部有效信息；固定小数位只用于结果展示，不修改模型中的数值。
#include "double-conversion/double-conversion.h"
#include "toolbox/core.hpp"
#include <cwctype>
#include <map>
#include <regex>
#include <set>

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

std::wstring processNumber(double value, bool fullPrecision) {
    if (!std::isfinite(value))
        return L"—";
    if (fullPrecision)
        return inputNumber(value);
    if (value == 0)
        return L"0";
    char buffer[192];
    StringBuilder builder(buffer, sizeof(buffer));
    require(DoubleToStringConverter::EcmaScriptConverter().ToPrecision(value, 13, &builder),
            "无法格式化计算过程");
    auto text = plain(builder.Finalize());
    if (text.find('.') != std::string::npos) {
        while (text.back() == '0')
            text.pop_back();
        if (text.back() == '.')
            text.pop_back();
    }
    return text == "-0" ? L"0" : wide(text);
}

std::wstring parenthesizeNegativeSubtrahends(const std::wstring &line) {
    static const std::wregex pattern(LR"([-−][ \t]*([-−](?:(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_α-ωΑ-Ω₀-₉]*|√(?:[0-9]+(?:\.[0-9]*)?|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_]*|\([^()]*\))|\([^()]*\))(?:[⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺]+|\^(?:\([^()]*\)|\{[^{}]*\}|[+-]?[0-9]+|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_]*))?))");
    const auto asciiAlnum = [](wchar_t c) { return (c >= L'0' && c <= L'9') || (c >= L'A' && c <= L'Z') || (c >= L'a' && c <= L'z'); };
    const auto greek = [](wchar_t c) { return (c >= L'α' && c <= L'ω') || (c >= L'Α' && c <= L'Ω'); };
    const auto operandEnd = [&](wchar_t c) { return asciiAlnum(c) || greek(c) || (c >= L'₀' && c <= L'₉') || std::wstring(L")]}⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺").find(c) != std::wstring::npos; };
    std::wstring result;
    std::size_t copied = 0;
    for (std::wsregex_iterator it(line.begin(), line.end(), pattern), end; it != end; ++it) {
        const auto &match = *it;
        const auto start = static_cast<std::size_t>(match.position());
        const auto stop = start + static_cast<std::size_t>(match.length());
        auto left = start;
        while (left && (line[left - 1] == L' ' || line[left - 1] == L'\t')) --left;
        if (!left || !operandEnd(line[left - 1])) continue;
        if (line[start] == L'-' && (line[left - 1] == L'e' || line[left - 1] == L'E') && left > 1 &&
            ((line[left - 2] >= L'0' && line[left - 2] <= L'9') || line[left - 2] == L'.')) continue;
        if (stop < line.size() && (asciiAlnum(line[stop]) || line[stop] == L'.' || line[stop] == L'_' || line[stop] == L'^')) continue;
        if (line[start] == L'-') {
            const auto previous = line.find_last_of(L"；。，：\n", start);
            const auto following = line.find_first_of(L"；。，：\n", stop);
            const auto clauseStart = previous == std::wstring::npos ? 0 : previous + 1;
            const auto clause = line.substr(clauseStart, following == std::wstring::npos ? following : following - clauseStart);
            const bool standalone = std::all_of(line.begin(), line.end(), [&](wchar_t c) {
                return asciiAlnum(c) || greek(c) || (c >= L'₀' && c <= L'₉') || c == L' ' || c == L'\t' || std::wstring(L"⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺._+-−×÷*/()[]{}^").find(c) != std::wstring::npos;
            });
            if (clause.find_first_of(L"=≈") == std::wstring::npos && !standalone) continue;
        }
        const auto value = match[1].str();
        const auto valueStart = stop - value.size();
        result += line.substr(copied, valueStart - copied);
        result += L"(" + value + L")";
        copied = stop;
    }
    if (!copied) return line;
    return result + line.substr(copied);
}

std::wstring processDisplayLine(const std::wstring &line, bool fullPrecision) {
    std::wstring raw;
    std::map<std::wstring, std::wstring> shown;
    for (std::size_t pos = 0; pos < line.size();) {
        if (line[pos] != L'\u2063') { raw += line[pos++]; continue; }
        auto end = line.find(L'\u2063', pos + 1);
        if (end == std::wstring::npos) { raw += line.substr(pos); break; }
        auto token = line.substr(pos + 1, end - pos - 1);
        raw += token;
        shown[token] = processNumber(parse(utf8(token)), fullPrecision);
        pos = end + 1;
    }
    if (fullPrecision) return parenthesizeNegativeSubtrahends(raw);
    // 相近值相减时，保留能解释差值的数字，避免两个操作数被显示成相等。
    std::set<std::wstring> preserve;
    auto numberChar = [](wchar_t c) { return (c >= L'0' && c <= L'9') || c == L'.' || c == L'-' || c == L'+' || c == L'e' || c == L'E'; };
    for (std::size_t op = 0; (op = raw.find(L'−', op)) != std::wstring::npos; ++op) {
        auto leftEnd = op;
        while (leftEnd > 0 && std::iswspace(raw[leftEnd - 1])) --leftEnd;
        auto left = leftEnd;
        while (left > 0 && numberChar(raw[left - 1])) --left;
        auto right = op + 1;
        while (right < raw.size() && std::iswspace(raw[right])) ++right;
        auto end = right;
        while (end < raw.size() && numberChar(raw[end])) ++end;
        if (left == leftEnd || right == end) continue;
        auto aText = raw.substr(left, leftEnd - left), bText = raw.substr(right, end - right);
        try {
            double a = parse(utf8(aText)), b = parse(utf8(bText));
            double sa = shown.count(aText) ? parse(utf8(shown[aText])) : a;
            double sb = shown.count(bText) ? parse(utf8(shown[bText])) : b;
            double actual = a - b, display = sa - sb;
            if (!std::isfinite(actual) || !std::isfinite(display) || std::abs(display - actual) > std::abs(actual) * 1e-12) {
                preserve.insert(aText); preserve.insert(bText);
            }
        } catch (const std::exception &) { /* 该减号不是数值代入。 */ }
    }
    std::wstring result;
    for (std::size_t pos = 0; pos < line.size();) {
        if (line[pos] != L'\u2063') { result += line[pos++]; continue; }
        auto end = line.find(L'\u2063', pos + 1);
        if (end == std::wstring::npos) { result += line.substr(pos); break; }
        auto token = line.substr(pos + 1, end - pos - 1);
        result += preserve.count(token) ? token : shown[token];
        pos = end + 1;
    }
    return parenthesizeNegativeSubtrahends(result);
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
} // 命名空间 toolbox
