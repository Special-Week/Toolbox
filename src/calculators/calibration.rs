//! 校准传感器共用数值工具：升幂多项式、区间反算、缩放 QR 拟合及温标转换。
use super::numerics::finite;

/// 系数按常数项、一次项……排列，使用 Horner 法减少乘法及舍入次数。
pub(crate) fn polynomial(c: &[f64], x: f64) -> f64 {
    c.iter().rev().fold(0.0, |y, c| y * x + c)
}

/// 反算连续单调函数，调用者负责选择正确分支并给出有效边界。
/// 末尾检查正算残差，防止把分段曲线的跳变误当作已经收敛的根。
pub(crate) fn inverse(
    f: impl Fn(f64) -> Result<f64, String>,
    target: f64,
    mut low: f64,
    mut high: f64,
) -> Result<f64, String> {
    finite(&[target, low, high])?;
    let a = f(low)?;
    let b = f(high)?;
    finite(&[a, b])?;
    if target < a.min(b) - 1e-10 || target > a.max(b) + 1e-10 {
        return Err("输入超出所选曲线的有效范围".into());
    }
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        let value = f(mid)?;
        finite(&[value])?;
        if (value < target) == (a < b) {
            low = mid;
        } else {
            high = mid;
        }
    }
    let root = (low + high) / 2.0;
    if (f(root)? - target).abs() > 1e-9 * target.abs().max(1.0) {
        return Err("所选系数曲线在此输入处没有连续的反算解".into());
    }
    Ok(root)
}

/// 列缩放后两次重正交化 QR；避免正规方程放大高阶校准矩阵的条件数。
pub(crate) fn least_squares(rows: &[Vec<f64>], y: &[f64]) -> Result<Vec<f64>, String> {
    let n = rows.first().map_or(0, Vec::len);
    let m = rows.len();
    if n == 0 || m < n || y.len() != m || rows.iter().any(|r| r.len() != n) {
        return Err("校准点数量不足以确定所选系数".into());
    }
    finite(y)?;
    for row in rows {
        finite(row)?;
    }
    let mut q: Vec<Vec<f64>> = Vec::new();
    let mut r = vec![vec![0.0; n]; n];
    let mut scale = vec![0.0; n];
    for j in 0..n {
        scale[j] = rows.iter().map(|row| row[j].abs()).fold(0.0, f64::max);
        if scale[j] == 0.0 {
            return Err("校准点不能独立确定系数".into());
        }
        let mut v: Vec<_> = rows.iter().map(|row| row[j] / scale[j]).collect();
        for _ in 0..2 {
            for i in 0..j {
                let dot: f64 = q[i].iter().zip(&v).map(|(a, b)| a * b).sum();
                r[i][j] += dot;
                for k in 0..m {
                    v[k] -= dot * q[i][k];
                }
            }
        }
        let norm = v.iter().fold(0.0_f64, |a, b| a.hypot(*b));
        if norm < 1e-12 {
            return Err("校准点重合或分布过于集中，无法可靠拟合".into());
        }
        r[j][j] = norm;
        q.push(v.iter().map(|v| v / norm).collect());
    }
    let mut x: Vec<f64> = q
        .iter()
        .map(|col| col.iter().zip(y).map(|(a, b)| a * b).sum())
        .collect();
    for j in (0..n).rev() {
        for k in j + 1..n {
            x[j] -= r[j][k] * x[k];
        }
        x[j] /= r[j][j];
    }
    for j in 0..n {
        x[j] /= scale[j];
    }
    finite(&x)?;
    Ok(x)
}

#[derive(Clone, Copy)]
/// UI 索引依次为 °C、K、°F；构造值来自对应下拉框。
pub(crate) struct TemperatureUnit(pub usize);
impl TemperatureUnit {
    pub fn symbol(self) -> &'static str {
        ["°C", "K", "°F"][self.0]
    }
    pub fn celsius(self, t: f64) -> f64 {
        match self.0 {
            1 => t - 273.15,
            2 => (t - 32.0) / 1.8,
            _ => t,
        }
    }
    pub fn display(self, t: f64) -> f64 {
        match self.0 {
            1 => t + 273.15,
            2 => t * 1.8 + 32.0,
            _ => t,
        }
    }
    /// 温差与导数只使用比例，不使用绝对温度的零点偏移。
    pub fn span(self) -> f64 {
        if self.0 == 2 { 1.8 } else { 1.0 }
    }
}
