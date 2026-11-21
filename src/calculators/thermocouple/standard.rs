//! S/B 标准热电偶：固定点偏差叠加参考曲线，或使用限定温区的直接多项式。
#![allow(clippy::items_after_test_module)]
use super::{reference, thermocouple_emf};
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

pub(crate) enum FixedMethod {
    SLinear,
    SQuadratic([f64; 3]),
    BZero,
    BLinearFrom23,
    BConstant(usize),
    BTransition { index: usize, fraction: f64 },
}

pub(crate) struct FixedAdjustment {
    pub offsets: Vec<f64>,
    pub value: f64,
    pub method: FixedMethod,
}

fn derivative(coefficients: &[f64]) -> Vec<f64> {
    coefficients
        .iter()
        .enumerate()
        .skip(1)
        .map(|(power, coefficient)| *coefficient * power as f64)
        .collect()
}

// 导数零点划分单调区间，再在各区间内二分寻找穿零点。
fn polynomial_zeros(coefficients: &[f64], low: f64, high: f64) -> Result<Vec<f64>, String> {
    if coefficients.len() <= 1 {
        return Ok(Vec::new());
    }
    let mut bounds = vec![low];
    bounds.extend(polynomial_zeros(&derivative(coefficients), low, high)?);
    bounds.push(high);
    let mut zeros = Vec::new();
    for interval in bounds.windows(2) {
        let (mut a, mut b) = (interval[0], interval[1]);
        let (left, right) = (polynomial(coefficients, a), polynomial(coefficients, b));
        finite(&[left, right])?;
        if left == 0.0 && a > low && a < high {
            zeros.push(a);
        }
        if left != 0.0 && right != 0.0 && left.is_sign_positive() != right.is_sign_positive() {
            for _ in 0..80 {
                let middle = a + (b - a) * 0.5;
                let value = polynomial(coefficients, middle);
                finite(&[value])?;
                if value == 0.0 {
                    a = middle;
                    b = middle;
                    break;
                }
                if value.is_sign_positive() == left.is_sign_positive() {
                    a = middle;
                } else {
                    b = middle;
                }
            }
            let root = a + (b - a) * 0.5;
            if root > low && root < high && zeros.last() != Some(&root) {
                zeros.push(root);
            }
        }
    }
    Ok(zeros)
}

impl StandardCouple {
    pub(crate) fn bounds(&self) -> (f64, f64) {
        self.zone.as_ref().map(|(lo, hi, _)| (*lo, *hi)).unwrap_or(
            if self.kind == "S" {
                (-50.0, 1768.1)
            } else {
                (0.0, 1820.0)
            },
        )
    }

    fn fixed_split(&self) -> Result<f64, String> {
        if self.kind == "S" {
            Ok(400.0)
        } else {
            Ok(self.points.first().ok_or("B 型需要校准点")?.0 - 300.0)
        }
    }

    /// 返回同一曲线分段内的差分点及变化率；分段交界保留温度和电势，但不给出变化率。
    pub(crate) fn slope(&self, t: f64) -> Result<Option<(f64, f64, f64)>, String> {
        let (low, high) = self.bounds();
        let mut a = (t - 0.01).max(low);
        let mut b = (t + 0.01).min(high);
        if self.zone.is_none() {
            let reference = reference(self.kind)?;
            let mut joins = vec![self.fixed_split()?];
            joins.extend(reference.segments.iter().map(|segment| segment.max));
            if self.kind == "B" {
                joins.push(23.0);
                for &(point, _) in self.points.iter().take(4) {
                    joins.extend([point + 30.0, point + 70.0]);
                }
            }
            for join in joins.into_iter().filter(|join| *join > low && *join < high) {
                if t == join {
                    return Ok(None);
                }
                if t < join {
                    b = b.min(join.next_down());
                } else {
                    a = a.max(join.next_up());
                }
            }
        }
        if a >= b {
            return Ok(None);
        }
        let slope = (self.emf(b)? - self.emf(a)?) / (b - a);
        finite(&[slope])?;
        Ok(Some((a, b, slope)))
    }

    /// 校准值是实际热电势，不是修正量；仅修正基准多项式的一次与二次系数。
    pub fn fit_zone_coefficients(
        zone: &mut (f64, f64, Vec<f64>),
        points: &[(f64, f64)],
    ) -> Result<[f64; 2], String> {
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
        Ok([c[0], c[1]])
    }
    pub fn emf(&self, t: f64) -> Result<f64, String> {
        if let Some((lo, hi, c)) = &self.zone {
            if *lo <= 0.0 && *hi >= 0.0 && c.first().is_some_and(|value| *value != 0.0) {
                return Err("含 0 °C 的系数温区要求常数项 C0 为零".into());
            }
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
        let adjustment = self.fixed_adjustment(t)?;
        let e = thermocouple_emf(self.kind, t)? + adjustment.value;
        finite(&[e])?;
        Ok(e)
    }

    /// 展示与正算共用同一次固定点偏差和分段插值规则。
    pub(crate) fn fixed_adjustment(&self, t: f64) -> Result<FixedAdjustment, String> {
        let mut delta = Vec::new();
        for &(p, e) in &self.points {
            finite(&[p, e])?;
            delta.push(e - thermocouple_emf(self.kind, p)?);
        }
        // S 型按固定点偏差作二次插值；B 型采用各校准点邻域及过渡段。
        let (value, method) = if self.kind == "S" {
            if self.points.len() != 3 {
                return Err("S 型需要锌、铝、铜三个校准点".into());
            }
            if t < 400.0 {
                (delta[0] * t / self.points[0].0, FixedMethod::SLinear)
            } else {
                let mut value = 0.0;
                let mut terms = [0.0; 3];
                for (i, &offset) in delta.iter().enumerate().take(3) {
                    let mut term = offset;
                    for j in 0..3 {
                        if j != i {
                            term *= (t - self.points[j].0) / (self.points[i].0 - self.points[j].0);
                        }
                    }
                    terms[i] = term;
                    value += term;
                }
                (value, FixedMethod::SQuadratic(terms))
            }
        } else {
            if self.points.len() != 5 {
                return Err("B 型需要 1100～1500 °C 五个校准点".into());
            }
            let mut value = *delta.last().unwrap();
            let mut method = FixedMethod::BConstant(4);
            for (i, &(p, _)) in self.points.iter().enumerate() {
                if t <= p + 30.0 {
                    value = if i == 0 && t < p - 300.0 {
                        if t >= 23.0 {
                            method = FixedMethod::BLinearFrom23;
                            delta[0] * (t - 23.0) / (p - 23.0)
                        } else {
                            method = FixedMethod::BZero;
                            0.0
                        }
                    } else {
                        method = FixedMethod::BConstant(i);
                        delta[i]
                    };
                    break;
                }
                if t < p + 70.0 {
                    value = if i == 4 {
                        method = FixedMethod::BConstant(i);
                        delta[i]
                    } else {
                        method = FixedMethod::BTransition {
                            index: i,
                            fraction: (t - p - 30.0) / 40.0,
                        };
                        delta[i] + (delta[i + 1] - delta[i]) * (t - p - 30.0) / 40.0
                    };
                    break;
                }
            }
            (value, method)
        };
        Ok(FixedAdjustment {
            offsets: delta,
            value,
            method,
        })
    }
    /// B 型只选 ≥50 °C 的分支，避免其低温转折带来的多值反算。
    pub fn temperature(&self, e: f64) -> Result<f64, String> {
        let (mut lo, hi) = self.bounds();
        if self.kind == "B" {
            lo = lo.max(50.0);
        }
        if let Some((_, _, coefficients)) = &self.zone {
            let left = self.emf(lo)?;
            let right = self.emf(hi)?;
            let slope = derivative(coefficients);
            let extrema = polynomial_zeros(&derivative(&slope), lo, hi)?;
            for t in [lo, hi].into_iter().chain(extrema) {
                let value = polynomial(&slope, t);
                finite(&[value])?;
                if left == right || (right > left && value < 0.0) || (right < left && value > 0.0) {
                    return Err("所选系数在反算温区内不是严格单调曲线，请检查系数".into());
                }
            }
            return inverse(|t| self.emf(t), e, lo, hi);
        }
        finite(&[e])?;
        // 固定点修正可能在此跳变，产生无解区间或重叠解。
        let split = self.fixed_split()?;
        if !(lo < split && split <= hi) {
            return inverse(|t| self.emf(t), e, lo, hi);
        }
        let mut solution: Option<f64> = None;
        for (start, end) in [(lo, split.next_down()), (split, hi)] {
            let (a, b) = (self.emf(start)?, self.emf(end)?);
            let target = if (a.min(b)..=a.max(b)).contains(&e) {
                e
            } else if start == lo && (e - a).abs() <= 1e-10 {
                a
            } else if end == hi && (e - b).abs() <= 1e-10 {
                b
            } else {
                continue;
            };
            let root = if target == a {
                start
            } else if target == b {
                end
            } else {
                inverse(|t| self.emf(t), e, start, end)?
            };
            if solution.is_some_and(|previous| (previous - root).abs() > 1e-9) {
                return Err("该热电势对应多个温度，请核对校准点或使用限定温区的系数换算".into());
            }
            solution = Some(root);
        }
        solution.ok_or_else(|| "热电势超出有效范围或落在固定点分段间隙内".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(kind: &'static str, temperatures: &[f64]) -> StandardCouple {
        StandardCouple {
            kind,
            points: temperatures
                .iter()
                .enumerate()
                .map(|(i, &t)| {
                    (
                        t,
                        thermocouple_emf(kind, t).unwrap() + (i + 1) as f64 * 0.01,
                    )
                })
                .collect(),
            zone: None,
        }
    }

    #[test]
    fn slope_does_not_cross_fixed_point_joins() {
        let mut b = model("B", &[1100.0, 1200.0, 1300.0, 1400.0, 1500.0]);
        b.points[0].1 = 5.8029;
        assert!(b.slope(800.0).unwrap().is_none());
        for t in [799.995, 800.005] {
            let (a, b, slope) = b.slope(t).unwrap().unwrap();
            assert!((a < 800.0 && b < 800.0) || (a > 800.0 && b > 800.0));
            assert!((0.007..0.008).contains(&slope));
        }
    }

    #[test]
    fn manual_inversion_checks_zero_reference_and_monotonicity() {
        let mut model = StandardCouple {
            kind: "S", points: Vec::new(), zone: Some((-50.0, 1064.18, vec![0.01, 0.01])),
        };
        assert!(model.emf(-1.0).unwrap_err().contains("C0"));
        assert!(model.temperature(0.0).unwrap_err().contains("C0"));
        // The derivative is positive at both endpoints but negative near 100 °C.
        model.zone = Some((-50.0, 1064.18, vec![0.0, 29_999.0, -300.0, 1.0]));
        assert!(model.temperature(0.0).unwrap_err().contains("单调"));
        model.zone = Some((-50.0, 1064.18, vec![0.0, 30_000.0, -300.0, 1.0]));
        assert!((model.temperature(model.emf(101.0).unwrap()).unwrap() - 101.0).abs() < 1e-8);
        let (lo, hi, coefficients) = zones("S")[1].clone();
        model.zone = Some((lo, hi, coefficients));
        assert_eq!(model.emf(0.0).unwrap(), 0.0);
        assert!((model.temperature(model.emf(1400.0).unwrap()).unwrap() - 1400.0).abs() < 1e-8);
    }

    #[test]
    fn fixed_point_inverse_handles_overlaps_and_gaps() {
        let mut b = model("B", &[1100.0, 1200.0, 1300.0, 1400.0, 1500.0]);
        let reference = thermocouple_emf("B", 1100.0).unwrap();
        let left = b.emf(800.0_f64.next_down()).unwrap();
        let right = b.emf(800.0).unwrap();
        assert!(b.temperature((left + right) / 2.0).is_err());
        assert_eq!(b.temperature(right).unwrap(), 800.0);
        b.points[0].1 = reference - 0.01;
        let error = b.temperature(b.emf(800.0).unwrap()).unwrap_err();
        assert!(error.contains("多个温度"), "{error}");
        for t in [799.0, 801.0] {
            assert!((b.temperature(b.emf(t).unwrap()).unwrap() - t).abs() < 1e-8);
        }
        let mut s = model("S", &[419.527, 660.323, 1084.62]);
        s.points[0].1 = thermocouple_emf("S", 419.527).unwrap() - 0.01;
        let error = s.temperature(s.emf(400.0).unwrap()).unwrap_err();
        assert!(error.contains("多个温度"), "{error}");
    }

    #[test]
    fn fixed_point_process_tracks_the_actual_interpolation_branch() {
        let s = model("S", &[419.527, 660.323, 1084.62]);
        let low = s.fixed_adjustment(200.0).unwrap();
        assert!(matches!(low.method, FixedMethod::SLinear));
        assert_eq!(low.value, low.offsets[0] * 200.0 / s.points[0].0);
        let at_point = s.fixed_adjustment(660.323).unwrap();
        assert!(matches!(at_point.method, FixedMethod::SQuadratic(_)));
        assert!((at_point.value - at_point.offsets[1]).abs() < 1e-14);

        let b = model("B", &[1100.0, 1200.0, 1300.0, 1400.0, 1500.0]);
        assert!(matches!(
            b.fixed_adjustment(22.0).unwrap().method,
            FixedMethod::BZero
        ));
        let low = b.fixed_adjustment(500.0).unwrap();
        assert!(matches!(low.method, FixedMethod::BLinearFrom23));
        assert_eq!(low.value, low.offsets[0] * (500.0 - 23.0) / (1100.0 - 23.0));
        assert!(matches!(
            b.fixed_adjustment(800.0).unwrap().method,
            FixedMethod::BConstant(0)
        ));
        let below_switch = b.fixed_adjustment(800.0 - 1e-8).unwrap().value;
        let at_switch = b.fixed_adjustment(800.0).unwrap().value;
        assert!((at_switch - below_switch).abs() > 0.001);
        assert!(matches!(
            b.fixed_adjustment(1130.0).unwrap().method,
            FixedMethod::BConstant(0)
        ));
        let transition = b.fixed_adjustment(1150.0).unwrap();
        assert!(
            matches!(transition.method, FixedMethod::BTransition { index: 0, fraction } if fraction == 0.5)
        );
        assert_eq!(
            transition.value,
            transition.offsets[0]
                + (transition.offsets[1] - transition.offsets[0]) * (1150.0 - 1100.0 - 30.0) / 40.0
        );
        assert!(matches!(
            b.fixed_adjustment(1170.0).unwrap().method,
            FixedMethod::BConstant(1)
        ));
        assert!(matches!(
            b.fixed_adjustment(1700.0).unwrap().method,
            FixedMethod::BConstant(4)
        ));
        for (model, temperatures) in [
            (&s, &[200.0, 400.0, 660.323][..]),
            (
                &b,
                &[22.0, 500.0, 800.0, 1130.0, 1150.0, 1170.0, 1700.0][..],
            ),
        ] {
            for &t in temperatures {
                let adjustment = model.fixed_adjustment(t).unwrap();
                assert_eq!(
                    model.emf(t).unwrap(),
                    thermocouple_emf(model.kind, t).unwrap() + adjustment.value
                );
            }
        }
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
