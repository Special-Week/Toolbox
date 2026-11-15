// 识别计算过程中的简单除法；复杂嵌套、连除和单位保留原文。
#pragma once
#include <cwctype>
#include <regex>
#include <string>
#include <vector>

namespace toolbox {
struct MathToken {
    std::wstring text;
    std::wstring latex;
};

// 只转换确定含义的表达式，未识别内容原样保留，避免改变运算次序。
inline const std::vector<std::pair<std::wstring, std::wstring>> &mathExpressions() {
    static const std::vector<std::pair<std::wstring, std::wstring>> expressions = {
        {L"atan[L/(πD₂)] × 180/π", LR"(\operatorname{atan}\left(\frac{L}{\pi D_2}\right)\times\frac{180}{\pi})"},
        {L"P/[2cos(α/2)]", LR"(\frac{P}{2\cos\left(\frac{\alpha}{2}\right)})"},
        {L"√[Σvᵢ²/(n − 1)]", LR"(\sqrt{\frac{\sum v_i^2}{n-1}})"},
        {L"√[Σ(xᵢ−x̄)²/(n−1)]", LR"(\sqrt{\frac{\sum(x_i-\bar{x})^2}{n-1}})"},
        {L"a₀·exp[a₁(t−a₂)²]", LR"(a_0\exp\left[a_1(t-a_2)^2\right])"},
        {L"a₀exp[a₁(t−a₂)²]", LR"(a_0\exp\left[a_1(t-a_2)^2\right])"},
        {L"E(t) = c₀ + c₁t + c₂t² + … + cₙtⁿ", LR"(E(t)=c_0+c_1t+c_2t^2+\cdots+c_nt^n)"},
        {L"3√3·P/8", LR"(\frac{3\sqrt{3}P}{8})"}, {L"5√3·P/8", LR"(\frac{5\sqrt{3}P}{8})"},
        {L"3√3P/8", LR"(\frac{3\sqrt{3}P}{8})"}, {L"5√3P/8", LR"(\frac{5\sqrt{3}P}{8})"},
        {L"3√3/8", LR"(\frac{3\sqrt{3}}{8})"}, {L"5√3/8", LR"(\frac{5\sqrt{3}}{8})"},
        {L"Σcᵢzⁱ", LR"(\sum_i c_i z^i)"}, {L"Σcᵢxⁱ", LR"(\sum_i c_i x^i)"},
        {L"Σcᵢtⁱ", LR"(\sum_i c_i t^i)"}, {L"Σc[i]·t^i", LR"(\sum_i c_i t^i)"},
        {L"R/R₀", LR"(\frac{R}{R_0})"},
        {L"1 + At + Bt² + C(t−100)t³", LR"(1+At+Bt^2+C(t-100)t^3)"},
        {L"1 + At + Bt²", LR"(1+At+Bt^2)"},
        {L"√(Σeᵢ²/n)", LR"(\sqrt{\frac{\sum e_i^2}{n}})"},
        {L"√[Σeᵢ²/(n−p)]", LR"(\sqrt{\frac{\sum e_i^2}{n-p}})"},
        {L"√(Σe²/n)", LR"(\sqrt{\frac{\sum e^2}{n}})"},
        {L"√[Σe²/(n−p)]", LR"(\sqrt{\frac{\sum e^2}{n-p}})"},
        {L"Σ(xᵢ − x₀)/n", LR"(\frac{\sum(x_i-x_0)}{n})"},
        {L"Σ(xᵢ−x̄)²", LR"(\sum(x_i-\bar{x})^2)"},
        {L"Σeᵢ²/Σ(yᵢ−ȳ)²", LR"(\frac{\sum e_i^2}{\sum(y_i-\bar{y})^2})"},
        {L"√[(cA·uA)²+(cB·uB)²]", LR"(\sqrt{(c_Au_A)^2+(c_Bu_B)^2})"},
        {L"√Σuᵢ²", LR"(\sqrt{\sum u_i^2})"}, {L"√Σq[i]²", LR"(\sqrt{\sum q_i^2})"},
        {L"√Σe[i]²", LR"(\sqrt{\sum e_i^2})"},
        {L"u=R/(C√m)", LR"(u=\frac{R}{C\sqrt{m}})"},
        {L"u=a/√3", LR"(u=\frac{a}{\sqrt{3}})"}, {L"u=a/√6", LR"(u=\frac{a}{\sqrt{6}})"},
        {L"u=a/√2", LR"(u=\frac{a}{\sqrt{2}})"}, {L"s/√n", LR"(\frac{s}{\sqrt{n}})"},
        {L"x = (Wr^(1/6)−0.65)/0.35", LR"(x=\frac{W_r^{1/6}-0.65}{0.35})"},
        {L"Wr = W − a₄(W−1) − b₄(W−1)ln(W)", LR"(W_r=W-a_4(W-1)-b_4(W-1)\ln(W))"},
        {L"C(u)=ΣBᵢ(u)Pᵢ", LR"(C(u)=\sum_i B_i(u)P_i)"},
        {L"B₀=(1−u)³/6", LR"(B_0=\frac{(1-u)^3}{6})"},
        {L"B₁=(3u³−6u²+4)/6", LR"(B_1=\frac{3u^3-6u^2+4}{6})"},
        {L"B₂=(−3u³+3u²+3u+1)/6", LR"(B_2=\frac{-3u^3+3u^2+3u+1}{6})"},
        {L"B₃=u³/6", LR"(B_3=\frac{u^3}{6})"},
        {L"Pᵢ₋₁+4Pᵢ+Pᵢ₊₁=6Qᵢ", LR"(P_{i-1}+4P_i+P_{i+1}=6Q_i)"},
        {L"U=2√(aₚ²+aₑ²)/√3", LR"(U=\frac{2\sqrt{a_p^2+a_e^2}}{\sqrt{3}})"},
        {L"λ⁻⁵ / [exp(c₂/(λT)) − 1]", LR"(\frac{\lambda^{-5}}{\exp\left(\frac{c_2}{\lambda T}\right)-1})"},
#include "toolbox/generated_formula_catalog.inc"
    };
    return expressions;
}

struct FormulaPart {
    std::wstring text, denominator;
    bool fraction = false;
};
struct WireFormula {
    std::wstring before, numerator, source, after;
    int angle = 0;
};
inline bool parseWireFormula(const std::wstring &line, WireFormula &out) {
    static const std::wregex pattern(LR"((-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*/\s*\[2cos\((30|55|60|70|75)°/2\)\])");
    std::wsmatch match;
    if (!std::regex_search(line, match, pattern)) return false;
    const auto start = static_cast<std::size_t>(match.position());
    if (start && (std::iswalnum(line[start - 1]) || line[start - 1] == L'_' || line[start - 1] == L'.')) return false;
    out.before = line.substr(0, start);
    out.numerator = match[1].str();
    out.source = match[0].str();
    out.after = line.substr(start + static_cast<std::size_t>(match.length()));
    out.angle = std::stoi(match[2].str());
    return true;
}
inline std::vector<FormulaPart> formulaParts(const std::wstring &line) {
    static const std::wregex pattern(
        LR"((\([^()；。，/\r\n]{1,80}\)|\[[^\[\]；。，/\r\n]{1,80}\]|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|源系数|输入值|Σxᵢ|xᵢ|Δ|R|s)\s*/\s*(\([^()；。，/\r\n]{1,80}\)|\[[^\[\]；。，/\r\n]{1,80}\]|√(?:n|[0-9]+(?:\.[0-9]+)?)|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|目标系数|引用值|真值|跨度|Rtp|R₀|T₀|d₂|sₓ|sᵧ|T|d|n))");
    auto bad = [](wchar_t c) {
        return std::iswalnum(c) || (c >= 0x3400 && c <= 0x9fff) || (c >= 0x1d00 && c <= 0x1dff) ||
               (c >= 0x2070 && c <= 0x209f) || c == L'_' || c == L'.' || c == L'/' ||
               c == L'√' || c == L'°' || c == L'%' || c == L'‰' || c == L'²' || c == L'³';
    };
    std::vector<FormulaPart> result;
    std::size_t end = 0;
    for (std::wsregex_iterator it(line.begin(), line.end(), pattern), last; it != last; ++it) {
        const auto &match = *it;
        const auto start = static_cast<std::size_t>(match.position());
        const auto stop = start + static_cast<std::size_t>(match.length());
        static const std::wregex numeric(LR"(\s*\(?\s*[+-]?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\s*\)?\s*)");
        if (std::regex_match(match[1].str(), numeric) && std::regex_match(match[2].str(), numeric)) continue;
        if ((start && (bad(line[start - 1]) || line[start - 1] == L')' || line[start - 1] == L']')) ||
            (stop < line.size() && (bad(line[stop]) || line[stop] == L'(' || line[stop] == L'['))) continue;
        const auto previous = line.find_last_not_of(L" \t\r\n", start ? start - 1 : 0);
        const auto next = line.find_first_not_of(L" \t\r\n", stop);
        if ((start && previous != std::wstring::npos && line[previous] == L'/') ||
            (next != std::wstring::npos && line[next] == L'/')) continue;
        auto clause = line.find_last_of(L"；。：\n", start);
        clause = clause == std::wstring::npos ? 0 : clause + 1;
        const auto equal = line.find(L'=', clause);
        const auto laterEqual = line.find(L'=', stop);
        const auto clauseEnd = line.find_first_of(L"；。：\n", stop);
        const bool leftHandRatio = match[1].str() == L"R" && match[2].str() == L"R₀" &&
            laterEqual != std::wstring::npos && (clauseEnd == std::wstring::npos || laterEqual < clauseEnd);
        if ((equal == std::wstring::npos || equal >= start) && !leftHandRatio) continue;
        if (start > end) result.push_back({line.substr(end, start - end), L"", false});
        result.push_back({match[1].str(), match[2].str(), true});
        end = stop;
    }
    if (end < line.size()) result.push_back({line.substr(end), L"", false});
    return result;
}
inline std::wstring mathOperand(const std::wstring &value) {
    static const std::wregex numeric(LR"(-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)");
    if (std::regex_match(value, numeric)) {
        const auto exponent = value.find_first_of(L"eE");
        return exponent == std::wstring::npos ? value : value.substr(0, exponent) + L"\\times 10^{" + value.substr(exponent + 1) + L"}";
    }
    std::wstring out;
    const auto subscript = [](wchar_t c) -> wchar_t {
        const std::wstring from = L"₀₁₂₃₄₅₆₇₈₉ᵢₓᵧₙₖ";
        const std::wstring to = L"0123456789ixynk";
        const auto index = from.find(c);
        return index == std::wstring::npos ? 0 : to[index];
    };
    for (std::size_t i = 0; i < value.size(); ++i) {
        const wchar_t c = value[i];
        if (const wchar_t digit = subscript(c)) {
            out += L"_{";
            out += digit;
            while (i + 1 < value.size() && subscript(value[i + 1])) out += subscript(value[++i]);
            out += L'}';
            continue;
        }
        if (c == L'√') {
            std::wstring root;
            while (i + 1 < value.size() && ((value[i + 1] >= L'0' && value[i + 1] <= L'9') || value[i + 1] == L'.')) root += value[++i];
            if (root.empty() && i + 1 < value.size()) root += value[++i];
            out += L"\\sqrt{" + root + L"}";
        } else if (c == L'Δ') out += L"\\Delta ";
        else if (c == L'Σ') out += L"\\sum ";
        else if (c == L'π') out += L"\\pi ";
        else if (c == L'²') out += L"^2";
        else if (c == L'³') out += L"^3";
        else if (c == L'−') out += L'-';
        else if (c == L'×') out += L"\\times ";
        else if (std::wstring(L"#$%&_{}").find(c) != std::wstring::npos) { out += L'\\'; out += c; }
        else if (c == L'\\') out += L"\\backslash ";
        else if (c < 128) out += c;
        else { out += L"\\text{"; out += c; out += L'}'; }
    }
    return out;
}

inline std::vector<MathToken> mathTokens(const std::wstring &line) {
    const auto &expressions = mathExpressions();
    std::vector<MathToken> tokens;
    std::size_t cursor = 0;
    const auto number = [](const std::wstring &value) {
        const bool wrapped = value.size() >= 2 && value.front() == L'(' && value.back() == L')';
        auto plain = wrapped ? value.substr(1, value.size() - 2) : value;
        if (!plain.empty() && plain.front() == L'−') plain.front() = L'-';
        const auto exponent = plain.find_first_of(L"eE");
        const auto tex = exponent == std::wstring::npos ? plain : plain.substr(0, exponent) + L"\\times 10^{" + plain.substr(exponent + 1) + L"}";
        return wrapped ? L"\\left(" + tex + L"\\right)" : tex;
    };
    static const std::wregex wire(LR"((-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*/\s*\[2cos\((30|55|60|70|75)°/2\)\])");
    static const std::wregex helix(LR"(atan\[(-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*/\s*\(π\s*×\s*(-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\)\]\s*×\s*180/π)");
    static const std::wregex gaussian(LR"((-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*×\s*exp\[(-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*×\s*\((-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*−\s*(\([-−](?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\)|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\)²\])");
    static const auto patterns = [] {
        const std::vector<std::pair<std::wstring, std::wstring>> raw = {
#include "toolbox/generated_formula_patterns.inc"
        };
        const std::wstring num = LR"((?:-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|\([-−](?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\)))";
        std::vector<std::pair<std::wregex, std::wstring>> result;
        for (const auto &entry : raw) {
            auto pattern = entry.first;
            std::size_t at = 0;
            while ((at = pattern.find(L'@', at)) != std::wstring::npos) {
                pattern.replace(at, 1, num);
                at += num.size();
            }
            result.emplace_back(std::wregex(pattern), entry.second);
        }
        return result;
    }();
    const auto validStart = [&](std::size_t at) {
        return at == 0 || !(std::iswalnum(line[at - 1]) || line[at - 1] == L'_' || line[at - 1] == L'.');
    };
    while (cursor < line.size()) {
        std::size_t best = std::wstring::npos;
        const std::pair<std::wstring, std::wstring> *selected = nullptr;
        std::wstring dynamicSource, dynamicLatex;
        const std::wstring remaining = line.substr(cursor);
        std::wsmatch match;
        if (std::regex_search(remaining, match, wire) && validStart(cursor + static_cast<std::size_t>(match.position()))) {
            best = cursor + static_cast<std::size_t>(match.position());
            dynamicSource = match[0].str();
            dynamicLatex = L"\\frac{" + number(match[1].str()) + L"}{2\\cos(\\frac{" + match[2].str() + L"^\\circ}{2})}";
        }
        if (std::regex_search(remaining, match, helix)) {
            const auto at = cursor + static_cast<std::size_t>(match.position());
            if (at < best && validStart(at)) {
                best = at;
                dynamicSource = match[0].str();
                dynamicLatex = L"\\operatorname{atan}\\left(\\frac{" + number(match[1].str()) + L"}{\\pi\\times " + number(match[2].str()) + L"}\\right)\\times\\frac{180}{\\pi}";
            }
        }
        if (std::regex_search(remaining, match, gaussian)) {
            const auto at = cursor + static_cast<std::size_t>(match.position());
            if (at < best && validStart(at)) {
                best = at;
                dynamicSource = match[0].str();
                const auto rightSource = match[4].str();
                const auto right = !rightSource.empty() && (rightSource[0] == L'-' || rightSource[0] == L'−')
                    ? L"\\left(" + number(rightSource) + L"\\right)" : number(rightSource);
                dynamicLatex = number(match[1].str()) + L"\\times\\exp[" + number(match[2].str()) + L"\\times(" + number(match[3].str()) + L"-" + right + L")^2]";
            }
        }
        for (const auto &entry : patterns) {
            if (!std::regex_search(remaining, match, entry.first)) continue;
            const auto at = cursor + static_cast<std::size_t>(match.position());
            if (!validStart(at) || (at > best) || (at == best && match.length() <= dynamicSource.size())) continue;
            best = at;
            dynamicSource = match[0].str();
            dynamicLatex = entry.second;
            for (std::size_t i = match.size(); i-- > 1;) {
                const auto placeholder = L"$" + std::to_wstring(i);
                std::size_t pos = 0;
                while ((pos = dynamicLatex.find(placeholder, pos)) != std::wstring::npos) {
                    const auto replacement = number(match[i].str());
                    dynamicLatex.replace(pos, placeholder.size(), replacement);
                    pos += replacement.size();
                }
            }
        }
        std::size_t offset = 0;
        for (const auto &part : formulaParts(remaining)) {
            if (!part.fraction) { offset += part.text.size(); continue; }
            const auto afterNumerator = offset + part.text.size();
            const auto denominator = remaining.find(part.denominator, afterNumerator);
            if (denominator != std::wstring::npos) {
                const auto at = cursor + offset;
                const auto end = denominator + part.denominator.size();
                if (at < best || (at == best && end - offset > dynamicSource.size())) {
                    best = at;
                    dynamicSource = remaining.substr(offset, end - offset);
                    dynamicLatex = L"\\frac{" + mathOperand(part.text) + L"}{" + mathOperand(part.denominator) + L"}";
                }
            }
            break;
        }
        for (const auto &expression : expressions) {
            const auto found = line.find(expression.first, cursor);
            const auto currentSize = selected ? selected->first.size() : dynamicSource.size();
            if (found != std::wstring::npos && (found < best || (found == best && expression.first.size() > currentSize))) {
                best = found;
                selected = &expression;
            }
        }
        if (best == std::wstring::npos) break;
        const auto &source = selected ? selected->first : dynamicSource;
        const auto &latex = selected ? selected->second : dynamicLatex;
        if (best > cursor) tokens.push_back({line.substr(cursor, best - cursor), L""});
        tokens.push_back({source, latex});
        cursor = best + source.size();
    }
    if (cursor < line.size()) tokens.push_back({line.substr(cursor), L""});
    return tokens;
}
} // namespace toolbox
