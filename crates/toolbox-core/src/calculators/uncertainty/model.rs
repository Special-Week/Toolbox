//! 不确定度分量树：先求各分量标准不确定度，再应用灵敏系数并逐级合成。
#![allow(clippy::items_after_test_module)]
use crate::calculators::{
    numerics::{finite, rss, statistics},
    range_method::coefficient,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DegreesOfFreedom {
    Infinite,
    Finite(f64),
    NotApplicable,
}

pub enum Kind {
    TypeA {
        values: Vec<f64>,
        range_method: bool,
        average_count: Option<usize>,
    },
    TypeB {
        half_width: f64,
        distribution: usize,
        dof: Option<f64>,
    },
    Combined {
        children: Vec<Component>,
        maximum: bool,
    },
}
/// 留空时使用一次测量；取测量次数必须是正整数。
pub fn average_count(text: &str) -> Result<Option<usize>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("取测量次数 n 须为正整数".into());
    }
    match text.parse::<usize>() {
        Ok(n) if n > 0 => Ok(Some(n)),
        _ => Err("取测量次数 n 须为正整数".into()),
    }
}
pub struct Component {
    pub name: String,
    pub sensitivity: f64,
    pub kind: Kind,
    pub rounding: Option<Rounding>,
}
#[derive(Clone, Copy)]
pub struct Rounding {
    pub digits: Option<usize>,
    pub carry: bool,
}
pub fn significant_digits(text: &str) -> Result<Option<usize>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("有效数字须为 1～17 的整数，留空保留原始计算精度".into());
    }
    match text.parse::<usize>() {
        Ok(n @ 1..=17) => Ok(Some(n)),
        _ => Err("有效数字须为 1～17 的整数，留空保留原始计算精度".into()),
    }
}
pub struct Detail {
    pub name: String,
    pub raw_standard: f64,
    pub standard: f64,
    pub sensitivity: f64,
    pub contribution: f64,
    pub dof: DegreesOfFreedom,
    pub children: Vec<Detail>,
    pub rounding: Option<Rounding>,
}

/// B 类自由度解析：留空或输入 inf/∞ 表示无穷大，其余须为有限正实数。
pub fn parse_dof(text: &str) -> Result<Option<f64>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed == "∞"
        || trimmed.eq_ignore_ascii_case("inf")
        || trimmed.eq_ignore_ascii_case("infinity")
    {
        return Ok(None);
    }
    let value = trimmed
        .parse::<f64>()
        .map_err(|_| "自由度须为大于 0 的数，留空表示 ∞")?;
    if !value.is_finite() || value <= 0.0 {
        return Err("自由度须为大于 0 的数，留空表示 ∞".into());
    }
    Ok(Some(value))
}

/// 极差法（2～9 次独立测量）等效自由度（JJF 1059.1 近似表：0.9·(m−1)）。
pub fn range_dof(n: usize) -> Result<f64, String> {
    const TABLE: [f64; 8] = [0.9, 1.8, 2.7, 3.6, 4.5, 5.4, 6.3, 7.2];
    n.checked_sub(2)
        .and_then(|i| TABLE.get(i))
        .copied()
        .ok_or_else(|| "极差法支持 2～9 次测量".into())
}

pub fn format_dof(dof: DegreesOfFreedom) -> String {
    match dof {
        DegreesOfFreedom::Infinite => "∞".into(),
        DegreesOfFreedom::Finite(v) => crate::calculators::format::format_number(v),
        DegreesOfFreedom::NotApplicable => "不适用".into(),
    }
}

/// Welch–Satterthwaite 有效自由度；先除以自由度的四次方根，再累加平方和根。
pub fn combine_dof(
    terms: &[(f64, DegreesOfFreedom)],
    maximum: bool,
) -> Result<DegreesOfFreedom, String> {
    if terms.is_empty() {
        return Err("至少需要一个不确定度分量".into());
    }
    if maximum {
        return Ok(DegreesOfFreedom::NotApplicable);
    }
    let contributions: Vec<f64> = terms.iter().map(|(q, _)| *q).collect();
    let uc = rss(contributions.iter().copied())?;
    if uc == 0.0 {
        return Ok(DegreesOfFreedom::NotApplicable);
    }
    let mut norm = 0.0_f64;
    for &(q, dof) in terms {
        if q == 0.0 {
            continue;
        }
        match dof {
            DegreesOfFreedom::NotApplicable => return Ok(DegreesOfFreedom::NotApplicable),
            DegreesOfFreedom::Infinite => {}
            DegreesOfFreedom::Finite(v) => {
                if !v.is_finite() || v <= 0.0 {
                    return Err("自由度须为大于 0 的数".into());
                }
                let weight = (q / uc).abs() / v.sqrt().sqrt();
                norm = norm.hypot(weight * weight);
            }
        }
    }
    let eff = (1.0 / norm) / norm;
    if !eff.is_finite() {
        Ok(DegreesOfFreedom::Infinite)
    } else if eff <= 0.0 {
        Err("有效自由度超出双精度数值范围".into())
    } else {
        Ok(DegreesOfFreedom::Finite(eff))
    }
}

/// Lanczos 近似求 ln(Γ(z))，z > 0；通过递推平移至 z >= 1.5 确保全定义域满双精度。
fn ln_gamma(mut z: f64) -> f64 {
    const COEFFS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    let mut shift = 0.0;
    while z < 1.5 {
        shift -= z.ln();
        z += 1.0;
    }
    let z = z - 1.0;
    let mut x = COEFFS[0];
    for (i, &c) in COEFFS.iter().enumerate().skip(1) {
        x += c / (z + i as f64);
    }
    let t = z + 7.5;
    shift + 0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

/// 修改 Lentz 算法求不完全 Beta 函数的连分数部分。
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    const FPMIN: f64 = 1e-300;
    const EPS: f64 = 1e-15;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=200 {
        let mf = m as f64;
        let m2 = 2.0 * mf;
        let aa = mf * (b - mf) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + mf) * (qab + mf) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// 对数域计算学生 t 分布双侧尾概率 P(|T_ν| > e^y) = I_x(ν/2, 1/2)，避免极小 x 下溢。
fn student_t_tail(a: f64, ln_v: f64, y: f64) -> f64 {
    let s = 2.0 * y - ln_v;
    let (x, ln_x, ln_1mx) = if s > 36.0 {
        let x = (-s).exp();
        (x, -s, -x)
    } else if s < -36.0 {
        let es = s.exp();
        (1.0 - es, -es, s)
    } else {
        let es = s.exp();
        let ln_denom = es.ln_1p();
        (1.0 / (1.0 + es), -ln_denom, s - ln_denom)
    };
    let ln_half_pi = 0.5 * std::f64::consts::PI.ln();
    if x < (a + 1.0) / (a + 2.5) {
        // 用 Γ(a+1) = a·Γ(a) 吸收分母 a，使极小自由度 a → 0+ 无 0/0 或 ln(0) 奇异。
        let log_pref = ln_gamma(a + 0.5) - ln_gamma(a + 1.0) - ln_half_pi + a * ln_x + 0.5 * ln_1mx;
        log_pref.exp() * beta_cf(a, 0.5, x)
    } else {
        let log_pref =
            ln_gamma(a + 0.5) - ln_gamma(a) - ln_gamma(1.5) + 0.5 * ln_1mx + a * ln_x;
        1.0 - log_pref.exp() * beta_cf(0.5, a, 1.0 - x)
    }
}

/// 学生 t 分布双侧分位数 t_p(ν)，支持连续实数自由度 ν > 0；p ∈ {0.95, 0.99}。
pub fn student_t_quantile(dof: DegreesOfFreedom, confidence: f64) -> Result<f64, String> {
    let z = if (confidence - 0.95).abs() < 1e-12 {
        1.959963984540054
    } else if (confidence - 0.99).abs() < 1e-12 {
        2.5758293035489004
    } else {
        return Err("仅支持 95% 或 99% 置信概率".into());
    };
    match dof {
        DegreesOfFreedom::NotApplicable => {
            Err("当前合成模式或零不确定度不适用有效自由度，请改用手动指定包含因子 k".into())
        }
        DegreesOfFreedom::Infinite => Ok(z),
        DegreesOfFreedom::Finite(v) => {
            if !v.is_finite() || v <= 0.0 {
                return Err("有效自由度须为大于 0 的数".into());
            }
            if v >= 1e5 {
                let z2 = z * z;
                let z3 = z2 * z;
                let z5 = z3 * z2;
                return Ok(
                    z + (z3 + z) / (4.0 * v) + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * v * v),
                );
            }
            let alpha = 1.0 - confidence;
            let a = 0.5 * v;
            let ln_v = v.ln();
            if a == 0.0 || !ln_v.is_finite() {
                return Err("有效自由度过小，t 分布包含因子超出双精度数值范围".into());
            }
            let mut low = z.ln();
            let mut high = f64::MAX.ln();
            let tail_high = student_t_tail(a, ln_v, high);
            if !tail_high.is_finite() || tail_high > alpha {
                return Err("有效自由度过小，t 分布包含因子超出双精度数值范围".into());
            }
            for _ in 0..80 {
                let mid = 0.5 * (low + high);
                if student_t_tail(a, ln_v, mid) > alpha {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            let y = 0.5 * (low + high);
            let residual = (student_t_tail(a, ln_v, y) - alpha).abs();
            let t = y.exp();
            if !t.is_finite() || !residual.is_finite() || residual > 1e-9 * alpha {
                return Err("有效自由度过小，t 分布包含因子超出双精度数值范围".into());
            }
            Ok(t)
        }
    }
}

/// 根据包含因子模式计算 k：0=手动指定，1=t 分布近似（95%），2=t 分布近似（99%）。
pub fn coverage_factor(
    mode: usize,
    manual_k: f64,
    dof: DegreesOfFreedom,
) -> Result<f64, String> {
    match mode {
        1 => student_t_quantile(dof, 0.95),
        2 => student_t_quantile(dof, 0.99),
        _ => {
            finite(&[manual_k])?;
            if manual_k <= 0.0 {
                return Err("包含因子须大于 0".into());
            }
            Ok(manual_k)
        }
    }
}
/// UI 分布索引依次为均匀、三角、正态 k=2、正态 k=3、两点和反正弦。
/// 输入是区间半宽 a，标准不确定度为 a/除数。
pub fn divisor(distribution: usize) -> f64 {
    match distribution {
        1 => 6.0_f64.sqrt(),
        2 => 2.0,
        3 => 3.0,
        4 => 1.0,
        5 => 2.0_f64.sqrt(),
        _ => 3.0_f64.sqrt(),
    }
}
/// maximum 只取主导贡献；默认 RSS 用于不相关分量，不自动引入相关系数。
pub fn combine(values: &[f64], maximum: bool) -> Result<f64, String> {
    if values.is_empty() {
        return Err("至少需要一个不确定度分量".into());
    }
    if maximum {
        finite(values)?;
        Ok(values.iter().map(|v| v.abs()).fold(0.0, f64::max))
    } else {
        rss(values.iter().copied())
    }
}
impl Component {
    /// 子分量已应用自己的灵敏系数；父节点仅在合成后再应用父节点系数一次。
    pub fn calculate(&self) -> Result<Detail, String> {
        finite(&[self.sensitivity])?;
        let mut children = Vec::new();
        let (standard, dof) = match &self.kind {
            Kind::TypeA {
                values,
                range_method,
                average_count,
            } => {
                let stats = statistics(values)?;
                if *range_method {
                    (
                        (stats.max - stats.min)
                            / coefficient(values.len())?
                            / (values.len() as f64).sqrt(),
                        DegreesOfFreedom::Finite(range_dof(values.len())?),
                    )
                } else {
                    let std_val = match average_count {
                        None => stats.sd,
                        Some(0) => return Err("取测量次数 n 须为正整数".into()),
                        Some(count) => stats.sd / (*count as f64).sqrt(),
                    };
                    (std_val, DegreesOfFreedom::Finite((values.len() - 1) as f64))
                }
            }
            Kind::TypeB {
                half_width,
                distribution,
                dof,
            } => {
                finite(&[*half_width])?;
                if *half_width < 0.0 {
                    return Err("B 类半宽不能为负数".into());
                }
                let component_dof = match dof {
                    None => DegreesOfFreedom::Infinite,
                    Some(v) if v.is_finite() && *v > 0.0 => DegreesOfFreedom::Finite(*v),
                    Some(_) => return Err("自由度须为大于 0 的数，留空表示 ∞".into()),
                };
                (half_width / divisor(*distribution), component_dof)
            }
            Kind::Combined {
                children: items,
                maximum,
            } => {
                for c in items {
                    children.push(c.calculate()?);
                }
                let std_val = combine(
                    &children.iter().map(|c| c.contribution).collect::<Vec<_>>(),
                    *maximum,
                )?;
                let terms: Vec<_> = children.iter().map(|c| (c.contribution, c.dof)).collect();
                (std_val, combine_dof(&terms, *maximum)?)
            }
        };
        finite(&[standard])?;
        let raw_standard = standard;
        let standard = match self.rounding {
            Some(rounding) => round_significant(standard, rounding)?,
            None => standard,
        };
        let contribution = standard * self.sensitivity;
        finite(&[standard, contribution])?;
        Ok(Detail {
            name: self.name.clone(),
            raw_standard,
            standard,
            sensitivity: self.sensitivity,
            contribution,
            dof,
            children,
            rounding: self.rounding,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bessel_uses_one_by_default_or_custom_measurement_count() {
        let component = |average_count| Component {
            name: "A".into(),
            sensitivity: 1.0,
            rounding: None,
            kind: Kind::TypeA {
                values: vec![1.0, 2.0, 3.0],
                range_method: false,
                average_count,
            },
        };
        assert!((component(None).calculate().unwrap().standard - 1.0).abs() < 1e-12);
        assert!((component(Some(4)).calculate().unwrap().standard - 0.5).abs() < 1e-12);
        assert!(component(Some(0)).calculate().is_err());
        assert_eq!(average_count(" ").unwrap(), None);
        assert_eq!(average_count("4").unwrap(), Some(4));
        for invalid in ["0", "-1", "2.5", "abc", "184467440737095516160"] {
            assert!(average_count(invalid).is_err());
        }
    }

    #[test]
    fn decimal_rounding_uses_half_even_and_handles_boundaries() {
        for (value, places, expected) in [
            (2.345, 2, 2.34),
            (2.355, 2, 2.36),
            (2.3451, 2, 2.35),
            (2.3449, 2, 2.34),
            (1.005, 2, 1.0),
            (0.005, 2, 0.0),
            (0.015, 2, 0.02),
            (15.0, -1, 20.0),
            (25.0, -1, 20.0),
        ] {
            assert_eq!(rounded_decimal(value, places, false).unwrap(), expected);
        }
        assert_eq!(rounded_decimal(0.01, 2, true).unwrap(), 0.01);
        assert_eq!(rounded_decimal(0.01001, 2, true).unwrap(), 0.02);
        assert_eq!(rounded_decimal(1234.0, -2, true).unwrap(), 1300.0);
        assert!(
            round_significant(
                f64::MAX,
                Rounding {
                    digits: Some(2),
                    carry: true
                }
            )
            .is_err()
        );
        let tiny = f64::from_bits(1);
        assert_eq!(
            round_significant(
                tiny,
                Rounding {
                    digits: Some(1),
                    carry: true
                }
            )
            .unwrap(),
            tiny
        );
    }

    #[test]
    fn combined_rounding_uses_coverage_and_selected_method() {
        assert_eq!(rounded_combined(2.45, 1.0, 2, false).unwrap(), (2.4, 2.4));
        assert_eq!(rounded_combined(2.45, 1.0, 2, true).unwrap(), (2.5, 2.5));
        assert_eq!(rounded_combined(0.0, 2.0, 2, true).unwrap(), (0.0, 0.0));
        assert_eq!(
            rounded_combined(12345.0, 2.0, 2, true).unwrap(),
            (13000.0, 25000.0)
        );
        assert!(rounded_combined(1.0, 0.0, 2, true).is_err());
        assert!(rounded_combined(1.0, 2.0, 0, true).is_err());
        assert!(rounded_combined(f64::MAX, 2.0, 2, true).is_err());
    }

    #[test]
    fn component_rounding_precedes_sensitivity_and_group_combination() {
        let component = |value, sensitivity, digits, carry| Component {
            name: "分量".into(),
            sensitivity,
            kind: Kind::TypeB {
                half_width: value,
                distribution: 4,
                dof: None,
            },
            rounding: Some(Rounding { digits, carry }),
        };
        let no_rounding = component(1.23456789, 1.0, None, true).calculate().unwrap();
        assert_eq!(no_rounding.standard, 1.23456789);
        let first = component(1.235, 2.0, Some(3), false);
        let second = component(0.445, 1.0, Some(2), false);
        let group = Component {
            name: "组合".into(),
            sensitivity: 1.0,
            kind: Kind::Combined {
                children: vec![first, second],
                maximum: false,
            },
            rounding: Some(Rounding {
                digits: Some(2),
                carry: true,
            }),
        }
        .calculate()
        .unwrap();
        assert_eq!(group.children[0].standard, 1.24);
        assert_eq!(group.children[0].contribution, 2.48);
        assert_eq!(group.children[1].standard, 0.44);
        assert!((group.raw_standard - 2.48_f64.hypot(0.44)).abs() < 1e-12);
        assert_eq!(group.standard, 2.6);
        assert_eq!(
            format_rounded(
                1.2,
                Some(Rounding {
                    digits: Some(3),
                    carry: true
                })
            ),
            "1.20"
        );
        assert_eq!(
            format_rounded(
                0.0012,
                Some(Rounding {
                    digits: Some(3),
                    carry: true
                })
            ),
            "0.00120"
        );
        assert_eq!(significant_digits(" ").unwrap(), None);
        assert_eq!(significant_digits("17").unwrap(), Some(17));
        for invalid in ["0", "18", "-1", "2.5", "abc"] {
            assert!(significant_digits(invalid).is_err());
        }
    }

    #[test]
    fn welch_satterthwaite_and_student_t_quantiles_match_reference_tables() {
        // 标准 t 分布双侧 95% 和 99% 分位数对照（ν=1,2 解析闭式；其余用独立 PDF 数值积分 + 表值核验）
        let t95_1 = (0.5 * std::f64::consts::PI * 0.95).tan();
        let t99_1 = (0.5 * std::f64::consts::PI * 0.99).tan();
        let t95_2 = (2.0_f64 * 0.95 * 0.95 / (1.0 - 0.95 * 0.95)).sqrt();
        let t99_2 = (2.0_f64 * 0.99 * 0.99 / (1.0 - 0.99 * 0.99)).sqrt();
        for (dof, t95, t99) in [
            (1.0, t95_1, t99_1),
            (2.0, t95_2, t99_2),
            (4.0, 2.776445105, 4.604094871),
            (10.0, 2.228138852, 3.169272673),
            (30.0, 2.042272456, 2.749995654),
            (100.0, 1.983971518, 2.625890521),
        ] {
            let q95 = student_t_quantile(DegreesOfFreedom::Finite(dof), 0.95).unwrap();
            let q99 = student_t_quantile(DegreesOfFreedom::Finite(dof), 0.99).unwrap();
            assert!((q95 - t95).abs() < 1e-8, "ν={dof}: got {q95}, want {t95}");
            assert!((q99 - t99).abs() < 1e-8, "ν={dof}: got {q99}, want {t99}");
            // 独立复合辛普森积分核验 P(|T_ν| <= q) == p（覆盖非整数自由度）
            for (q, p) in [(q95, 0.95), (q99, 0.99)] {
                let norm = (ln_gamma(0.5 * (dof + 1.0)) - ln_gamma(0.5 * dof)).exp()
                    / (dof * std::f64::consts::PI).sqrt();
                let n = 4096_usize;
                let h = q / n as f64;
                let pdf = |x: f64| norm * (1.0 + x * x / dof).powf(-0.5 * (dof + 1.0));
                let mut sum = pdf(0.0) + pdf(q);
                for i in 1..n {
                    sum += if i % 2 == 0 { 2.0 } else { 4.0 } * pdf(i as f64 * h);
                }
                let prob = 2.0 * sum * h / 3.0;
                assert!((prob - p).abs() < 1e-11, "ν={dof}, p={p}: prob={prob}");
            }
        }
        // 非整数自由度（如极差法 m=2 => ν=0.9）同样通过独立积分验证
        let q09 = student_t_quantile(DegreesOfFreedom::Finite(0.9), 0.95).unwrap();
        let norm = (ln_gamma(0.95) - ln_gamma(0.45)).exp() / (0.9 * std::f64::consts::PI).sqrt();
        let n = 8192_usize;
        let h = q09 / n as f64;
        let pdf = |x: f64| norm * (1.0 + x * x / 0.9).powf(-0.95);
        let mut sum = pdf(0.0) + pdf(q09);
        for i in 1..n {
            sum += if i % 2 == 0 { 2.0 } else { 4.0 } * pdf(i as f64 * h);
        }
        assert!((2.0 * sum * h / 3.0 - 0.95).abs() < 1e-11);
        assert!(
            (student_t_quantile(DegreesOfFreedom::Infinite, 0.95).unwrap() - 1.959963984540054)
                .abs()
                < 1e-14
        );
        assert!(
            (student_t_quantile(DegreesOfFreedom::Infinite, 0.99).unwrap() - 2.5758293035489004)
                .abs()
                < 1e-14
        );
        // 大自由度平滑衔接 Cornish–Fisher 展开
        let q_large = student_t_quantile(DegreesOfFreedom::Finite(1e6), 0.95).unwrap();
        assert!((q_large - 1.95996635).abs() < 1e-7);
        // 缩放防溢出：大贡献 1e150 仍可精确算出有效自由度
        let eff = combine_dof(
            &[
                (3e150, DegreesOfFreedom::Finite(4.0)),
                (4e150, DegreesOfFreedom::Infinite),
            ],
            false,
        )
        .unwrap();
        // uc = 5e150, (3/5)^4 / 4 = 81/2500 => ν_eff = 2500/81
        match eff {
            DegreesOfFreedom::Finite(v) => assert!((v - 2500.0 / 81.0).abs() < 1e-11),
            other => panic!("expected finite dof, got {other:?}"),
        }
        assert_eq!(
            combine_dof(&[(1.0, DegreesOfFreedom::Finite(4.0))], true).unwrap(),
            DegreesOfFreedom::NotApplicable
        );
        assert!(student_t_quantile(DegreesOfFreedom::NotApplicable, 0.95).is_err());
        assert_eq!(parse_dof(" ").unwrap(), None);
        assert_eq!(parse_dof("∞").unwrap(), None);
        assert_eq!(parse_dof("inf").unwrap(), None);
        assert_eq!(parse_dof("12.5").unwrap(), Some(12.5));
        for invalid in ["0", "-2", "abc", "NaN"] {
            assert!(parse_dof(invalid).is_err());
        }
        // 小自由度（0.01, 0.05, 0.1）精确求解且保持严格单调，极小自由度溢出明确报错
        let q01 = student_t_quantile(DegreesOfFreedom::Finite(0.1), 0.95).unwrap();
        let q005 = student_t_quantile(DegreesOfFreedom::Finite(0.05), 0.95).unwrap();
        let q001 = student_t_quantile(DegreesOfFreedom::Finite(0.01), 0.95).unwrap();
        assert!(q01 > 1e12 && q005 > q01 && q001 > q005, "{q01}, {q005}, {q001}");
        let q01_99 = student_t_quantile(DegreesOfFreedom::Finite(0.1), 0.99).unwrap();
        assert!(q01_99 > q01 * 1e6, "99% at ν=0.1 should far exceed 95%: {q01_99} vs {q01}");
        assert!(student_t_quantile(DegreesOfFreedom::Finite(1e-6), 0.95).is_err());
        assert!(student_t_quantile(DegreesOfFreedom::Finite(f64::MIN_POSITIVE * 1e-10), 0.95).is_err());
        // 极小正自由度缩放不发生中间溢出而下溢为 0
        assert_eq!(
            combine_dof(&[(1.0, DegreesOfFreedom::Finite(1e-310))], false).unwrap(),
            DegreesOfFreedom::Finite(1e-310)
        );
    }
}

/// 按 f64 的最短十进制表示修约，避免二进制缩放把十进制的 5 误判到一侧。
pub fn rounded_decimal(value: f64, places: i32, carry: bool) -> Result<f64, String> {
    finite(&[value])?;
    if value < 0.0 {
        return Err("不确定度不能为负数".into());
    }
    if value == 0.0 {
        return Ok(0.0);
    }
    let text = value.to_string();
    let (mantissa, exp) = match text.split_once(['e', 'E']) {
        Some((a, b)) => (a, b.parse::<i32>().map_err(|_| "修约指数无效")?),
        None => (text.as_str(), 0),
    };
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let all: Vec<u8> = mantissa.bytes().filter(|c| *c != b'.').collect();
    let first = all.iter().position(|c| *c != b'0').ok_or("修约数值无效")?;
    let digits = &all[first..];
    let exponent = decimal - first as i32 - 1 + exp;
    let keep = i64::from(exponent) + i64::from(places) + 1;
    if keep >= digits.len() as i64 {
        return Ok(value);
    }
    let take = keep.max(0) as usize;
    let rest = &digits[take..];
    let increment = if carry {
        rest.iter().any(|&c| c != b'0')
    } else if keep < 0 {
        false
    } else {
        let first_discarded = rest[0];
        first_discarded > b'5'
            || (first_discarded == b'5'
                && (rest[1..].iter().any(|&c| c != b'0')
                    || (take > 0 && (digits[take - 1] - b'0') % 2 == 1)))
    };
    let mut retained = if take == 0 {
        vec![b'0']
    } else {
        digits[..take].to_vec()
    };
    if increment {
        let mut i = retained.len();
        loop {
            if i == 0 {
                retained.insert(0, b'1');
                break;
            }
            i -= 1;
            if retained[i] < b'9' {
                retained[i] += 1;
                break;
            }
            retained[i] = b'0';
        }
    }
    let number = String::from_utf8(retained).map_err(|_| "修约数值无效")?;
    let result = format!("{number}e{}", -i64::from(places))
        .parse::<f64>()
        .map_err(|_| "修约数值超出范围")?;
    finite(&[result])?;
    Ok(result)
}

pub fn round_significant(value: f64, rounding: Rounding) -> Result<f64, String> {
    finite(&[value])?;
    if value < 0.0 {
        return Err("不确定度不能为负数".into());
    }
    let Some(digits) = rounding.digits else {
        return Ok(value);
    };
    if !(1..=17).contains(&digits) {
        return Err("有效数字须为 1～17 位".into());
    }
    if value == 0.0 {
        return Ok(0.0);
    }
    let exponent = decimal_exponent(value)?;
    rounded_decimal(value, digits as i32 - 1 - exponent, rounding.carry)
}

pub fn decimal_exponent(value: f64) -> Result<i32, String> {
    finite(&[value])?;
    if value <= 0.0 {
        return Err("修约数值必须大于 0".into());
    }
    let text = value.to_string();
    let (mantissa, exp) = match text.split_once(['e', 'E']) {
        Some((a, b)) => (a, b.parse::<i32>().map_err(|_| "修约指数无效")?),
        None => (text.as_str(), 0),
    };
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let first = mantissa
        .bytes()
        .filter(|c| *c != b'.')
        .position(|c| c != b'0')
        .ok_or("修约数值无效")?;
    Ok(decimal - first as i32 - 1 + exp)
}

/// 从未修约的 u_c 和 U 分别得到指定有效数字的报告值。
pub fn rounded_combined(
    uc: f64,
    k: f64,
    digits: usize,
    carry: bool,
) -> Result<(f64, f64), String> {
    finite(&[uc, k])?;
    if uc < 0.0 || k <= 0.0 {
        return Err("合成不确定度须非负，包含因子须大于 0".into());
    }
    if !(1..=4).contains(&digits) {
        return Err("合成有效数字须为 1～4 位".into());
    }
    let expanded = uc * k;
    finite(&[expanded])?;
    let rounding = Rounding { digits: Some(digits), carry };
    Ok((
        round_significant(uc, rounding)?,
        round_significant(expanded, rounding)?,
    ))
}

/// 有效数字指定时保留必要的末尾零，计算仍使用修约后的数值。
pub fn format_rounded(value: f64, rounding: Option<Rounding>) -> String {
    let Some(digits) = rounding.and_then(|r| r.digits) else {
        return crate::calculators::format::format_number(value);
    };
    if value == 0.0 {
        return "0".into();
    }
    let Ok(exponent) = decimal_exponent(value) else {
        return "—".into();
    };
    let places = (digits as i32 - 1 - exponent).max(0) as usize;
    let mut text = value.to_string();
    if text.contains(['e', 'E']) {
        return crate::calculators::format::format_number(value);
    }
    let present = text
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    if places > present {
        if present == 0 {
            text.push('.');
        }
        text.push_str(&"0".repeat(places - present));
    }
    text
}
