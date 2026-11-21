//! NIST Monograph 175 / SRD 60 正向参考函数，单位为 °C 和 mV。
//! 只嵌入高精度正向函数系数，不使用三位小数表或近似逆函数参与计算。
use std::sync::OnceLock;
#[path = "thermocouple/extra.rs"]
mod extra;
#[path = "thermocouple/standard.rs"]
pub(crate) mod standard;
#[cfg(feature = "desktop-ui")]
#[path = "thermocouple/standard_page.rs"]
mod standard_page;
#[path = "thermocouple/tolerance.rs"]
pub(crate) mod tolerance;

pub(crate) fn slope(kind: &str, t: f64) -> Result<f64, String> {
    let r = reference(kind)?;
    r.segments
        .iter()
        .find(|s| (s.min..=s.max).contains(&t))
        .map(|s| s.value_and_slope(t).1)
        .ok_or_else(|| "温度超出分度范围".into())
}

struct Segment {
    min: f64,
    max: f64,
    coefficients: Vec<f64>,
    gaussian: Option<[f64; 3]>,
}

impl Segment {
    fn polynomial_and_slope(&self, t: f64) -> (f64, f64) {
        let mut value = 0.0;
        let mut slope = 0.0;
        for &c in self.coefficients.iter().rev() {
            slope = slope * t + value;
            value = value * t + c;
        }
        (value, slope)
    }

    fn value_and_slope(&self, t: f64) -> (f64, f64) {
        let (mut value, mut slope) = self.polynomial_and_slope(t);
        // K 型非负温区包含额外高斯项，导数必须同时包含该项。
        if let Some([a0, a1, a2]) = self.gaussian {
            let exponential = a0 * (a1 * (t - a2).powi(2)).exp();
            value += exponential;
            slope += exponential * 2.0 * a1 * (t - a2);
        }
        (value, slope)
    }
}

struct Reference {
    segments: Vec<Segment>,
}
type CachedReference = OnceLock<Result<Reference, String>>;
static REFERENCES: [CachedReference; 8] = [const { OnceLock::new() }; 8];

fn reference(kind: &str) -> Result<&'static Reference, String> {
    let (index, bytes): (usize, &[u8]) = match kind {
        "B" => (0, include_bytes!("../../data/thermocouple/type_b.ref.txt")),
        "E" => (1, include_bytes!("../../data/thermocouple/type_e.ref.txt")),
        "J" => (2, include_bytes!("../../data/thermocouple/type_j.ref.txt")),
        "K" => (3, include_bytes!("../../data/thermocouple/type_k.ref.txt")),
        "N" => (4, include_bytes!("../../data/thermocouple/type_n.ref.txt")),
        "R" => (5, include_bytes!("../../data/thermocouple/type_r.ref.txt")),
        "S" => (6, include_bytes!("../../data/thermocouple/type_s.ref.txt")),
        "T" => (7, include_bytes!("../../data/thermocouple/type_t.ref.txt")),
        _ => return extra::get(kind),
    };
    REFERENCES[index]
        .get_or_init(|| parse_reference(&String::from_utf8_lossy(bytes)))
        .as_ref()
        .map_err(Clone::clone)
}

/// 系数顺序与分段边界按 NIST 原始格式读取，一次初始化后缓存，不在每帧重解析。
fn parse_reference(source: &str) -> Result<Reference, String> {
    let failure = || "NIST 热电偶参考系数无法读取".to_owned();
    let (_, source) = source
        .split_once("name: reference function on ITS-90")
        .ok_or_else(failure)?;
    let mut lines = source.lines();
    let mut segments: Vec<Segment> = Vec::new();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.starts_with("Inverse coefficients") {
            break;
        }
        if let Some(range) = line.strip_prefix("range:") {
            let parts: Vec<_> = range.split(',').map(str::trim).collect();
            if parts.len() != 3 {
                return Err(failure());
            }
            let min = parts[0].parse::<f64>().map_err(|_| failure())?;
            let max = parts[1].parse::<f64>().map_err(|_| failure())?;
            let degree = parts[2].parse::<usize>().map_err(|_| failure())?;
            if degree > 20 {
                return Err(failure());
            }
            let mut coefficients = Vec::with_capacity(degree + 1);
            for _ in 0..=degree {
                let coefficient = lines
                    .next()
                    .ok_or_else(failure)?
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| failure())?;
                if !coefficient.is_finite() {
                    return Err(failure());
                }
                coefficients.push(coefficient);
            }
            segments.push(Segment {
                min,
                max,
                coefficients,
                gaussian: None,
            });
        } else if line == "exponential:" {
            let mut gaussian = [0.0; 3];
            for value in &mut gaussian {
                *value = lines
                    .next()
                    .and_then(|line| line.split_once('='))
                    .and_then(|(_, value)| value.trim().parse::<f64>().ok())
                    .ok_or_else(failure)?;
            }
            segments.last_mut().ok_or_else(failure)?.gaussian = Some(gaussian);
        }
    }
    if segments.is_empty() {
        return Err(failure());
    }
    Ok(Reference { segments })
}

impl Reference {
    fn emf(&self, t: f64) -> Result<f64, String> {
        if !t.is_finite() {
            return Err("温度必须是有限数字".to_owned());
        }
        let segment = self
            .segments
            .iter()
            .find(|s| (s.min..=s.max).contains(&t))
            .ok_or_else(|| {
                format!(
                    "温度应在 {}～{} °C 之间",
                    self.segments[0].min,
                    self.segments.last().unwrap().max
                )
            })?;
        Ok(segment.value_and_slope(t).0)
    }
}

pub(crate) fn thermocouple_emf(kind: &str, t: f64) -> Result<f64, String> {
    reference(kind)?.emf(t)
}

/// 过程展示直接读取正算使用的分段和系数，连接点沿用正算的先匹配规则。
pub(crate) fn reference_process(
    kind: &str,
    t: f64,
    label: &str,
    calculated_temperature: bool,
) -> Result<Vec<String>, String> {
    use crate::calculators::common::{format_process_computed as c, format_process_number as n};
    let shown_t = if calculated_temperature { c(t) } else { n(t) };
    let reference = reference(kind)?;
    let value = reference.emf(t)?;
    let segment = reference
        .segments
        .iter()
        .find(|s| (s.min..=s.max).contains(&t))
        .unwrap();
    let polynomial = segment.polynomial_and_slope(t).0;
    let mut lines = vec![
        format!(
            "{label}：{} 型，t = {} °C；实际使用分段 {}～{} °C。",
            kind,
            shown_t.clone(),
            n(segment.min),
            n(segment.max)
        ),
        "参考函数：E(t) = c₀ + c₁t + c₂t² + … + cₙtⁿ；t 以 °C 数值代入，E 的单位为 mV。".to_owned(),
    ];
    lines.push(
        if matches!(kind, "B" | "E" | "J" | "K" | "N" | "R" | "S" | "T") {
            "系数来源：项目内置的 NIST ITS-90 参考函数表。"
        } else {
            "使用项目内置的扩展分度曲线；系数已统一为 mV。"
        }
        .to_owned(),
    );
    lines.extend(
        segment
            .coefficients
            .iter()
            .enumerate()
            .map(|(i, c)| format!("c[{i}] = {}", n(*c))),
    );
    lines.push(format!(
        "将 t = {} 代入多项式（按秦九韶法求值），得到多项式部分 {} mV。",
        shown_t.clone(),
        c(polynomial)
    ));
    if let Some([a0, a1, a2]) = segment.gaussian {
        let extra = a0 * (a1 * (t - a2).powi(2)).exp();
        lines.push(format!(
            "本段还需加指数项 a₀·exp[a₁(t−a₂)²]；a₀ = {}，a₁ = {}，a₂ = {}。",
            n(a0),
            n(a1),
            n(a2)
        ));
        lines.push(format!(
            "指数项 = {} × exp[{} × ({} − {})²] = {} mV。",
            n(a0),
            n(a1),
            shown_t.clone(),
            n(a2),
            c(extra)
        ));
        lines.push(format!(
            "E({}) = 多项式部分 + 指数项 = {} + {} = {} mV。",
            shown_t.clone(),
            c(polynomial),
            c(extra),
            c(value)
        ));
    } else {
        lines.push(format!(
            "本段无附加指数项，E({}) = {} mV。",
            shown_t.clone(),
            c(value)
        ));
    }
    Ok(lines)
}
pub(crate) fn bounds(kind: &str) -> Result<(f64, f64), String> {
    let r = reference(kind)?;
    Ok((r.segments[0].min, r.segments.last().unwrap().max))
}

#[cfg(test)]
fn thermocouple_temperature(kind: &str, emf: f64) -> Result<f64, String> {
    temperature_with_roundoff(kind, emf, 0.0)
}

fn temperature_with_roundoff(kind: &str, emf: f64, roundoff: f64) -> Result<f64, String> {
    if !emf.is_finite() {
        return Err("热电势必须是有限数字".to_owned());
    }
    let reference = reference(kind)?;
    let mut lower = reference.segments[0].min;
    let mut upper = reference.segments.last().unwrap().max;
    if kind == "B" {
        // B 型曲线在约 21 °C 处有最小值；负电势或零电势可能对应两个温度，
        // 正电势可以反算，无需设置 250 °C 的截断点。
        if emf <= 0.0 {
            return Err("B 型在低温区有转折，此非正热电势无法唯一反算温度".to_owned());
        }
        let mut left = 0.0;
        let mut right = 50.0;
        for _ in 0..64 {
            let middle = (left + right) * 0.5;
            if reference.segments[0].value_and_slope(middle).1 < 0.0 {
                left = middle;
            } else {
                right = middle;
            }
        }
        lower = (left + right) * 0.5;
    }
    let low_emf = reference.emf(lower)?;
    let high_emf = reference.emf(upper)?;
    if (emf < low_emf && low_emf - emf > roundoff)
        || (emf > high_emf && emf - high_emf > roundoff)
    {
        return Err(format!("热电势超出 {kind} 型参考函数范围"));
    }
    if (emf - low_emf).abs() <= roundoff {
        return Ok(lower);
    }
    if (emf - high_emf).abs() <= roundoff {
        return Ok(upper);
    }
    // 分段多项式在连接处存在微小数值差异。连接点本身若精确命中，
    // 直接返回该点，避免二分迭代收敛到另一段的一侧后误报无解。
    for segment in &reference.segments {
        let join = segment.max;
        if join > lower && join < upper && (reference.emf(join)? - emf).abs() <= roundoff {
            return Ok(join);
        }
    }
    for _ in 0..80 {
        let middle = lower + (upper - lower) * 0.5;
        if reference.emf(middle)? < emf {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    let t = lower + (upper - lower) * 0.5;
    if (reference.emf(t)? - emf).abs() > 1e-9 * emf.abs().max(1.0) {
        return Err("该电势位于分段曲线的不连续区间，无法可靠反算温度".into());
    }
    Ok(t)
}

/// correction 以“应加到实测电势上的修正量”定义，正算时反向扣除。
pub(crate) fn measured_emf(
    kind: &str,
    hot: f64,
    cold: f64,
    correction: f64,
) -> Result<f64, String> {
    let emf = thermocouple_emf(kind, hot)? - thermocouple_emf(kind, cold)? - correction;
    if !emf.is_finite() {
        return Err("热电势计算溢出".to_owned());
    }
    Ok(emf)
}

/// 先加电势修正和参考端电势，再用相同正向曲线反算热端温度。
pub(crate) fn corrected_temperature(
    kind: &str,
    measured: f64,
    cold: f64,
    correction: f64,
) -> Result<(f64, f64), String> {
    let cold_emf = thermocouple_emf(kind, cold)?;
    let compensated = measured + correction + cold_emf;
    let scale = measured.abs().max(correction.abs()).max(cold_emf.abs());
    let roundoff = scale * (4.0 * f64::EPSILON);
    Ok((temperature_with_roundoff(kind, compensated, roundoff)?, compensated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compensation_preserves_valid_range_endpoints() {
        for (kind, hot) in [("E", -270.0), ("S", -50.0), ("G", 630.615)] {
            for correction in [0.0, 0.01, -0.1] {
                let measured = measured_emf(kind, hot, 100.0, correction).unwrap();
                assert_eq!(corrected_temperature(kind, measured, 100.0, correction).unwrap().0, hot);
            }
        }
        let measured = measured_emf("E", -270.0, 100.0, 0.0).unwrap();
        assert!(corrected_temperature("E", measured - 1e-8, 100.0, 0.0).is_err());
        assert!(corrected_temperature("B", 0.0, 0.0, 0.0).is_err());
    }

    #[test]
    #[cfg(feature = "desktop-ui")]
    fn displayed_reference_segments_and_gaussian_match_the_calculation() {
        let lines = reference_process("K", 100.0, "热端", false).unwrap();
        assert!(lines.iter().any(|s| s.contains("实际使用分段 0～1372 °C")));
        assert!(lines.iter().any(|s| s == "c[0] = -0.017600413686"));
        assert!(
            lines
                .iter()
                .any(|s| s.contains("指数项 =") && s.contains("exp"))
        );
        let value = thermocouple_emf("K", 100.0).unwrap();
        assert!((value - 4.096230218723254).abs() < 1e-12);
        assert!(
            lines
                .iter()
                .any(|s| s.contains(&crate::calculators::common::format_process_number(value)))
        );
        let zero = reference_process("K", 0.0, "参考端", false).unwrap();
        assert!(zero.iter().any(|s| s.contains("实际使用分段 -270～0 °C")));
        assert!(zero.iter().any(|s| s.contains("无附加指数项")));

        for kind in [
            "B", "E", "J", "K", "N", "R", "S", "T", "C", "D", "G", "L", "LR", "U", "A",
        ] {
            let reference = reference(kind).unwrap();
            for segment in &reference.segments {
                for t in [
                    segment.min,
                    segment.min + (segment.max - segment.min) / 2.0,
                    segment.max,
                ] {
                    let selected = reference
                        .segments
                        .iter()
                        .find(|item| (item.min..=item.max).contains(&t))
                        .unwrap();
                    let lines = reference_process(kind, t, "校验", false).unwrap();
                    let count = lines.iter().filter(|s| s.starts_with("c[")).count();
                    assert_eq!(count, selected.coefficients.len(), "{kind} {t}");
                    for (index, coefficient) in selected.coefficients.iter().enumerate() {
                        assert!(
                            lines.contains(&format!(
                                "c[{index}] = {}",
                                crate::calculators::common::format_process_number(*coefficient)
                            )),
                            "{kind} {t} {index}"
                        );
                    }
                    assert_eq!(
                        lines.iter().any(|s| s.contains("指数项 =")),
                        selected.gaussian.is_some(),
                        "{kind} {t}"
                    );
                    assert!(lines.iter().any(|s| s.contains(
                        &crate::calculators::common::format_process_number(
                            reference.emf(t).unwrap()
                        )
                    )));
                }
            }
        }
    }

    #[test]
    fn type_k_zero_and_near_zero_round_trip() {
        assert_eq!(thermocouple_temperature("K", 0.0).unwrap(), 0.0);
        for temperature in [-0.00001, 0.0, 0.00001, 100.0] {
            let emf = thermocouple_emf("K", temperature).unwrap();
            let restored = thermocouple_temperature("K", emf).unwrap();
            assert!(
                (restored - temperature).abs() < 1e-7,
                "{temperature}: {restored}"
            );
        }
    }

    #[test]
    fn all_reference_segment_boundaries_and_midpoints_round_trip() {
        for kind in [
            "B", "E", "J", "K", "N", "R", "S", "T", "C", "D", "G", "L", "LR", "U", "A",
        ] {
            let r = reference(kind).unwrap();
            for segment in &r.segments {
                for temperature in [segment.min, (segment.min + segment.max) * 0.5, segment.max] {
                    let emf = thermocouple_emf(kind, temperature).unwrap();
                    if kind == "B" && emf <= 0.0 {
                        continue;
                    }
                    let restored = thermocouple_temperature(kind, emf)
                        .unwrap_or_else(|error| panic!("{kind}, {temperature} °C: {error}"));
                    assert!(
                        (restored - temperature).abs() < 1e-6,
                        "{kind}, {temperature} °C: {restored}"
                    );
                }
            }
        }
    }

    #[test]
    fn ambiguous_type_b_and_nonfinite_emf_stay_invalid() {
        for emf in [0.0, -0.001, f64::NAN, f64::INFINITY] {
            assert!(thermocouple_temperature("B", emf).is_err());
        }
        assert!(thermocouple_temperature("K", f64::NAN).is_err());
    }
}
