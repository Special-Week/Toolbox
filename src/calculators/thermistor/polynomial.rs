//! 热敏电阻双向校准多项式；A、B 独立拟合，不保证互为精确解析反函数。
use crate::calculators::{
    calibration::{least_squares, polynomial},
    numerics::finite,
};
pub(crate) struct PolynomialThermistor {
    /// 1/T=ΣA[i]·ln(R)^i；T 用 K、R 用 Ω，系数按升幂排列。
    pub a: Vec<f64>,
    /// ln(R)=ΣB[i]·(1/T)^i；与 A 使用相同温度、电阻单位。
    pub b: Vec<f64>,
}
impl PolynomialThermistor {
    pub fn temperature(&self, r: f64) -> Result<f64, String> {
        if !r.is_finite() || r <= 0.0 {
            return Err("电阻必须大于 0 Ω".into());
        }
        let inv = polynomial(&self.a, r.ln());
        if inv <= 0.0 {
            return Err("系数及电阻对应的绝对温度必须大于 0 K".into());
        }
        let t = 1.0 / inv - 273.15;
        finite(&[t])?;
        Ok(t)
    }
    pub fn resistance(&self, t: f64) -> Result<f64, String> {
        if !t.is_finite() || t <= -273.15 {
            return Err("温度必须高于绝对零度".into());
        }
        let r = polynomial(&self.b, 1.0 / (t + 273.15)).exp();
        finite(&[r])?;
        if r <= 0.0 {
            return Err("计算电阻低于可表示范围".into());
        }
        Ok(r)
    }
    /// 校准点输入为 (°C, Ω)；Steinhart–Hart 形式将二次项固定为零，不作为未知量。
    pub fn fit(points: &[(f64, f64)], order: usize, sh: bool) -> Result<Self, String> {
        if !(3..=6).contains(&order) {
            return Err("多项式阶数须为 3～6".into());
        }
        for &(t, r) in points {
            finite(&[t, r])?;
            if t <= -273.15 || r <= 0.0 {
                return Err("校准温度须高于绝对零度，电阻须大于 0".into());
            }
        }
        let powers: Vec<_> = (0..=order).filter(|i| !sh || *i != 2).collect();
        let features = |x: f64| powers.iter().map(|p| x.powi(*p as i32)).collect::<Vec<_>>();
        let ra: Vec<_> = points.iter().map(|(_, r)| features(r.ln())).collect();
        let rb: Vec<_> = points
            .iter()
            .map(|(t, _)| features(1.0 / (t + 273.15)))
            .collect();
        let ya: Vec<_> = points.iter().map(|(t, _)| 1.0 / (t + 273.15)).collect();
        let yb: Vec<_> = points.iter().map(|(_, r)| r.ln()).collect();
        let expand = |c: Vec<f64>| {
            let mut out = vec![0.0; order + 1];
            for (p, v) in powers.iter().zip(c) {
                out[*p] = v;
            }
            out
        };
        Ok(Self {
            a: expand(least_squares(&ra, &ya)?),
            b: expand(least_squares(&rb, &yb)?),
        })
    }
}
