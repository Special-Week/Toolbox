//! 极差估计的两位小数系数表，适用于 2～9 次独立测量。
pub(crate) const COEFFICIENTS: [f64; 8] = [1.13, 1.69, 2.06, 2.33, 2.53, 2.70, 2.85, 2.97];
/// 返回 s≈R/C 中的 C；此处保留表值，不混入其他精度或口径的 d₂ 系数。
pub(crate) fn coefficient(n: usize) -> Result<f64, String> {
    n.checked_sub(2)
        .and_then(|i| COEFFICIENTS.get(i))
        .copied()
        .ok_or_else(|| "极差法支持 2～9 次测量".into())
}
