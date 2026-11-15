//! 将三针公式的数值代入交给排版引擎，保留输入中的实际数字。
use regex::Regex;
use std::sync::OnceLock;

pub(super) fn parse(line: &str) -> Option<(&str, &str, String, &str)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?P<p>-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*/\s*\[2cos\((?P<a>30|55|60|70|75)°/2\)\]").unwrap());
    let captures = re.captures(line)?;
    let whole = captures.get(0)?;
    if line[..whole.start()]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return None;
    }
    let tex = format!(
        r"\frac{{{}}}{{2\cos\!\left(\frac{{{}^\circ}}{{2}}\right)}}",
        super::tex_number(captures.name("p")?.as_str()),
        captures.name("a")?.as_str()
    );
    Some((
        &line[..whole.start()],
        whole.as_str(),
        tex,
        &line[whole.end()..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_dynamic_numerator_and_angle() {
        for angle in [30, 55, 60, 70, 75] {
            let line = format!("代入：2.5 / [2cos({angle}°/2)] = 1.4。");
            let (before, source, tex, after) = parse(&line).unwrap();
            assert_eq!(format!("{before}{source}{after}"), line);
            assert!(tex.contains("2.5"));
            assert!(tex.contains(&angle.to_string()));
            assert!(super::super::render_formula(&tex, 17.5, 1.0).is_ok());
        }
        assert!(parse("2.5 / [2cos(45°/2)]").is_none());
    }
}
