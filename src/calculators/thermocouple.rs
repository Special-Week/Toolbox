//! NIST Monograph 175 / SRD 60 forward reference functions, in °C and mV.
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
    fn value_and_slope(&self, t: f64) -> (f64, f64) {
        let mut value = 0.0;
        let mut slope = 0.0;
        for &c in self.coefficients.iter().rev() {
            slope = slope * t + value;
            value = value * t + c;
        }
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
pub(crate) fn bounds(kind: &str) -> Result<(f64, f64), String> {
    let r = reference(kind)?;
    Ok((r.segments[0].min, r.segments.last().unwrap().max))
}

pub(crate) fn thermocouple_temperature(kind: &str, emf: f64) -> Result<f64, String> {
    if !emf.is_finite() {
        return Err("热电势必须是有限数字".to_owned());
    }
    let reference = reference(kind)?;
    let mut lower = reference.segments[0].min;
    let mut upper = reference.segments.last().unwrap().max;
    if kind == "B" {
        // Type B has a minimum near 21 °C. Negative/zero emf can have two
        // temperatures; positive emf is invertible without a 250 °C cutoff.
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
    if emf < low_emf || emf > high_emf {
        return Err(format!("热电势超出 {kind} 型参考函数范围"));
    }
    if emf == low_emf {
        return Ok(lower);
    }
    if emf == high_emf {
        return Ok(upper);
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
    let compensated = measured + correction + thermocouple_emf(kind, cold)?;
    Ok((thermocouple_temperature(kind, compensated)?, compensated))
}
