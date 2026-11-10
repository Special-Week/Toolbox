//! 不确定度分量树：先求各分量标准不确定度，再应用灵敏系数并逐级合成。
use crate::calculators::{
    numerics::{finite, rss, statistics},
    range_method::coefficient,
};

pub(crate) enum Kind {
    TypeA {
        values: Vec<f64>,
        range_method: bool,
        average_count: Option<usize>,
    },
    TypeB {
        half_width: f64,
        distribution: usize,
    },
    Combined {
        children: Vec<Component>,
        maximum: bool,
    },
}
/// 留空时使用一次测量；取测量次数必须是正整数。
pub(crate) fn average_count(text: &str) -> Result<Option<usize>, String> {
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
pub(crate) struct Component {
    pub name: String,
    pub sensitivity: f64,
    pub kind: Kind,
    pub rounding: Option<Rounding>,
}
#[derive(Clone, Copy)]
pub(crate) struct Rounding {
    pub digits: Option<usize>,
    pub carry: bool,
}
pub(crate) fn significant_digits(text: &str) -> Result<Option<usize>, String> {
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
pub(crate) struct Detail {
    pub name: String,
    pub raw_standard: f64,
    pub standard: f64,
    pub sensitivity: f64,
    pub contribution: f64,
    pub children: Vec<Detail>,
    pub rounding: Option<Rounding>,
}
/// UI 分布索引依次为均匀、三角、正态 k=2、正态 k=3、两点和反正弦。
/// 输入是区间半宽 a，标准不确定度为 a/除数。
pub(crate) fn divisor(distribution: usize) -> f64 {
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
pub(crate) fn combine(values: &[f64], maximum: bool) -> Result<f64, String> {
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
        let standard = match &self.kind {
            Kind::TypeA {
                values,
                range_method,
                average_count,
            } => {
                let stats = statistics(values)?;
                if *range_method {
                    (stats.max - stats.min)
                        / coefficient(values.len())?
                        / (values.len() as f64).sqrt()
                } else {
                    match average_count {
                        None => stats.sd,
                        Some(0) => return Err("取测量次数 n 须为正整数".into()),
                        Some(count) => stats.sd / (*count as f64).sqrt(),
                    }
                }
            }
            Kind::TypeB {
                half_width,
                distribution,
            } => {
                finite(&[*half_width])?;
                if *half_width < 0.0 {
                    return Err("B 类半宽不能为负数".into());
                }
                half_width / divisor(*distribution)
            }
            Kind::Combined {
                children: items,
                maximum,
            } => {
                for c in items {
                    children.push(c.calculate()?);
                }
                combine(
                    &children.iter().map(|c| c.contribution).collect::<Vec<_>>(),
                    *maximum,
                )?
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
        assert!(round_significant(
            f64::MAX,
            Rounding {
                digits: Some(2),
                carry: true
            }
        )
        .is_err());
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
            (13000.0, 26000.0)
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
}

/// 按 f64 的最短十进制表示修约，避免二进制缩放把十进制的 5 误判到一侧。
pub(crate) fn rounded_decimal(value: f64, places: i32, carry: bool) -> Result<f64, String> {
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

pub(crate) fn round_significant(value: f64, rounding: Rounding) -> Result<f64, String> {
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

pub(crate) fn decimal_exponent(value: f64) -> Result<i32, String> {
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

/// 合成标准不确定度和扩展不确定度按较大者确定修约位数。
pub(crate) fn rounded_combined(
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
    let reference = uc.max(expanded);
    let places = if reference == 0.0 {
        digits as i32 - 1
    } else {
        digits as i32 - decimal_exponent(reference)? - 1
    };
    let rounded = rounded_decimal(uc, places, carry)?;
    let rounded_expanded = rounded * k;
    finite(&[rounded_expanded])?;
    Ok((rounded, rounded_expanded))
}

/// 有效数字指定时保留必要的末尾零，计算仍使用修约后的数值。
pub(crate) fn format_rounded(value: f64, rounding: Option<Rounding>) -> String {
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
