//! 共享数值约束与稳定统计，集中处理溢出、大基准上的微差和样本分母。
pub(crate) fn finite(values: &[f64]) -> Result<(), String> {
    if values.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err("计算超出双精度数值范围，请检查输入数量级".to_owned())
    }
}

pub(crate) fn span(low: f64, high: f64) -> Result<f64, String> {
    let span = high - low;
    finite(&[low, high, span])?;
    if span <= 0.0 {
        return Err("量程上限必须大于下限".to_owned());
    }
    Ok(span)
}

/// 用 hypot 累积平方和根，避免中间平方先溢出，而最终结果本可表示。
pub(crate) fn rss(values: impl IntoIterator<Item = f64>) -> Result<f64, String> {
    let result = values.into_iter().fold(0.0_f64, f64::hypot);
    finite(&[result])?;
    Ok(result)
}

// Neumaier 补偿求和，保留数值相加时丢失的低位。
fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0_f64;
    let mut correction = 0.0;
    for value in values {
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    sum + correction
}

pub(crate) struct Statistics {
    pub mean: f64,
    pub sd: f64,
    pub min: f64,
    pub max: f64,
    pub standard_error: f64,
    pub residuals: Vec<f64>,
    pub center: Option<(f64, f64)>,
}

/// sd 为以 n−1 作分母的样本标准差；standard_error 为均值的标准差 s/√n。
pub(crate) fn statistics(values: &[f64]) -> Result<Statistics, String> {
    if values.len() < 2 {
        return Err("至少需要 2 个测量值".to_owned());
    }
    finite(values)?;
    let count = values.len() as f64;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // 一般情况先中心化，保留大偏移量中的细小差异。
    // 符号相反的极端值需要缩放，避免 x - origin 溢出。
    let origin = values[0];
    let differences: Vec<_> = values.iter().map(|x| x - origin).collect();
    let (mean, residuals, center, sd) = if differences.iter().all(|x| x.is_finite()) {
        let delta_mean = compensated_sum(differences.iter().map(|x| x / count));
        let residuals = differences
            .iter()
            .map(|x| x - delta_mean)
            .collect::<Vec<_>>();
        let sd = rss(residuals.iter().map(|x| x / (count - 1.0).sqrt()))?;
        (origin + delta_mean, residuals, Some((origin, delta_mean)), sd)
    } else {
        let scale = min.abs().max(max.abs());
        let normalized_mean = compensated_sum(values.iter().map(|x| (x / scale) / count));
        let mean = normalized_mean * scale;
        let factor = (count - 1.0).sqrt();
        let sd = rss(values
            .iter()
            .map(|x| ((x / scale) - normalized_mean) / factor))?
            * scale;
        (
            mean,
            values.iter().map(|x| x - mean).collect::<Vec<_>>(),
            None,
            sd,
        )
    };
    finite(&[mean, sd, min, max])?;
    Ok(Statistics {
        mean,
        sd,
        min,
        max,
        standard_error: sd / count.sqrt(),
        residuals,
        center,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_scales_residuals_when_unscaled_difference_from_mean_would_overflow() {
        let values = [1.5e308, -1.5e308, -1.5e308];
        let stats = statistics(&values).unwrap();
        assert!(stats.center.is_none());
        assert!((stats.mean - (-0.5e308)).abs() < 1e293);
        let expected_sd = 3.0_f64.sqrt() * 1.0e308;
        assert!((stats.sd - expected_sd).abs() <= expected_sd * 1e-14);
    }
}
