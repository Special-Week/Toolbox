//! 只对含义明确的简单除法排版；复杂表达式保留原文，避免改变运算次序。
use eframe::egui::{self, Align2, FontId, Sense, Stroke, TextStyle, vec2};
use regex::Regex;
use std::sync::OnceLock;
mod wire_math;

#[derive(Debug, PartialEq)]
enum Part<'a> {
    Text(&'a str),
    Fraction(&'a str, &'a str),
}

const TEX_FORMULAS: &[(&str, &str)] = &[
    ("3√3/8", r"\frac{3\sqrt{3}}{8}"),
    ("5√3/8", r"\frac{5\sqrt{3}}{8}"),
    ("3√3P/8", r"\frac{3\sqrt{3}\,P}{8}"),
    ("5√3P/8", r"\frac{5\sqrt{3}\,P}{8}"),
    ("3√3·P/8", r"\frac{3\sqrt{3}\,P}{8}"),
    ("5√3·P/8", r"\frac{5\sqrt{3}\,P}{8}"),
    (
        "atan[L/(πD₂)] × 180/π",
        r"\operatorname{atan}\!\left(\frac{L}{\pi D_2}\right)\times\frac{180}{\pi}",
    ),
    (
        "P/[2cos(α/2)]",
        r"\frac{P}{2\cos\!\left(\frac{\alpha}{2}\right)}",
    ),
    ("√[Σvᵢ²/(n − 1)]", r"\sqrt{\frac{\sum v_i^2}{n-1}}"),
    (
        "√[Σ(xᵢ−x̄)²/(n−1)]",
        r"\sqrt{\frac{\sum (x_i-\bar{x})^2}{n-1}}",
    ),
    (
        "a₀·exp[a₁(t−a₂)²]",
        r"a_0\cdot\exp\!\left[a_1(t-a_2)^2\right]",
    ),
    (
        "a₀exp[a₁(t−a₂)²]",
        r"a_0\cdot\exp\!\left[a_1(t-a_2)^2\right]",
    ),
    (
        "E(t) = c₀ + c₁t + c₂t² + … + cₙtⁿ",
        r"E(t)=c_0+c_1t+c_2t^2+\cdots+c_nt^n",
    ),
    ("Σcᵢtⁱ", r"\sum_i c_i t^i"),
    ("Σc[i]·t^i", r"\sum_i c_i t^i"),
    ("Σcᵢzⁱ", r"\sum_i c_i z^i"),
    ("Σcᵢxⁱ", r"\sum_i c_i x^i"),
    ("R/R₀", r"\frac{R}{R_0}"),
    ("1 + At + Bt² + C(t−100)t³", r"1+At+Bt^2+C(t-100)t^3"),
    ("1 + At + Bt²", r"1+At+Bt^2"),
    ("√(Σeᵢ²/n)", r"\sqrt{\frac{\sum e_i^2}{n}}"),
    ("√[Σeᵢ²/(n−p)]", r"\sqrt{\frac{\sum e_i^2}{n-p}}"),
    ("√(Σe²/n)", r"\sqrt{\frac{\sum e^2}{n}}"),
    ("√[Σe²/(n−p)]", r"\sqrt{\frac{\sum e^2}{n-p}}"),
    ("Σ(xᵢ − x₀)/n", r"\frac{\sum(x_i-x_0)}{n}"),
    ("Σ(xᵢ−x̄)²", r"\sum(x_i-\bar{x})^2"),
    ("Σeᵢ²/Σ(yᵢ−ȳ)²", r"\frac{\sum e_i^2}{\sum(y_i-\bar{y})^2}"),
    ("√[(cA·uA)²+(cB·uB)²]", r"\sqrt{(c_Au_A)^2+(c_Bu_B)^2}"),
    ("√Σuᵢ²", r"\sqrt{\sum u_i^2}"),
    ("√Σq[i]²", r"\sqrt{\sum q_i^2}"),
    ("√Σe[i]²", r"\sqrt{\sum e_i^2}"),
    ("u=R/(C√m)", r"u=\frac{R}{C\sqrt{m}}"),
    ("u=a/√3", r"u=\frac{a}{\sqrt{3}}"),
    ("u=a/√6", r"u=\frac{a}{\sqrt{6}}"),
    ("u=a/√2", r"u=\frac{a}{\sqrt{2}}"),
    ("s/√n", r"\frac{s}{\sqrt{n}}"),
    ("x = (Wr^(1/6)−0.65)/0.35", r"x=\frac{W_r^{1/6}-0.65}{0.35}"),
    (
        "Wr = W − a₄(W−1) − b₄(W−1)ln(W)",
        r"W_r=W-a_4(W-1)-b_4(W-1)\ln(W)",
    ),
    ("C(u)=ΣBᵢ(u)Pᵢ", r"C(u)=\sum_i B_i(u)P_i"),
    ("B₀=(1−u)³/6", r"B_0=\frac{(1-u)^3}{6}"),
    ("B₁=(3u³−6u²+4)/6", r"B_1=\frac{3u^3-6u^2+4}{6}"),
    ("B₂=(−3u³+3u²+3u+1)/6", r"B_2=\frac{-3u^3+3u^2+3u+1}{6}"),
    ("B₃=u³/6", r"B_3=\frac{u^3}{6}"),
    ("Pᵢ₋₁+4Pᵢ+Pᵢ₊₁=6Qᵢ", r"P_{i-1}+4P_i+P_{i+1}=6Q_i"),
    (
        "U=2√(aₚ²+aₑ²)/√3",
        r"U=\frac{2\sqrt{a_p^2+a_e^2}}{\sqrt{3}}",
    ),
    (
        "λ⁻⁵ / [exp(c₂/(λT)) − 1]",
        r"\frac{\lambda^{-5}}{\exp\left(\frac{c_2}{\lambda T}\right)-1}",
    ),
];

fn tex_number(value: &str) -> String {
    if let Some(inner) = value.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        return format!(r"\left({}\right)", tex_number(inner));
    }
    if let Some(inner) = value.strip_prefix('−') {
        return format!("-{}", tex_number(inner));
    }
    if let Some((mantissa, exponent)) = value.split_once(['e', 'E']) {
        format!(r"{mantissa}\times 10^{{{exponent}}}")
    } else {
        value.to_owned()
    }
}

fn tex_operand(value: &str) -> String {
    if value.parse::<f64>().is_ok() {
        return tex_number(value);
    }
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        let subscript = |ch| match ch {
            '₀' => Some('0'),
            '₁' => Some('1'),
            '₂' => Some('2'),
            '₃' => Some('3'),
            '₄' => Some('4'),
            '₅' => Some('5'),
            '₆' => Some('6'),
            '₇' => Some('7'),
            '₈' => Some('8'),
            '₉' => Some('9'),
            'ᵢ' => Some('i'),
            'ₖ' => Some('k'),
            'ₓ' => Some('x'),
            'ᵧ' => Some('y'),
            'ₙ' => Some('n'),
            _ => None,
        };
        if let Some(first) = subscript(c) {
            out.push_str("_{");
            out.push(first);
            while let Some(next) = chars.peek().copied().and_then(subscript) {
                chars.next();
                out.push(next);
            }
            out.push('}');
            continue;
        }
        match c {
            '√' => {
                let mut root = String::new();
                while chars
                    .peek()
                    .is_some_and(|next| next.is_ascii_digit() || *next == '.')
                {
                    root.push(chars.next().unwrap());
                }
                if root.is_empty()
                    && let Some(next) = chars.next()
                {
                    root.push(next);
                }
                out.push_str(&format!(r"\sqrt{{{root}}}"));
            }
            'Δ' => out.push_str(r"\Delta "),
            'Σ' => out.push_str(r"\sum "),
            'π' => out.push_str(r"\pi "),
            '²' => out.push_str("^2"),
            '³' => out.push_str("^3"),
            '−' => out.push('-'),
            '×' => out.push_str(r"\times "),
            '#' | '$' | '%' | '&' | '_' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            '\\' => out.push_str(r"\backslash "),
            c if c.is_ascii() => out.push(c),
            c => out.push_str(&format!(r"\text{{{c}}}")),
        }
    }
    out
}

fn tex_formula(line: &str) -> Option<(&str, &str, std::borrow::Cow<'static, str>, &str)> {
    let mut best = wire_math::parse(line)
        .map(|(before, source, tex, after)| (before, source, tex.into(), after));
    static GAUSSIAN: OnceLock<Regex> = OnceLock::new();
    let pattern = GAUSSIAN.get_or_init(|| Regex::new(
        r"(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*×\s*exp\[(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*×\s*\((-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*−\s*(\([-−](?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?\)|-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\)²\]"
    ).unwrap());
    if let Some(c) = pattern.captures(line) {
        let m = c.get(0).unwrap();
        let right = if c[4].starts_with(['-', '−']) {
            format!(r"\left({}\right)", tex_number(&c[4]))
        } else {
            tex_number(&c[4])
        };
        let tex = format!(
            r"{}\times\exp\!\left[{}\times\left({}-{}\right)^2\right]",
            tex_number(&c[1]),
            tex_number(&c[2]),
            tex_number(&c[3]),
            right
        );
        if best
            .as_ref()
            .is_none_or(|(before, _, _, _)| m.start() < before.len())
        {
            best = Some((&line[..m.start()], m.as_str(), tex.into(), &line[m.end()..]));
        }
    }
    static HELIX: OnceLock<Regex> = OnceLock::new();
    let helix = HELIX.get_or_init(|| Regex::new(r"atan\[(-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\s*/\s*\(π\s*×\s*(-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)\)\]\s*×\s*180/π").unwrap());
    if let Some(c) = helix.captures(line) {
        let m = c.get(0).unwrap();
        if best
            .as_ref()
            .is_none_or(|(before, _, _, _)| m.start() < before.len())
        {
            let tex = format!(
                r"\operatorname{{atan}}\left(\frac{{{}}}{{\pi\times {}}}\right)\times\frac{{180}}{{\pi}}",
                tex_number(&c[1]),
                tex_number(&c[2])
            );
            best = Some((&line[..m.start()], m.as_str(), tex.into(), &line[m.end()..]));
        }
    }
    for (source, tex) in TEX_FORMULAS {
        if let Some(start) = line.find(source)
            && best.as_ref().is_none_or(|(before, current, _, _)| {
                start < before.len() || (start == before.len() && source.len() > current.len())
            })
        {
            best = Some((
                &line[..start],
                source,
                (*tex).into(),
                &line[start + source.len()..],
            ));
        }
    }
    static CATALOG: OnceLock<Vec<[String; 2]>> = OnceLock::new();
    for [source, tex] in CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/math/formula-catalog.json"))
            .expect("内置公式目录应为有效 JSON")
    }) {
        if let Some(start) = line.find(source.as_str())
            && best.as_ref().is_none_or(|(before, current, _, _)| {
                start < before.len() || (start == before.len() && source.len() > current.len())
            })
        {
            best = Some((
                &line[..start],
                source.as_str(),
                tex.as_str().into(),
                &line[start + source.len()..],
            ));
        }
    }
    static PATTERNS: OnceLock<Vec<(Regex, String)>> = OnceLock::new();
    let number_pattern = r"(?:-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|\([-−](?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\))";
    for (pattern, template) in PATTERNS.get_or_init(|| {
        let entries: Vec<[String; 2]> =
            serde_json::from_str(include_str!("../../assets/math/formula-patterns.json"))
                .expect("动态公式目录应为有效 JSON");
        entries
            .into_iter()
            .map(|[pattern, template]| {
                (
                    Regex::new(&pattern.replace('@', number_pattern))
                        .expect("动态公式表达式应有效"),
                    template,
                )
            })
            .collect()
    }) {
        if let Some(captures) = pattern.captures(line) {
            let m = captures.get(0).unwrap();
            if best.as_ref().is_none_or(|(before, current, _, _)| {
                m.start() < before.len() || (m.start() == before.len() && m.len() > current.len())
            }) {
                let mut tex = template.clone();
                for index in (1..captures.len()).rev() {
                    tex = tex.replace(&format!("${index}"), &tex_number(&captures[index]));
                }
                best = Some((&line[..m.start()], m.as_str(), tex.into(), &line[m.end()..]));
            }
        }
    }
    let mut offset = 0;
    for part in parts(line) {
        match part {
            Part::Text(text) => offset += text.len(),
            Part::Fraction(numerator, denominator) => {
                let after_numerator = offset + numerator.len();
                if let Some(relative) = line[after_numerator..].find(denominator) {
                    let end = after_numerator + relative + denominator.len();
                    if best.as_ref().is_none_or(|(before, current, _, _)| {
                        offset < before.len()
                            || (offset == before.len() && end - offset > current.len())
                    }) {
                        let tex = format!(
                            r"\frac{{{}}}{{{}}}",
                            tex_operand(numerator),
                            tex_operand(denominator)
                        );
                        best = Some((
                            &line[..offset],
                            &line[offset..end],
                            tex.into(),
                            &line[end..],
                        ));
                    }
                }
                break;
            }
        }
    }
    best
}

// 以屏幕像素的两倍分辨率绘制；极高 DPI 下限制纹理尺寸。
fn formula_raster_scale(pixels_per_point: f32) -> f32 {
    (pixels_per_point * 2.0).min(16.0)
}

/// 按当前屏幕密度生成公式，同时保留公式基线供正文对齐。
fn render_formula(
    tex: &str,
    font_size: f32,
    pixels_per_point: f32,
) -> Result<(Vec<u8>, f32, f32), String> {
    use ratex_layout::{LayoutOptions, layout, to_display_list, validate_layout};
    use ratex_types::color::Color;
    let ast = ratex_parser::parser::parse(tex).map_err(|e| e.to_string())?;
    let options = LayoutOptions::default().with_color(Color::WHITE);
    let laid_out = layout(&ast, &options);
    validate_layout(&laid_out)?;
    let display_list = to_display_list(&laid_out);
    let baseline = 1.0 + display_list.height as f32 * font_size;
    let render = |density| {
        ratex_render::render_to_png(
            &display_list,
            &ratex_render::RenderOptions {
                font_size,
                padding: 1.0,
                background_color: Color::new(0.0, 0.0, 0.0, 0.0),
                font_dir: String::new(),
                device_pixel_ratio: density,
            },
        )
    };
    let mut raster_scale = formula_raster_scale(pixels_per_point);
    // 超长公式可能超过引擎的图片尺寸限制，退回原有屏幕密度。
    let png = match render(raster_scale) {
        Ok(png) => png,
        Err(_) if raster_scale > pixels_per_point => {
            raster_scale = pixels_per_point;
            render(raster_scale)?
        }
        Err(error) => return Err(error),
    };
    Ok((png, baseline, raster_scale))
}

/// 将高分辨率公式预缩至屏幕像素，并增强细笔画的透明度。
fn formula_screen_image(
    mut source: egui::IconData,
    raster_scale: f32,
    pixels_per_point: f32,
) -> Option<(egui::ColorImage, f32)> {
    use image::{RgbaImage, imageops::FilterType};

    let scale = pixels_per_point.min(raster_scale);
    let width = ((source.width as f32 * scale / raster_scale).ceil() as u32).max(1);
    let height = ((source.height as f32 * scale / raster_scale).ceil() as u32).max(1);
    // 先转成预乘透明度，避免缩放时透明边缘混入黑色。
    for pixel in source.rgba.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let buffer = RgbaImage::from_raw(source.width, source.height, source.rgba)?;
    let mut resized = if width == buffer.width() && height == buffer.height() {
        buffer
    } else {
        image::imageops::resize(&buffer, width, height, FilterType::Triangle)
    };
    static COVERAGE: OnceLock<[u8; 256]> = OnceLock::new();
    let coverage = COVERAGE.get_or_init(|| {
        std::array::from_fn(|alpha| ((alpha as f64 / 255.0).powf(0.7) * 255.0).round() as u8)
    });
    for pixel in resized.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        let adjusted = u16::from(coverage[usize::from(pixel[3])]);
        for channel in &mut pixel.0[..3] {
            let value = (u16::from(*channel) * adjusted + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0);
            *channel = value.min(adjusted) as u8;
        }
        pixel[3] = adjusted as u8;
    }
    Some((
        egui::ColorImage::from_rgba_premultiplied(
            [width as usize, height as usize],
            resized.as_raw(),
        ),
        scale,
    ))
}

fn parts(line: &str) -> Vec<Part<'_>> {
    static FORMULA: OnceLock<Regex> = OnceLock::new();
    let pattern = FORMULA.get_or_init(|| Regex::new(
        r"(?P<a>\([^()；。，/\r\n]{1,80}\)|\[[^\[\]；。，/\r\n]{1,80}\]|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|源系数|输入值|Σxᵢ|xᵢ|Δ|R|s)\s*/\s*(?P<b>\([^()；。，/\r\n]{1,80}\)|\[[^\[\]；。，/\r\n]{1,80}\]|√(?:n|[0-9]+(?:\.[0-9]+)?)|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|目标系数|引用值|真值|跨度|Rtp|R₀|T₀|d₂|sₓ|sᵧ|T|d|n)"
    ).unwrap());
    let mut out = Vec::new();
    let mut end = 0;
    for m in pattern.captures_iter(line) {
        let whole = m.get(0).unwrap();
        let numeric = |text: &str| {
            let text = text.trim();
            let text = text
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(text);
            text.trim().parse::<f64>().is_ok()
        };
        if numeric(m.name("a").unwrap().as_str()) && numeric(m.name("b").unwrap().as_str()) {
            continue;
        }
        let before = line[..whole.start()].chars().next_back();
        let after = line[whole.end()..].chars().next();
        let bad =
            |c: char| c.is_alphanumeric() || matches!(c, '_' | '.' | '/' | '√' | '°' | '%' | '‰');
        if before.is_some_and(|c| bad(c) || matches!(c, ')' | ']'))
            || after.is_some_and(|c| bad(c) || matches!(c, '(' | '['))
        {
            continue;
        }
        if line[..whole.start()].trim_end().ends_with('/')
            || line[whole.end()..].trim_start().starts_with('/')
        {
            continue;
        }
        let clause = line[..whole.start()]
            .rsplit(['；', '。', '：', '\n'])
            .next()
            .unwrap_or("");
        let left_hand_ratio = m.name("a").unwrap().as_str() == "R"
            && m.name("b").unwrap().as_str() == "R₀"
            && line[whole.end()..]
                .split(['；', '。', '：', '\n'])
                .next()
                .unwrap_or("")
                .contains('=');
        if !clause.contains('=') && !left_hand_ratio {
            continue;
        }
        if whole.start() > end {
            out.push(Part::Text(&line[end..whole.start()]));
        }
        out.push(Part::Fraction(
            m.name("a").unwrap().as_str(),
            m.name("b").unwrap().as_str(),
        ));
        end = whole.end();
    }
    if end < line.len() {
        out.push(Part::Text(&line[end..]));
    }
    out
}

pub(super) fn has_formattable_math(line: &str) -> bool {
    tex_formula(line).is_some()
        || parts(line)
            .iter()
            .any(|part| matches!(part, Part::Fraction(..)))
}

#[cfg(test)]
pub(crate) fn visible_text_fragments(line: &str) -> Vec<&str> {
    if line
        .split('；')
        .filter(|clause| tex_formula(clause).is_some())
        .count()
        > 1
    {
        return line
            .split_inclusive('；')
            .flat_map(visible_text_fragments)
            .collect();
    }
    if let Some((before, _, _, after)) = tex_formula(line) {
        let mut fragments = visible_text_fragments(before);
        fragments.extend(visible_text_fragments(after));
        return fragments;
    }
    parts(line)
        .into_iter()
        .flat_map(|part| match part {
            Part::Text(text) => vec![text],
            Part::Fraction(a, b) => vec![a, b],
        })
        .collect()
}

/// 一行能完整放下时绘制分式，否则沿用原本自动换行的文本。
pub(super) fn show(ui: &mut egui::Ui, line: &str) {
    let normalized = super::math_notation::parenthesize_negative_subtrahends(line);
    let line = normalized.as_ref();
    if line
        .split('；')
        .filter(|clause| tex_formula(clause).is_some())
        .count()
        > 1
    {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for clause in line.split_inclusive('；') {
                show(ui, clause);
            }
        });
        return;
    }
    if let Some((before, source, _, after)) = tex_formula(line) {
        if parts(before)
            .iter()
            .any(|p| matches!(p, Part::Fraction(..)))
        {
            let split = before.len();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                show(ui, &line[..split]);
                show(ui, &line[split..]);
            });
            return;
        }
        if let Some((between, _, _, _)) = tex_formula(after) {
            let split = before.len() + source.len() + between.len();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                show(ui, &line[..split]);
                show(ui, &line[split..]);
            });
            return;
        }
        if parts(after).iter().any(|p| matches!(p, Part::Fraction(..))) {
            let split = before.len() + source.len();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                show(ui, &line[..split]);
                show(ui, &line[split..]);
            });
            return;
        }
    }
    if let Some((before, source, tex, after)) = tex_formula(line) {
        let font = TextStyle::Body.resolve(ui.style());
        // 数学字体的小写字母通常比界面字体矮，略微放大后与正文更协调。
        let math_size = font.size * 1.30;
        let pixels_per_point = ui.ctx().pixels_per_point().clamp(0.25, 16.0);
        let key = (
            tex.to_string(),
            math_size.to_bits(),
            pixels_per_point.to_bits(),
        );
        // 动态输入可能不断变化，限制缓存数量，避免纹理持续增长。
        let cache_id = egui::Id::new("typeset-formula-cache");
        type FormulaCache =
            std::collections::VecDeque<((String, u32, u32), egui::TextureHandle, f32, f32)>;
        let cached = ui.ctx().data_mut(|data| {
            data.get_temp::<FormulaCache>(cache_id).and_then(|cache| {
                cache
                    .iter()
                    .find(|(cached_key, _, _, _)| cached_key == &key)
                    .map(|(_, texture, baseline, scale)| (texture.clone(), *baseline, *scale))
            })
        });
        let (texture, baseline, raster_scale) = if let Some(cached) = cached {
            cached
        } else {
            let rendered = render_formula(&tex, math_size, pixels_per_point)
                .ok()
                .and_then(|(png, baseline, scale)| {
                    eframe::icon_data::from_png_bytes(&png)
                        .ok()
                        .and_then(|image| formula_screen_image(image, scale, pixels_per_point))
                        .map(|(image, scale)| (image, baseline, scale))
                });
            let Some((image, baseline, raster_scale)) = rendered else {
                ui.add(egui::Label::new(line).wrap());
                return;
            };
            let texture = ui
                .ctx()
                .load_texture(tex.as_ref(), image, egui::TextureOptions::LINEAR);
            ui.ctx().data_mut(|data| {
                let mut cache = data.get_temp::<FormulaCache>(cache_id).unwrap_or_default();
                if cache.len() >= 128 {
                    cache.pop_front();
                }
                cache.push_back((key, texture.clone(), baseline, raster_scale));
                data.insert_temp(cache_id, cache);
            });
            (texture, baseline, raster_scale)
        };
        let [width, height] = texture.size();
        let natural_size = vec2(width as f32 / raster_scale, height as f32 / raster_scale);
        let scale = (ui.available_width() / natural_size.x).min(1.0);
        let image_size = natural_size * scale;
        let image_baseline = baseline * scale;
        let color = ui.visuals().text_color();
        let layout_text = |text: &str| {
            ui.painter()
                .layout_no_wrap(text.to_owned(), font.clone(), color)
        };
        let before_galley = layout_text(before);
        let after_galley = layout_text(after);
        let sample_galley = layout_text("x");
        let sample_baseline = sample_galley.rows[0].glyphs[0].pos.y;
        let baseline_of = |galley: &egui::Galley| {
            galley
                .rows
                .first()
                .and_then(|row| row.glyphs.first())
                .map_or(sample_baseline, |glyph| glyph.pos.y)
        };
        let before_baseline = baseline_of(&before_galley);
        let after_baseline = baseline_of(&after_galley);
        let before_width = before_galley.size().x;
        let after_width = after_galley.size().x;
        let total = before_width + image_size.x + after_width;
        if total <= ui.available_width() {
            let above = before_baseline.max(after_baseline).max(image_baseline);
            let below = (before_galley.size().y - before_baseline)
                .max(after_galley.size().y - after_baseline)
                .max(image_size.y - image_baseline);
            let row_height = (above + below).max(font.size * 1.5);
            let (rect, response) = ui.allocate_exact_size(vec2(total, row_height), Sense::hover());
            let painter = ui.painter();
            let row_baseline = rect.center().y - (above + below) / 2.0 + above;
            if !before.is_empty() {
                painter.galley(
                    egui::pos2(rect.left(), row_baseline - before_baseline),
                    before_galley,
                    color,
                );
            }
            let image_rect = egui::Rect::from_min_size(
                egui::pos2(
                    ((rect.left() + before_width) * pixels_per_point).round() / pixels_per_point,
                    ((row_baseline - image_baseline) * pixels_per_point).round() / pixels_per_point,
                ),
                image_size,
            );
            painter.image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                color,
            );
            if !after.is_empty() {
                painter.galley(
                    egui::pos2(image_rect.right(), row_baseline - after_baseline),
                    after_galley,
                    color,
                );
            }
            response.on_hover_text(source);
            return;
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            if !before.is_empty() {
                ui.add(egui::Label::new(before).wrap());
            }
            ui.add(
                egui::Image::new((texture.id(), image_size))
                    .tint(color)
                    .alt_text(source),
            );
            if !after.is_empty() {
                ui.add(egui::Label::new(after).wrap());
            }
        });
        return;
    }
    let parts = parts(line);
    if !parts.iter().any(|p| matches!(p, Part::Fraction(..))) {
        ui.add(egui::Label::new(line).wrap());
        return;
    }
    let font = TextStyle::Body.resolve(ui.style());
    let fraction_font = FontId::new(font.size, font.family.clone());
    let color = ui.visuals().text_color();
    let width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), color)
            .size()
            .x
    };
    let widths: Vec<f32> = parts
        .iter()
        .map(|p| match p {
            Part::Text(text) => width(text),
            Part::Fraction(a, b) => width(a).max(width(b)) + 8.0,
        })
        .collect();
    let total: f32 = widths.iter().sum();
    if total > ui.available_width() {
        ui.add(egui::Label::new(line).wrap());
        return;
    }
    let height = (font.size * 2.5).max(34.0);
    let (rect, _) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
    let painter = ui.painter();
    let mut x = rect.left();
    for (part, width) in parts.iter().zip(widths) {
        match part {
            Part::Text(text) => {
                painter.text(
                    egui::pos2(x, rect.center().y),
                    Align2::LEFT_CENTER,
                    *text,
                    font.clone(),
                    color,
                );
            }
            Part::Fraction(a, b) => {
                let mid = x + width / 2.0;
                painter.text(
                    egui::pos2(mid, rect.top() + height * 0.25),
                    Align2::CENTER_CENTER,
                    *a,
                    fraction_font.clone(),
                    color,
                );
                painter.line_segment(
                    [
                        egui::pos2(x + 2.0, rect.center().y),
                        egui::pos2(x + width - 2.0, rect.center().y),
                    ],
                    Stroke::new(1.0_f32, color),
                );
                painter.text(
                    egui::pos2(mid, rect.top() + height * 0.75),
                    Align2::CENTER_CENTER,
                    *b,
                    fraction_font.clone(),
                    color,
                );
            }
        }
        x += width;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_unambiguous_divisions_become_fractions() {
        assert_eq!(
            parts("相对误差 = Δ/真值 × 100 = 0.02 / 100 × 100"),
            vec![
                Part::Text("相对误差 = "),
                Part::Fraction("Δ", "真值"),
                Part::Text(" × 100 = 0.02 / 100 × 100")
            ]
        );
        assert_eq!(
            parts("u = s/√n = 2 / √1"),
            vec![
                Part::Text("u = "),
                Part::Fraction("s", "√n"),
                Part::Text(" = "),
                Part::Fraction("2", "√1")
            ]
        );
        for s in [
            "斜率 = 0.3 m/s",
            "结果 = 2 / 3 / 4",
            "日期 2026/11/15",
            "值 = √2/3",
            "值 = 2 / 30°C",
            "值 = 2 / 30%",
            "公差 = 224/1000 = 0.224",
            "值 = (2) / (-3)",
        ] {
            assert!(
                !parts(s).iter().any(|p| matches!(p, Part::Fraction(..))),
                "{s}"
            );
        }
        assert_eq!(
            parts("值 = (1 + 2) / 3"),
            vec![Part::Text("值 = "), Part::Fraction("(1 + 2)", "3")]
        );
        assert_eq!(parts("值 = (1/1000)"), vec![Part::Text("值 = (1/1000)")]);
        assert!(parts("CVD：R/R₀ = 1 + At").contains(&Part::Fraction("R", "R₀")));
    }

    #[test]
    fn exact_latex_assets_keep_surrounding_substitutions() {
        for (line, formula) in [
            (
                "升角 ψ = atan[L/(πD₂)] × 180/π = 4.85°。",
                "atan[L/(πD₂)] × 180/π",
            ),
            ("针径 d₀ = P/[2cos(α/2)] = 1.44 mm。", "P/[2cos(α/2)]"),
            ("s = √[Σvᵢ²/(n − 1)] = 0.01。", "√[Σvᵢ²/(n − 1)]"),
            ("指数项 a₀·exp[a₁(t−a₂)²]；a₀=0.1", "a₀·exp[a₁(t−a₂)²]"),
            ("k₂ = 3√3/8", "3√3/8"),
            ("D₂ = D − 3√3·P/8", "3√3·P/8"),
        ] {
            let (before, source, tex, after) = tex_formula(line).expect("应识别公式");
            assert_eq!(source, formula);
            assert_eq!(format!("{before}{source}{after}"), line);
            let (image, _, _) = render_formula(&tex, 17.5, 1.0).expect("公式应能排版");
            assert!(eframe::icon_data::from_png_bytes(&image).is_ok());
        }
    }

    #[test]
    fn gaussian_uses_current_numbers_and_preserves_result() {
        for temperature in ["100", "250", "-10"] {
            let line = format!(
                "指数项 = 0.1185976 × exp[-0.0001183432 × ({temperature} − 126.9686)²] = 0.1 mV。"
            );
            let (before, source, tex, after) = tex_formula(&line).expect("应识别数值代入");
            assert_eq!(format!("{before}{source}{after}"), line);
            assert!(tex.contains(temperature));
            assert!(render_formula(&tex, 17.5, 1.0).is_ok());
        }
        assert_eq!(tex_number("-1.2e-9"), r"-1.2\times 10^{-9}");
    }

    #[test]
    fn rendering_tracks_screen_density_without_changing_logical_size() {
        let tex = r"E(t)=c_0+c_1t+c_2t^2+\cdots+c_nt^n";
        let (first, first_baseline, first_scale) = render_formula(tex, 17.5, 1.0).unwrap();
        let first = eframe::icon_data::from_png_bytes(&first).unwrap();
        assert_eq!(formula_raster_scale(1.0), 2.0);
        assert_eq!(first_scale, 2.0);
        for density in [1.25, 1.5, 2.0, 3.0] {
            let (png, baseline, scale) = render_formula(tex, 17.5, density).unwrap();
            let image = eframe::icon_data::from_png_bytes(&png).unwrap();
            assert_eq!(scale, formula_raster_scale(density));
            assert!((image.width as f32 / scale - first.width as f32 / 2.0).abs() <= 1.0);
            assert!((image.height as f32 / scale - first.height as f32 / 2.0).abs() <= 1.0);
            assert_eq!(first_baseline, baseline);
        }
        assert_eq!(formula_raster_scale(16.0), 16.0);
        assert!(first.rgba.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn screen_formula_strengthens_fine_edges_without_breaking_transparency() {
        let mut rgba = Vec::new();
        for _ in 0..16 {
            rgba.extend_from_slice(&[255, 128, 64, 64]);
        }
        let input = egui::IconData {
            width: 4,
            height: 4,
            rgba,
        };
        let (image, scale) = formula_screen_image(input, 2.0, 1.0).unwrap();
        assert_eq!(image.size, [2, 2]);
        assert_eq!(scale, 1.0);
        for pixel in image.pixels {
            assert!(pixel.a() > 64 && pixel.a() < 255);
            assert!(pixel.r() <= pixel.a() && pixel.g() <= pixel.a() && pixel.b() <= pixel.a());
            assert!(pixel.r() > pixel.g() && pixel.g() > pixel.b());
        }
        let empty = egui::IconData {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
        };
        let (empty, _) = formula_screen_image(empty, 2.0, 1.0).unwrap();
        assert_eq!(empty.pixels[0], egui::Color32::TRANSPARENT);
    }

    #[test]
    fn screen_formula_keeps_logical_size_at_different_dpi() {
        let tex = r"\sqrt{\frac{\sum v_i^2}{n-1}}";
        let mut size = None;
        for density in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let (png, _, raster_scale) = render_formula(tex, 17.5, density).unwrap();
            let source = eframe::icon_data::from_png_bytes(&png).unwrap();
            let (image, scale) = formula_screen_image(source, raster_scale, density).unwrap();
            assert_eq!(scale, density);
            let logical = [image.size[0] as f32 / scale, image.size[1] as f32 / scale];
            if let Some(first) = size {
                let first: [f32; 2] = first;
                assert!((logical[0] - first[0]).abs() <= 1.0);
                assert!((logical[1] - first[1]).abs() <= 1.0);
            } else {
                size = Some(logical);
            }
            assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
        }
    }

    #[test]
    fn shared_catalog_renders_and_preserves_every_source_expression() {
        let entries: Vec<[String; 2]> =
            serde_json::from_str(include_str!("../../assets/math/formula-catalog.json")).unwrap();
        for [source, tex] in entries {
            let (_, matched, _, _) =
                tex_formula(&source).unwrap_or_else(|| panic!("未识别 {source}"));
            assert_eq!(matched, source);
            let (png, _, _) =
                render_formula(&tex, 17.5, 1.0).unwrap_or_else(|error| panic!("{source}: {error}"));
            let image = eframe::icon_data::from_png_bytes(&png).unwrap();
            assert!(
                image.rgba.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0),
                "{source}"
            );
        }
    }

    #[test]
    fn dynamic_substitutions_are_preserved_and_rendered() {
        for source in [
            "1 / [exp(22135.3/1273.15) − 1] − (1 − 0.95) / [exp(22135.3/293.15) − 1]",
            "22135.3 / ln(1 + 0.95/2.8e-8)",
            "0.0015 × ∛0.25 + 0.0015 × √0.25 + 0.015 × 0.05^(2/3)",
            "611.2 × exp[22.46 × -20 / (272.62 + -20)]",
            "611.2 × exp[17.62 × 0.5 / (243.12 + 0.5)]",
            "10^[(10.286 × 293.15 − 2148.4909) / (293.15 − 35.85)]",
        ] {
            let (_, matched, tex, _) = tex_formula(source).expect("应识别动态代入");
            assert_eq!(matched, source);
            assert!(render_formula(&tex, 17.5, 1.0).is_ok(), "{source}");
        }
        let (_, _, tex, _) = tex_formula("22135.3 / ln(1 + 0.95/2.8e-8)").unwrap();
        assert_eq!(
            tex,
            r"\frac{22135.3}{\ln(1+\frac{0.95}{2.8\times 10^{-8}})}"
        );
        let source = "0.1 × exp[-0.2 × (10 − (-3))²]";
        let (_, matched, tex, _) = tex_formula(source).expect("负数代入仍应识别为高斯公式");
        assert_eq!(matched, source);
        assert!(tex.contains(r"10-\left(-3\right)"), "{tex}");
        assert!(render_formula(&tex, 17.5, 1.0).is_ok());
        let source = "1 / [exp(2/3) − 1] − (1 − (-0.5)) / [exp(2/4) − 1]";
        let (_, matched, tex, _) = tex_formula(source).expect("负数代入仍应识别为辐射公式");
        assert_eq!(matched, source);
        assert!(tex.contains(r"1-\left(-0.5\right)"), "{tex}");
        assert!(render_formula(&tex, 17.5, 1.0).is_ok());
    }

    #[test]
    fn atmosphere_and_radiation_substitutions_render_as_whole_formulas() {
        for source in [
            "101325 × exp[−9.80665 × (1000−0) / (287.05287 × 288.15)]",
            "101325 × [(288.15 + -0.0065 × (1000−0)) / 288.15]^(−9.80665 / (287.05287 × -0.0065))",
            "0.9996 × 1⁴ + 0.0004 × 1e-8⁴",
            "0.99 × 1⁴ + (1−0.99) × [0.98 × 0.9⁴ + (1−0.98) × 0.99 × 1⁴]",
        ] {
            let (_, matched, tex, _) = tex_formula(source).expect("应识别完整的数值代入公式");
            assert_eq!(matched, source);
            assert!(render_formula(&tex, 17.5, 1.0).is_ok(), "{source}: {tex}");
        }
        let (_, _, tex, _) = tex_formula(
            "101325 × [(288.15 + -0.0065 × (1000−0)) / 288.15]^(−9.80665 / (287.05287 × -0.0065))",
        )
        .unwrap();
        assert!(tex.contains(r"\left(-0.0065\right)"), "{tex}");
        let source = "1 × -2⁴ + 3 × 4⁴";
        assert!(tex_formula(source).is_none_or(|(_, matched, _, _)| matched != source));
    }

    #[test]
    fn several_formulas_in_the_same_clause_are_all_rendered() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, "本段参考函数 R/R₀ = 倍率 × Σcᵢzⁱ；z = t + 偏移。");
            });
        });
        let images = output.shapes.iter().filter(|shape| matches!(
            &shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default()
        )).count();
        assert_eq!(images, 2);
    }

    #[test]
    fn rendered_latex_aligns_with_its_text() {
        for line in [
            "升角 = atan[L/(πD₂)] × 180/π = 4.85°。",
            "参考函数：E(t) = c₀ + c₁t + c₂t² + … + cₙtⁿ；t 用 °C。",
            "指数项 = 0.1185976 × exp[-0.0001183432 × (100 − 126.9686)²] = 0.1 mV。",
        ] {
            let ctx = egui::Context::default();
            crate::fonts::configure_fonts(&ctx);
            crate::configure_style(&ctx);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| show(ui, line));
            });
            let text_baseline = |expected: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == expected => {
                            Some(text.pos.y + text.galley.rows[0].glyphs[0].pos.y)
                        }
                        _ => None,
                    })
                    .expect("公式两侧文字应可见")
            };
            let image_top = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) if !mesh.vertices.is_empty() => {
                        Some(mesh.calc_bounds().top())
                    }
                    _ => None,
                })
                .expect("应绘制公式图像");
            let (before, _, tex, after) = tex_formula(line).unwrap();
            let font_size = TextStyle::Body.resolve(&ctx.style()).size * 1.30;
            let (_, baseline, _) = render_formula(&tex, font_size, ctx.pixels_per_point()).unwrap();
            assert!(
                (text_baseline(before) - (image_top + baseline)).abs() < 1.0,
                "{line}"
            );
            assert!(
                (text_baseline(after) - (image_top + baseline)).abs() < 1.0,
                "{line}"
            );
        }
    }

    #[test]
    fn formula_texture_is_aligned_to_physical_pixels() {
        for density in [1.0, 1.25, 1.5, 2.0] {
            let ctx = egui::Context::default();
            crate::fonts::configure_fonts(&ctx);
            crate::configure_style(&ctx);
            ctx.set_pixels_per_point(density);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, "样本标准差 s = √[Σvᵢ²/(n − 1)] = 0.01。");
                });
            });
            let bounds = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                        Some(mesh.calc_bounds())
                    }
                    _ => None,
                })
                .expect("应绘制公式纹理");
            let actual_density = ctx.pixels_per_point();
            for coordinate in [bounds.left(), bounds.top(), bounds.width(), bounds.height()] {
                let pixel = coordinate * actual_density;
                assert!(
                    (pixel - pixel.round()).abs() < 0.01,
                    "DPI {density}: {bounds:?}"
                );
            }
        }
    }

    #[test]
    fn separate_formulas_on_the_same_line_are_both_drawn() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, "D₂=D−3√3·P/8；D₁=D−5√3·P/8。");
            });
        });
        assert_eq!(output.shapes.iter().filter(|shape| matches!(
            &shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default()
        )).count(), 2);
        assert_eq!(
            visible_text_fragments("D₂=D−3√3·P/8；D₁=D−5√3·P/8。"),
            vec!["D₂=D−", "；", "D₁=D−", "。"]
        );
    }

    #[test]
    fn symbolic_fraction_is_generated_by_math_engine() {
        let ctx = egui::Context::default();
        crate::fonts::configure_fonts(&ctx);
        crate::configure_style(&ctx);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| show(ui, "误差 = Δ/真值 × 100"));
        });
        let (_, source, tex, _) = tex_formula("误差 = Δ/真值 × 100").unwrap();
        assert_eq!(source, "Δ/真值");
        assert_eq!(tex, r"\frac{\Delta }{\text{真}\text{值}}");
        let (_, source, tex, _) = tex_formula("u = 2/√2.5").unwrap();
        assert_eq!(source, "2/√2.5");
        assert_eq!(tex, r"\frac{2}{\sqrt{2.5}}");
        assert_eq!(tex_operand("log₁₀(e)"), r"log_{10}(e)");
        let (_, source, tex, _) = tex_formula("目标值 = (373.15 − 0) / 1").unwrap();
        assert_eq!(source, "(373.15 − 0) / 1");
        assert_eq!(tex, r"\frac{(373.15 - 0)}{1}");
        let (_, source, tex, _) = tex_formula("u = 1/√4").unwrap();
        assert_eq!(source, "1/√4");
        assert_eq!(tex, r"\frac{1}{\sqrt{4}}");
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default()
        )));
    }
}
