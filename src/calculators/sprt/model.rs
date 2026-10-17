//! SPRT 电阻比模型；W=R/Rtp，先扣除证书偏差得到参考电阻比 Wr。
use super::curves::{B, D, reference};
use crate::calculators::{
    calibration::{inverse, least_squares, polynomial},
    numerics::finite,
};
#[derive(Clone, Copy)]
pub(crate) struct Sprt {
    pub rtp: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub a4: f64,
    pub b4: f64,
    // a4/b4 采用对数偏差；a5/b5 在子区间模式中采用二次偏差。
    pub low_quadratic: bool,
    // 高温银点项仅作用于 W>W660 的部分，低于铝点时该项为零。
    pub d: f64,
    pub w660: f64,
}
impl Default for Sprt {
    fn default() -> Self {
        Self {
            rtp: 25.12461,
            a: -0.000272072,
            b: 0.0000178924,
            c: -0.0000112269,
            a4: 0.0,
            b4: 0.0,
            low_quadratic: false,
            d: 0.0,
            w660: 3.376008599409339,
        }
    }
}
impl Sprt {
    pub fn validate(&self) -> Result<(), String> {
        finite(&[
            self.rtp, self.a, self.b, self.c, self.a4, self.b4, self.d, self.w660,
        ])?;
        if self.rtp <= 0.0 {
            return Err("水三相点电阻 Rtp 必须大于 0".into());
        }
        Ok(())
    }
    /// high 由当前启用分区决定；低温单独启用时不能仅按电阻大小切换分支。
    pub fn temperature_branch(&self, r: f64, high: bool) -> Result<f64, String> {
        self.validate()?;
        if !r.is_finite() || r <= 0.0 {
            return Err("电阻必须大于 0 Ω".into());
        }
        let w = r / self.rtp;
        let slope = if high {
            1.0 - self.a
                - 2.0 * self.b * (w - 1.0)
                - 3.0 * self.c * (w - 1.0).powi(2)
                - 2.0 * self.d * (w - self.w660).max(0.0)
        } else if self.low_quadratic {
            1.0 - self.a4 - 2.0 * self.b4 * (w - 1.0)
        } else {
            1.0 - self.a4 - self.b4 * (w.ln() + (w - 1.0) / w)
        };
        if !slope.is_finite() || slope <= 0.0 {
            return Err("偏差系数使电阻比曲线不再单调，请检查校准系数".into());
        }
        let wr = if high {
            w - self.a * (w - 1.0)
                - self.b * (w - 1.0).powi(2)
                - self.c * (w - 1.0).powi(3)
                - self.d * (w - self.w660).max(0.0).powi(2)
        } else {
            w - self.a4 * (w - 1.0)
                - self.b4
                    * if self.low_quadratic {
                        (w - 1.0).powi(2)
                    } else {
                        (w - 1.0) * w.ln()
                    }
        };
        if wr <= 0.0 {
            return Err("偏差系数使参考电阻比非正".into());
        }
        let t = if high || (self.low_quadratic && w >= 1.0) {
            polynomial(D, (wr - 2.64) / 1.64)
        } else {
            273.16 * polynomial(B, (wr.powf(1.0 / 6.0) - 0.65) / 0.35) - 273.15
        };
        finite(&[t])?;
        Ok(t)
    }
    pub fn temperature(&self, r: f64) -> Result<f64, String> {
        self.temperature_branch(r, r >= self.rtp)
    }
    pub fn resistance(&self, t: f64) -> Result<f64, String> {
        self.resistance_branch(t, t >= 0.0)
    }
    pub fn resistance_branch(&self, t: f64, high: bool) -> Result<f64, String> {
        self.validate()?;
        if !(-200.0..=961.78).contains(&t) {
            return Err("SPRT 模型换算范围为 −200～961.78 °C".into());
        }
        inverse(
            |w| self.temperature_branch(w * self.rtp, high),
            t,
            if high { 0.99 } else { 0.01 },
            if high || (self.low_quadratic && t >= 0.0) {
                4.5
            } else {
                1.01
            },
        )
        .map(|w| w * self.rtp)
    }
    /// 固定点给出温度 °C 与实测电阻 Ω；高温项数必须与选定子区间一致。
    pub fn fit(
        &mut self,
        high_points: &[(f64, f64)],
        low_points: &[(f64, f64)],
        terms: usize,
    ) -> Result<(), String> {
        self.validate()?;
        for &(t, r) in high_points.iter().chain(low_points) {
            finite(&[t, r])?;
            if r <= 0.0 {
                return Err("各校准点电阻必须大于 0".into());
            }
        }
        let mut rows = Vec::new();
        let mut y = Vec::new();
        for &(t, r) in high_points {
            let w = r / self.rtp;
            rows.push((1..=terms).map(|n| (w - 1.0).powi(n as i32)).collect());
            y.push(w - reference(t));
        }
        if !high_points.is_empty() {
            let c = least_squares(&rows, &y)?;
            self.a = c[0];
            self.b = c.get(1).copied().unwrap_or(0.0);
            self.c = c.get(2).copied().unwrap_or(0.0);
        }
        if !low_points.is_empty() {
            let rows: Vec<_> = low_points
                .iter()
                .map(|(_, r)| {
                    let w = r / self.rtp;
                    vec![
                        w - 1.0,
                        if self.low_quadratic {
                            (w - 1.0).powi(2)
                        } else {
                            (w - 1.0) * w.ln()
                        },
                    ]
                })
                .collect();
            let y: Vec<_> = low_points
                .iter()
                .map(|(t, r)| r / self.rtp - reference(*t))
                .collect();
            let c = least_squares(&rows, &y)?;
            self.a4 = c[0];
            self.b4 = c[1];
        }
        Ok(())
    }
    /// 在已由锡、锌、铝点求得 a/b/c 后，单独用银点确定 d。
    pub fn fit_silver(&mut self, t: f64, resistance: f64) -> Result<(), String> {
        let w = resistance / self.rtp;
        if w <= self.w660 {
            return Err("银点电阻比须大于铝点电阻比 W660".into());
        }
        self.d = (w
            - reference(t)
            - self.a * (w - 1.0)
            - self.b * (w - 1.0).powi(2)
            - self.c * (w - 1.0).powi(3))
            / (w - self.w660).powi(2);
        finite(&[self.d])
    }
}
