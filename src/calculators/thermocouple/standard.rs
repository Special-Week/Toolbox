//! S/B 标准热电偶：固定点偏差叠加参考曲线，或使用限定温区的直接多项式。
use super::thermocouple_emf;
use crate::calculators::{
    calibration::{inverse, polynomial},
    numerics::finite,
};
#[derive(Clone)]
pub(crate) struct StandardCouple {
    pub kind: &'static str,
    pub points: Vec<(f64, f64)>,
    pub zone: Option<(f64, f64, Vec<f64>)>,
}
impl StandardCouple {
    /// 校准值是实际热电势，不是修正量；仅修正基准多项式的一次与二次系数。
    pub fn fit_zone_coefficients(
        zone: &mut (f64, f64, Vec<f64>),
        points: &[(f64, f64)],
    ) -> Result<(), String> {
        let mut rows = Vec::new();
        let mut rhs = Vec::new();
        for &(t, e) in points {
            finite(&[t, e])?;
            if !(zone.0..=zone.1).contains(&t) {
                return Err("校准点温度超出所选温区".into());
            }
            rows.push(vec![t, t * t]);
            rhs.push(e - polynomial(&zone.2, t));
        }
        let c = crate::calculators::calibration::least_squares(&rows, &rhs)?;
        zone.2[1] += c[0];
        zone.2[2] += c[1];
        Ok(())
    }
    pub fn emf(&self, t: f64) -> Result<f64, String> {
        if let Some((lo, hi, c)) = &self.zone {
            if t == 0.0 {
                return Ok(0.0);
            }
            if !(*lo..=*hi).contains(&t) {
                return Err("温度超出所选系数温区".into());
            }
            let e = polynomial(c, t);
            finite(&[e])?;
            return Ok(e);
        }
        let mut delta = Vec::new();
        for &(p, e) in &self.points {
            finite(&[p, e])?;
            delta.push(e - thermocouple_emf(self.kind, p)?);
        }
        // S 型按固定点偏差作二次插值；B 型采用各校准点邻域及过渡段。
        let adjustment = if self.kind == "S" {
            if self.points.len() != 3 {
                return Err("S 型需要锌、铝、铜三个校准点".into());
            }
            if t < 400.0 {
                delta[0] * t / self.points[0].0
            } else {
                let mut value = 0.0;
                for (i, &offset) in delta.iter().enumerate().take(3) {
                    let mut term = offset;
                    for j in 0..3 {
                        if j != i {
                            term *= (t - self.points[j].0) / (self.points[i].0 - self.points[j].0);
                        }
                    }
                    value += term;
                }
                value
            }
        } else {
            if self.points.len() != 5 {
                return Err("B 型需要 1100～1500 °C 五个校准点".into());
            }
            let mut value = *delta.last().unwrap();
            for (i, &(p, _)) in self.points.iter().enumerate() {
                if t <= p + 30.0 {
                    value = if i == 0 && t < p - 300.0 {
                        if t >= 23.0 {
                            delta[0] * (t - 23.0) / (p - 23.0)
                        } else {
                            0.0
                        }
                    } else {
                        delta[i]
                    };
                    break;
                }
                if t < p + 70.0 {
                    value = if i == 4 {
                        delta[i]
                    } else {
                        delta[i] + (delta[i + 1] - delta[i]) * (t - p - 30.0) / 40.0
                    };
                    break;
                }
            }
            value
        };
        let e = thermocouple_emf(self.kind, t)? + adjustment;
        finite(&[e])?;
        Ok(e)
    }
    /// B 型只选 ≥50 °C 的分支，避免其低温转折带来的多值反算。
    pub fn temperature(&self, e: f64) -> Result<f64, String> {
        let (mut lo, hi) =
            self.zone
                .as_ref()
                .map(|(l, h, _)| (*l, *h))
                .unwrap_or(if self.kind == "S" {
                    (-50.0, 1768.1)
                } else {
                    (50.0, 1820.0)
                });
        if self.kind == "B" {
            lo = lo.max(50.0);
        }
        inverse(|t| self.emf(t), e, lo, hi)
    }
}

pub(crate) fn zones(kind: &str) -> Vec<(f64, f64, Vec<f64>)> {
    if kind == "S" {
        vec![
            (
                -50.,
                1064.18,
                vec![
                    0.,
                    0.00540313308631,
                    0.000012593428974,
                    -2.32477968689e-8,
                    3.22028823036e-11,
                    -3.31465196389e-14,
                    2.55744251786e-17,
                    -1.25068871393e-20,
                    2.71443176145e-24,
                ],
            ),
            (
                1064.18,
                1664.5,
                vec![
                    1.32900444085,
                    0.00334509311344,
                    0.00000654805192818,
                    -1.64856259209e-9,
                    1.29989605174e-14,
                ],
            ),
            (
                1664.5,
                1768.1,
                vec![
                    146.628232636,
                    -0.258430516752,
                    0.000163693574641,
                    -3.30439046987e-8,
                    -9.43223690612e-15,
                ],
            ),
        ]
    } else {
        vec![
            (
                0.,
                630.615,
                vec![
                    0.,
                    -0.00024650818346,
                    0.0000059040421171,
                    -1.3257931636e-9,
                    1.5668291901e-12,
                    -1.694452924e-15,
                    6.2990347094e-19,
                ],
            ),
            (
                630.615,
                1820.,
                vec![
                    -3.8938168621,
                    0.02857174747,
                    -0.000084885104785,
                    1.5785280164e-7,
                    -1.6835344864e-10,
                    1.1109794013e-13,
                    -4.4515431033e-17,
                    9.8975640821e-21,
                    -9.3791330289e-25,
                ],
            ),
        ]
    }
}
