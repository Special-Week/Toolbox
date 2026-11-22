//! 通用校准曲线的等权最小二乘拟合；归一化求解，原坐标系输出系数。
use super::super::calibration::least_squares;
use super::super::numerics::{finite, rss};

pub const MAX_POINTS: usize = 200;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FitKind {
    Linear,
    ThroughOrigin,
    Quadratic,
    Cubic,
}

impl FitKind {
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::ThroughOrigin,
            2 => Self::Quadratic,
            3 => Self::Cubic,
            _ => Self::Linear,
        }
    }

    pub fn degree(self) -> usize {
        match self {
            Self::Quadratic => 2,
            Self::Cubic => 3,
            _ => 1,
        }
    }

    pub fn parameter_count(self) -> usize {
        if self == Self::ThroughOrigin {
            1
        } else {
            self.degree() + 1
        }
    }
}

/// 系数按 K0、K1……升幂保存；预测始终使用归一化模型，避免展开式抵消。
pub struct CurveFit {
    pub kind: FitKind,
    pub coefficients: Vec<f64>,
    pub fitted: Vec<f64>,
    pub residuals: Vec<f64>,
    pub correlation: Option<f64>,
    pub r_squared: Option<f64>,
    pub rmse: f64,
    pub residual_sd: Option<f64>,
    pub maximum_residual: f64,
    // Plotting interval. A single distinct input uses the origin as the second endpoint.
    pub minimum_x: f64,
    pub maximum_x: f64,
    x: Normalization,
    y: Normalization,
    normalized_coefficients: Vec<f64>,
}

#[derive(Clone, Copy)]
struct Normalization {
    origin: f64,
    scale: f64,
}

impl Normalization {
    fn for_values(values: &[f64], centered: bool) -> Self {
        let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // 分别除以 2，防止同号大数相加或异号极值相减溢出。
        let origin = if centered {
            if minimum == maximum {
                minimum
            } else {
                minimum / 2.0 + maximum / 2.0
            }
        } else {
            0.0
        };
        let scale = values
            .iter()
            .map(|value| (value - origin).abs())
            .fold(0.0, f64::max);
        Self {
            origin,
            scale: if scale == 0.0 { 1.0 } else { scale },
        }
    }

    fn normalize(self, value: f64) -> f64 {
        (value - self.origin) / self.scale
    }
}

// 补偿求和用于均值和协方差，减少正负项抵消造成的末位误差。
fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut total = 0.0_f64;
    let mut correction = 0.0;
    for value in values {
        let next = total + value;
        correction += if total.abs() >= value.abs() {
            (total - next) + value
        } else {
            (value - next) + total
        };
        total = next;
    }
    total + correction
}

fn evaluate(coefficients: &[f64], x: f64) -> f64 {
    coefficients
        .iter()
        .rev()
        .fold(0.0_f64, |value, coefficient| value.mul_add(x, *coefficient))
}

/// 将 ys·P((x−x0)/xs)+y0 展开到原 x 单位，不对输出系数作数值修约。
fn original_coefficients(
    coefficients: &[f64],
    x: Normalization,
    y: Normalization,
) -> Result<Vec<f64>, String> {
    let mut expanded = vec![coefficients[coefficients.len() - 1] * y.scale];
    if coefficients[coefficients.len() - 1] != 0.0 && expanded[0] == 0.0 {
        return Err("系数换算发生数值下溢，请调整输入量或响应量的单位".to_owned());
    }
    for &coefficient in coefficients[..coefficients.len() - 1].iter().rev() {
        let mut next = vec![0.0; expanded.len() + 1];
        for (index, value) in expanded.iter().enumerate() {
            let scaled = value / x.scale;
            if *value != 0.0 && scaled == 0.0 {
                return Err("展开系数过小，无法用双精度数值表示，请调整输入量单位".to_owned());
            }
            next[index] = (-x.origin).mul_add(scaled, next[index]);
            next[index + 1] += scaled;
        }
        next[0] += coefficient * y.scale;
        finite(&next).map_err(|_| "展开系数超出数值范围，请调整输入量的单位或基准".to_owned())?;
        expanded = next;
    }
    expanded[0] += y.origin;
    finite(&expanded)?;
    Ok(expanded)
}

impl CurveFit {
    pub fn prediction_formula(&self) -> String {
        let coefficients = self
            .normalized_coefficients
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "X = (x − ({})) / {}\nŷ = ({}) + {} × P(X)\nP 的升幂系数 = [{coefficients}]\n使用秦九韶法求 P(X)，有条件时使用融合乘加。",
            self.x.origin, self.x.scale, self.y.origin, self.y.scale
        )
    }

    pub fn process(&self, points: &[(f64, f64)]) -> Vec<String> {
        use crate::calculators::common::{
            format_process_computed as c, format_process_number as n,
        };
        let mut lines = vec![
            format!(
                "等权最小二乘：共有 {} 个点，模型次数 {}，自由参数 {}；最小化 Σ(yᵢ−ŷᵢ)²。{}",
                points.len(),
                self.kind.degree(),
                self.kind.parameter_count(),
                if self.kind == FitKind::ThroughOrigin {
                    "过原点模型的常数项固定为 0。"
                } else {
                    "模型包含常数项。"
                }
            ),
            format!(
                "数值归一化：X = (x − x₀)/sₓ，Y = (y − y₀)/sᵧ；x₀ = {}，sₓ = {}，y₀ = {}，sᵧ = {}。",
                n(self.x.origin),
                n(self.x.scale),
                n(self.y.origin),
                n(self.y.scale)
            ),
            "对归一化后的 X 构造幂次矩阵，采用列缩放和 QR 最小二乘求解；过原点时仅使用 X 列。"
                .to_owned(),
        ];
        for (power, coefficient) in self.normalized_coefficients.iter().enumerate() {
            lines.push(format!("归一化系数 a[{power}] = {}。", n(*coefficient)));
        }
        lines.push("预测时按秦九韶法计算 Ŷ = ΣaⱼXʲ，再恢复 ŷ = y₀ + sᵧŶ；显示的原坐标系数由归一化多项式展开。".to_owned());
        for (power, coefficient) in self.coefficients.iter().enumerate() {
            lines.push(format!("原坐标系数 K{power} = {}。", n(*coefficient)));
        }
        for (i, &(x, y)) in points.iter().enumerate() {
            let nx = self.x.normalize(x);
            let ny = self.y.normalize(y);
            let predicted = evaluate(&self.normalized_coefficients, nx);
            lines.push(String::new());
            lines.push(format!(
                "第 {} 点：x = {}，y = {}；X = {}，Y = {}，Ŷ = {}，恢复拟合值 ŷ = {}。",
                i + 1,
                n(x),
                n(y),
                c(nx),
                c(ny),
                c(predicted),
                c(self.fitted[i])
            ));
            lines.push(format!(
                "残差 e = (Y − Ŷ) × sᵧ = ({} − {}) × {} = {}；在归一化坐标中相减以减少抵消。",
                c(ny),
                c(predicted),
                n(self.y.scale),
                c(self.residuals[i])
            ));
        }
        lines.push(String::new());
        lines.push(format!("RMSE = √(Σeᵢ²/n) = {}；最大绝对残差 max|eᵢ| = {}。程序使用 hypot 累积避免中间平方溢出。", c(self.rmse), c(self.maximum_residual)));
        let freedom = points.len() - self.kind.parameter_count();
        lines.push(match self.residual_sd {
            Some(sd) => format!(
                "剩余自由度 n−p = {}；残差标准差 = √[Σeᵢ²/(n−p)] = {}。",
                freedom,
                c(sd)
            ),
            None => "剩余自由度 n−p = 0，残差标准差未定义。".to_owned(),
        });
        lines.push(match self.correlation {
            Some(r) => format!("Pearson r = Σ[(xᵢ−x̄)(yᵢ−ȳ)]/[√Σ(xᵢ−x̄)² · √Σ(yᵢ−ȳ)²] = {}。r 使用原始 x、y，与模型阶数无关。", c(r)),
            None => "输入量或响应量无变化，Pearson r 未定义。".to_owned(),
        });
        lines.push(match self.r_squared {
            Some(r2) => format!("中心化 R² = 1 − Σeᵢ²/Σ(yᵢ−ȳ)² = {}。过原点时 R² 可为负。", c(r2)),
            None => "响应量无变化，中心化 R² 未定义。".to_owned(),
        });
        lines
    }

    pub fn calculate(points: &[(f64, f64)], kind: FitKind) -> Result<Self, String> {
        if !(2..=MAX_POINTS).contains(&points.len()) {
            return Err(format!("校准数据须为 2～{MAX_POINTS} 组"));
        }
        if points.len() < kind.parameter_count() {
            return Err(format!(
                "所选模型至少需要 {} 组数据，并具有足够的不同 x 值",
                kind.parameter_count()
            ));
        }
        let inputs: Vec<_> = points.iter().map(|point| point.0).collect();
        let responses: Vec<_> = points.iter().map(|point| point.1).collect();
        finite(&inputs)?;
        finite(&responses)?;
        let minimum_x = inputs.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum_x = inputs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if minimum_x == maximum_x && kind != FitKind::ThroughOrigin {
            return Err("所有输入量 x 相同，无法建立有效的校准曲线".to_owned());
        }

        let centered = kind != FitKind::ThroughOrigin;
        let x = Normalization::for_values(&inputs, centered);
        let y = Normalization::for_values(&responses, centered);
        let normalized_x: Vec<_> = inputs.iter().map(|value| x.normalize(*value)).collect();
        let normalized_y: Vec<_> = responses.iter().map(|value| y.normalize(*value)).collect();
        let rows: Vec<_> = normalized_x
            .iter()
            .map(|&value| {
                if kind == FitKind::ThroughOrigin {
                    vec![value]
                } else {
                    (0..=kind.degree())
                        .map(|power| value.powi(power as i32))
                        .collect()
                }
            })
            .collect();
        let mut normalized_coefficients = least_squares(&rows, &normalized_y)?;
        if kind == FitKind::ThroughOrigin {
            normalized_coefficients.insert(0, 0.0);
        }
        let coefficients = original_coefficients(&normalized_coefficients, x, y)?;

        let mut fitted = Vec::with_capacity(points.len());
        let mut residuals = Vec::with_capacity(points.len());
        for (&input, &response) in normalized_x.iter().zip(&normalized_y) {
            let predicted = evaluate(&normalized_coefficients, input);
            fitted.push(predicted.mul_add(y.scale, y.origin));
            // 在响应量的归一化坐标内相减，再恢复量纲，保留大基准上的小残差。
            residuals.push((response - predicted) * y.scale);
        }
        finite(&fitted)?;
        finite(&residuals)?;

        // Pearson r 始终由原始 x、y 的中心化值计算，与拟合阶数和是否过原点无关。
        let centered_x = Normalization::for_values(&inputs, true);
        let centered_y = Normalization::for_values(&responses, true);
        let count = points.len() as f64;
        let sx: Vec<_> = inputs
            .iter()
            .map(|value| centered_x.normalize(*value))
            .collect();
        let sy: Vec<_> = responses
            .iter()
            .map(|value| centered_y.normalize(*value))
            .collect();
        let mean_x = sum(sx.iter().map(|value| value / count));
        let mean_y = sum(sy.iter().map(|value| value / count));
        let x_norm = rss(sx.iter().map(|value| value - mean_x))?;
        let y_norm = rss(sy.iter().map(|value| value - mean_y))?;
        let correlation = if x_norm == 0.0 || y_norm == 0.0 {
            None
        } else {
            let r = sum(sx
                .iter()
                .zip(&sy)
                .map(|(a, b)| ((a - mean_x) / x_norm) * ((b - mean_y) / y_norm)));
            finite(&[r])?;
            Some(r.clamp(-1.0, 1.0))
        };
        let r_squared = if y_norm == 0.0 {
            None
        } else {
            let residual_norm = rss(residuals.iter().map(|value| value / centered_y.scale))?;
            let ratio = residual_norm / y_norm;
            let r_squared = 1.0 - ratio * ratio;
            finite(&[r_squared])?;
            // 中心化 R² 在强制过原点时允许为负，不以 r² 冒充拟合优度。
            Some(r_squared)
        };
        let rmse = rss(residuals.iter().map(|value| value / count.sqrt()))?;
        let freedom = points.len() - kind.parameter_count();
        let residual_sd = if freedom > 0 {
            Some(rss(residuals
                .iter()
                .map(|value| value / (freedom as f64).sqrt()))?)
        } else {
            None
        };
        let maximum_residual = residuals
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max);
        let (minimum_x, maximum_x) = if minimum_x == maximum_x {
            (minimum_x.min(0.0), maximum_x.max(0.0))
        } else {
            (minimum_x, maximum_x)
        };
        Ok(Self {
            kind,
            coefficients,
            fitted,
            residuals,
            correlation,
            r_squared,
            rmse,
            residual_sd,
            maximum_residual,
            minimum_x,
            maximum_x,
            x,
            y,
            normalized_coefficients,
        })
    }

    pub fn predict(&self, input: f64) -> Result<f64, String> {
        finite(&[input])?;
        let value = evaluate(&self.normalized_coefficients, self.x.normalize(input))
            .mul_add(self.y.scale, self.y.origin);
        finite(&[value])?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_fit_accepts_replicates_at_a_nonzero_input() {
        let points = [(2.0, 4.0), (2.0, 6.0)];
        let fit = CurveFit::calculate(&points, FitKind::ThroughOrigin).unwrap();
        assert!((fit.coefficients[1] - 2.5).abs() < 1e-12);
        assert_eq!(fit.correlation, None);
        assert!(fit.r_squared.unwrap().abs() < 1e-12);
        assert_eq!((fit.minimum_x, fit.maximum_x), (0.0, 2.0));
        assert!(CurveFit::calculate(&points, FitKind::Linear).is_err());
        assert!(CurveFit::calculate(&[(0.0, 4.0), (0.0, 6.0)], FitKind::ThroughOrigin).is_err());
    }
}
