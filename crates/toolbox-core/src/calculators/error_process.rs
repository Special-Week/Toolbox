//! 误差及不确定度页面共用的统计和修约说明。
use super::{
    common::{format_process_computed as c, format_process_number as n},
    numerics::Statistics,
    uncertainty::model::Rounding,
};

pub fn samples(values: &[f64], stats: &Statistics, separate_points: bool) -> Vec<String> {
    let mut lines = vec![
        format!(
            "测量值（n = {}）：{}。",
            values.len(),
            values.iter().map(|x| n(*x)).collect::<Vec<_>>().join("，")
        ),
        format!(
            "平均值 x̄（双精度）为 {}。",
            if values
                .iter()
                .zip(&stats.residuals)
                .any(|(value, residual)| ((*value - stats.mean) - residual).abs()
                    > residual.abs() * 1e-8)
            {
                n(stats.mean)
            } else {
                c(stats.mean)
            }
        ),
    ];
    if let Some((origin, delta_mean)) = stats.center {
        lines.push(format!("计算采用首个测量值 x₀ = {}，中心化均值 d̄ = Σ(xᵢ − x₀)/n = {}；残差按 (xᵢ − x₀) − d̄ 计算。", n(origin), c(delta_mean)));
    } else {
        lines.push("均值从原始测量值补偿求和，必要时先缩放。标准差用缩放后的残差计算，下列残差以原单位表示。".to_owned());
    }
    for (i, (value, residual)) in values.iter().zip(&stats.residuals).enumerate() {
        if separate_points {
            lines.push(String::new());
        }
        lines.push(if let Some((origin, delta_mean)) = stats.center {
            let difference = *value - origin;
            format!("测量点 {}：xᵢ = {}；dᵢ = xᵢ − x₀ = {} − {} = {}；残差 vᵢ = dᵢ − d̄ = {} − {} = {}。", i + 1, n(*value), n(*value), n(origin), c(difference), c(difference), c(delta_mean), c(*residual))
        } else {
            format!("测量点 {}：xᵢ = {}；残差 vᵢ = xᵢ − x̄ = {} − {} = {}。", i + 1, n(*value), n(*value), n(stats.mean), c(*residual))
        });
    }
    lines.push(String::new());
    let method = if stats.center.is_some() {
        "中心化均值、补偿求和及 hypot 累积"
    } else {
        "缩放残差、补偿求和及 hypot 累积"
    };
    lines.push(format!(
        "样本标准差 s = √[Σvᵢ²/(n − 1)] = {}；程序采用{method}，降低数值误差。",
        c(stats.sd)
    ));
    lines
}

pub fn rounding(rounding: Option<Rounding>) -> String {
    match rounding {
        Some(Rounding {
            digits: Some(digits),
            carry,
        }) => format!(
            "保留 {digits} 位有效数字，{}",
            if carry {
                "进位"
            } else {
                "四舍六入、五成双"
            }
        ),
        _ => "未设置有效数字，保留原始精度".to_owned(),
    }
}

pub fn coverage(mode: usize, k: f64) -> String {
    let probability = if mode == 1 { 95 } else { 99 };
    format!("以 t 分布近似 {probability}% 覆盖区间，双侧分位数 k = t_p(ν_eff) = {}。适用于统计独立的分量且 t 分布近似合理的情况。", c(k))
}

pub fn combined(
    lines: &mut Vec<String>,
    uc: f64,
    k: f64,
    digits: usize,
    carry: bool,
    rounded: f64,
    rounded_expanded: f64,
) {
    lines.push(String::new());
    lines.push(format!(
        "扩展不确定度 U = k × u_c = {} × {} = {}。",
        n(k),
        c(uc),
        c(k * uc)
    ));
    lines.push(format!("u_c 和 U 各自修约，{}。修约后 u_c = {}，修约后 U = {}。", rounding(Some(Rounding { digits: Some(digits), carry })), c(rounded), c(rounded_expanded)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculators::numerics::statistics;

    #[test]
    fn uncertainty_measurements_are_contiguous_but_repeatability_points_remain_separated() {
        let values = [10.02, 10.01, 10.03];
        let stats = statistics(&values).unwrap();
        for separate_points in [false, true] {
            let lines = samples(&values, &stats, separate_points);
            let first = lines
                .iter()
                .position(|line| line.starts_with("测量点 1："))
                .unwrap();
            let second = lines
                .iter()
                .position(|line| line.starts_with("测量点 2："))
                .unwrap();
            let third = lines
                .iter()
                .position(|line| line.starts_with("测量点 3："))
                .unwrap();
            let spacing = if separate_points { 2 } else { 1 };
            assert_eq!(second - first, spacing);
            assert_eq!(third - second, spacing);
            assert!(lines[third + 1].is_empty());
        }
    }

    #[test]
    fn displayed_residuals_are_the_centered_values_used_by_statistics() {
        let values = [1e16, 1e16 + 2.0, 1e16 + 2.0];
        let stats = statistics(&values).unwrap();
        let lines = samples(&values, &stats, false);
        for (line, residual) in lines
            .iter()
            .filter(|line| line.starts_with("测量点"))
            .zip(&stats.residuals)
        {
            assert!(line.contains(&format!("= {}。", c(*residual))), "{line}");
        }
        assert_ne!(values[0] - stats.mean, stats.residuals[0]);
        assert!((stats.sd - (4.0_f64 / 3.0).sqrt()).abs() < 1e-14);
    }

    #[test]
    fn extreme_values_describe_the_scaled_mean_branch() {
        let values = [1e308, -1e308, 1e308, -1e308];
        let stats = statistics(&values).unwrap();
        assert!(stats.center.is_none());
        let lines = samples(&values, &stats, false).join("\n");
        assert!(lines.contains("程序采用缩放残差、补偿求和及 hypot 累积"));
        assert!(!lines.contains("程序采用中心化均值"));
    }
}
