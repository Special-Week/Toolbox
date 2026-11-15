//! 只整理计算展示中的符号，不参与数值计算。
use regex::Regex;
use std::borrow::Cow;
use std::sync::OnceLock;

/// 给减号后的负数或负变量加括号，避免把两个减号连写。
pub(crate) fn parenthesize_negative_subtrahends(line: &str) -> Cow<'_, str> {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = PATTERN.get_or_init(|| Regex::new(
        r"[-−][ \t]*([-−](?:(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_α-ωΑ-Ω₀-₉]*|√(?:[0-9]+(?:\.[0-9]*)?|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_]*|\([^()]*\))|\([^()]*\))(?:[⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺]+|\^(?:\([^()]*\)|\{[^{}]*\}|[+-]?[0-9]+|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_]*))?)"
    ).unwrap());
    let mut result = String::new();
    let mut copied = 0;
    for captures in pattern.captures_iter(line) {
        let whole = captures.get(0).unwrap();
        let value = captures.get(1).unwrap();
        let before = line[..whole.start()].trim_end();
        let Some(left) = before.chars().next_back() else {
            continue;
        };
        let operand_end = left.is_ascii_alphanumeric()
            || ('α'..='ω').contains(&left)
            || ('Α'..='Ω').contains(&left)
            || ('₀'..='₉').contains(&left)
            || ")]}⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺".contains(left);
        if !operand_end {
            continue;
        }
        // 科学计数法中的 e- 不是减法运算符。
        if whole.as_str().starts_with('-')
            && matches!(left, 'e' | 'E')
            && before[..before.len() - 1]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit() || c == '.')
        {
            continue;
        }
        let after = &line[whole.end()..];
        if after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '^'))
        {
            continue;
        }
        // 半角连字符也可能表示范围或名称，只在明确的公式上下文中处理。
        if whole.as_str().starts_with('-') {
            let start = line[..whole.start()]
                .rfind(['；', '。', '，', '：', '\n'])
                .map_or(0, |pos| {
                    pos + line[pos..].chars().next().unwrap().len_utf8()
                });
            let end = line[whole.end()..]
                .find(['；', '。', '，', '：', '\n'])
                .map_or(line.len(), |pos| whole.end() + pos);
            let clause = &line[start..end];
            let standalone = line.trim().chars().all(|c| {
                c.is_ascii_alphanumeric()
                    || c.is_ascii_whitespace()
                    || ('α'..='ω').contains(&c)
                    || ('Α'..='Ω').contains(&c)
                    || ('₀'..='₉').contains(&c)
                    || "⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺._+-−×÷*/()[]{}^".contains(c)
            });
            if !clause.contains(['=', '≈']) && !standalone {
                continue;
            }
        }
        result.push_str(&line[copied..value.start()]);
        result.push('(');
        result.push_str(value.as_str());
        result.push(')');
        copied = value.end();
    }
    if copied == 0 {
        Cow::Borrowed(line)
    } else {
        result.push_str(&line[copied..]);
        Cow::Owned(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_subtrahends_are_unambiguous_without_changing_exponents_or_ranges() {
        for (input, expected) in [
            ("10 − -2 = 12", "10 − (-2) = 12"),
            ("-3--5=2", "-3-(-5)=2"),
            ("a--b", "a-(-b)"),
            ("x − −0.25 = 1", "x − (−0.25) = 1"),
            ("1e-3 − -2e-4 = 0.0012", "1e-3 − (-2e-4) = 0.0012"),
            ("a − -2²", "a − (-2²)"),
            ("a−-2^(2/3)", "a−(-2^(2/3))"),
            ("a--b^x", "a-(-b^x)"),
            ("x₀--y₀", "x₀-(-y₀)"),
            ("10 − -√4 = 12", "10 − (-√4) = 12"),
            ("a−-(b+c)", "a−(-(b+c))"),
            ("a−-2^(b+(c))", "a−-2^(b+(c))"),
            ("10 − (-2)", "10 − (-2)"),
            ("温区 -200--100 °C", "温区 -200--100 °C"),
            ("分量：A--B", "分量：A--B"),
            ("x=1e--3", "x=1e--3"),
            ("−0.2；结果 = -1.2e-5", "−0.2；结果 = -1.2e-5"),
        ] {
            assert_eq!(
                parenthesize_negative_subtrahends(input),
                expected,
                "{input}"
            );
            assert_eq!(
                parenthesize_negative_subtrahends(expected),
                expected,
                "应能重复处理"
            );
        }
    }
}
