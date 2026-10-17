//! 不确定度分量树：先求各分量标准不确定度，再应用灵敏系数并逐级合成。
use crate::calculators::{
    numerics::{finite, rss, statistics},
    range_method::coefficient,
};

pub(crate) enum Kind {
    TypeA {
        values: Vec<f64>,
        range_method: bool,
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
pub(crate) struct Component {
    pub name: String,
    pub sensitivity: f64,
    pub kind: Kind,
}
pub(crate) struct Detail {
    pub name: String,
    pub standard: f64,
    pub sensitivity: f64,
    pub contribution: f64,
    pub children: Vec<Detail>,
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
            } => {
                let stats = statistics(values)?;
                if *range_method {
                    (stats.max - stats.min)
                        / coefficient(values.len())?
                        / (values.len() as f64).sqrt()
                } else {
                    stats.standard_error
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
        let contribution = standard * self.sensitivity;
        finite(&[standard, contribution])?;
        Ok(Detail {
            name: self.name.clone(),
            standard,
            sensitivity: self.sensitivity,
            contribution,
            children,
        })
    }
}
/// 非负不确定度的十进制修约；缩放无法表示时保留原值，避免制造无穷大。
pub(crate) fn rounded(value: f64, places: i32, carry: bool) -> f64 {
    let factor = 10.0_f64.powi(places);
    if !factor.is_finite() || !(value * factor).is_finite() {
        return value;
    }
    if carry {
        (value * factor).ceil() / factor
    } else {
        (value * factor).round() / factor
    }
}
