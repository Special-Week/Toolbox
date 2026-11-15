//! 工业热电阻的分段特性曲线，系数按各分度号独立保存。
//! 返回 R/R₀ 及其对摄氏温度的解析导数，避免把不同型号混为同一组 CVD 参数。

#[derive(Clone, Copy, Debug)]
pub(crate) enum Curve {
    Cvd([f64; 3]),
    Pt3916,
    Cu428,
    Cu427,
    Ni617,
    Ni618,
    Ni672,
    Ni1000,
}

pub(crate) const PT385: [f64; 3] = [3.9083e-3, -5.775e-7, -4.183e-12];

pub(crate) struct PolynomialForm {
    coefficients: [f64; 8],
    length: usize,
    pub argument: f64,
    pub shift: f64,
    pub scale: f64,
    pub segment: &'static str,
    pub bounds: Option<(f64, f64)>,
}

impl PolynomialForm {
    fn new(coefficients: &[f64], t: f64, shift: f64, scale: f64, segment: &'static str) -> Self {
        let mut data = [0.0; 8];
        data[..coefficients.len()].copy_from_slice(coefficients);
        Self {
            coefficients: data,
            length: coefficients.len(),
            argument: if shift == 0.0 { t } else { t + shift },
            shift,
            scale,
            segment,
            bounds: None,
        }
    }

    pub(crate) fn coefficients(&self) -> &[f64] {
        &self.coefficients[..self.length]
    }

    pub(crate) fn value_and_slope(&self) -> (f64, f64) {
        let (value, slope) = polynomial(self.coefficients(), self.argument);
        (self.scale * value, self.scale * slope)
    }
}

/// 返回 R(t)/R0 及其对摄氏温度的解析导数。
/// 两者使用相同的多项式系数，通过秦九韶法求值。
fn polynomial(coefficients: &[f64], t: f64) -> (f64, f64) {
    let mut value = 0.0;
    let mut derivative = 0.0;
    for &coefficient in coefficients.iter().rev() {
        derivative = derivative * t + value;
        value = value * t + coefficient;
    }
    (value, derivative)
}

impl Curve {
    pub(crate) fn evaluate(self, t: f64) -> (f64, f64) {
        if let Self::Cvd([a, b, c]) = self {
            if t < 0.0 {
                polynomial(&[1.0, a, b, -100.0 * c, c], t)
            } else {
                polynomial(&[1.0, a, b], t)
            }
        } else {
            self.polynomial_form(t).unwrap().value_and_slope()
        }
    }

    /// 非 CVD 分段的多项式形式同时用于计算与过程展示。
    pub(crate) fn polynomial_form(self, t: f64) -> Option<PolynomialForm> {
        let form = |coefficients: &[f64], shift, scale, segment| {
            PolynomialForm::new(coefficients, t, shift, scale, segment)
        };
        Some(match self {
            Self::Cvd(_) => return None,
            Self::Pt3916 if t < 0.0 => form(
                &[
                    1.0,
                    3.971686e-3,
                    -1.157433e-6,
                    -2.051844e-8,
                    -3.629438e-10,
                    -3.157615e-12,
                    -1.369914e-14,
                    -2.303654e-17,
                ],
                0.0,
                1.0,
                "Pt3916：t < 0 °C",
            ),
            Self::Pt3916 => form(
                &[1.0, 3.974973e-3, -5.8973e-7],
                0.0,
                1.0,
                "Pt3916：t ≥ 0 °C",
            ),
            Self::Cu428 => {
                let (a, b, c) = (0.00428, -9.31e-8, 1.23e-9);
                form(
                    &[1.0, a - 100.0 * b, b - 100.0 * c, c],
                    0.0,
                    1.0,
                    "Cu428 全温区",
                )
            }
            Self::Cu427 if t < -50.0 => form(
                &[1.0, 0.0392313, -7.45044e-6],
                200.0,
                0.117058,
                "Cu427：t < −50 °C",
            ),
            Self::Cu427 if t <= 150.0 => {
                form(&[1.0, 0.0042743], 0.0, 1.0, "Cu427：−50 °C ≤ t ≤ 150 °C")
            }
            Self::Cu427 => form(
                &[1.0, 0.00262628, 2.43732e-8],
                -150.0,
                1.641145,
                "Cu427：t > 150 °C",
            ),
            Self::Ni617 if t < 100.0 => {
                form(&[1.0, 0.0054963, 6.7556e-6], 0.0, 1.0, "Ni617：t < 100 °C")
            }
            Self::Ni617 => form(
                &[1.0, 0.0054963, 6.7556e-6 - 100.0 * 9.2004e-9, 9.2004e-9],
                0.0,
                1.0,
                "Ni617：t ≥ 100 °C",
            ),
            Self::Ni618 => form(
                &[1.0, 0.005485, 6.65e-6, 0.0, 2.805e-11, 0.0, -2.0e-17],
                0.0,
                1.0,
                "Ni618 全温区",
            ),
            Self::Ni672 => {
                let index = NI672
                    .iter()
                    .position(|segment| t <= segment.1)
                    .unwrap_or(NI672.len() - 1);
                let segment = &NI672[index];
                let mut result = form(&segment.0, 0.0, 1.0, "Ni672：按当前温度选取的分段");
                result.bounds = Some((
                    if index == 0 {
                        -80.0
                    } else {
                        NI672[index - 1].1
                    },
                    segment.1,
                ));
                result
            }
            Self::Ni1000 => form(
                &[1.0, 0.004427, 5.172e-6, 5.585e-9],
                0.0,
                1.0,
                "Ni1000 全温区",
            ),
        })
    }

    /// CVD 反算要求特性曲线在整个支持范围内为正且严格递增。
    /// 通过解析求解 dR/dt 的极值检查这一条件。
    pub(crate) fn validate(self, min: f64, max: f64) -> Result<(), String> {
        if let Self::Cvd([a, b, c]) = self {
            if ![a, b, c].iter().all(|v| v.is_finite()) {
                return Err("CVD 系数必须是有限数字".to_owned());
            }
            let mut candidates = vec![min, 0.0, max];
            // d²(R/R0)/dt² = 2B - 600Ct + 12Ct² (t < 0).
            if c != 0.0 {
                let discriminant = 360_000.0 * c * c - 96.0 * b * c;
                if !discriminant.is_finite() {
                    return Err("CVD 系数过大，无法可靠计算".to_owned());
                }
                if discriminant >= 0.0 {
                    for root in [
                        (600.0 * c + discriminant.sqrt()) / (24.0 * c),
                        (600.0 * c - discriminant.sqrt()) / (24.0 * c),
                    ] {
                        if root > min && root < 0.0 {
                            candidates.push(root);
                        }
                    }
                }
            }
            if candidates.iter().any(|&t| {
                let (ratio, slope) = self.evaluate(t);
                !ratio.is_finite() || ratio <= 0.0 || !slope.is_finite() || slope <= 0.0
            }) {
                return Err(
                    "该组 CVD 系数在 −200～850 °C 内不能形成正值、单调递增的电阻曲线".to_owned(),
                );
            }
        }
        Ok(())
    }
}

// alpha = 0.00672 的 Ni120 分段三次拟合；每行为
// （[常数、t、t²、t³ 的系数]，温度上限），总下限为 -80 °C。
const NI672: [([f64; 4], f64); 12] = [
    (
        [0.9980384367, 0.005779005438, 4.519218356e-6, 1.883007648e-8],
        -60.0,
    ),
    (
        [0.9995545058, 0.005854808892, 5.782609262e-6, 2.584891485e-8],
        -30.0,
    ),
    ([1.0, 0.005899358312, 7.267589932e-6, 4.234870007e-8], 0.0),
    ([1.0, 0.005899358312, 7.267589932e-6, 1.154640832e-8], 30.0),
    (
        [1.000118847, 0.005887473643, 7.663745572e-6, 7.144678985e-9],
        60.0,
    ),
    (
        [1.002329124, 0.005776959768, 9.50564349e-6, -3.088087226e-9],
        90.0,
    ),
    (
        [0.9940315172, 0.006053466667, 6.432455728e-6, 8.294089672e-9],
        120.0,
    ),
    (
        [1.007022904, 0.005728761999, 9.138994624e-6, 7.7592607e-10],
        150.0,
    ),
    (
        [0.891859209, 0.008032035898, -6.216164699e-6, 3.489850234e-8],
        180.0,
    ),
    (
        [
            0.9060247382,
            0.007795943744,
            -4.904541625e-6,
            3.246957072e-8,
        ],
        210.0,
    ),
    (
        [1.103473241, 0.004975250849, 8.527329303e-6, 1.114941068e-8],
        240.0,
    ),
    (
        [1.437355995, 0.0008017164189, 2.59170561e-5, -1.300325764e-8],
        260.0,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayed_polynomial_uses_the_actual_piecewise_curve() {
        let low = Curve::Cu427.polynomial_form(-100.0).unwrap();
        assert_eq!(low.argument, 100.0);
        assert_eq!(low.scale, 0.117058);
        assert_eq!(low.coefficients(), &[1.0, 0.0392313, -7.45044e-6]);
        assert!(
            (low.value_and_slope().0 - 0.117058 * (1.0 + 0.0392313 * 100.0 - 7.45044e-6 * 10000.0))
                .abs()
                < 1e-14
        );
        assert_eq!(
            Curve::Cu427.polynomial_form(-50.0).unwrap().coefficients(),
            &[1.0, 0.0042743]
        );
        assert_eq!(
            Curve::Ni672.polynomial_form(-60.0).unwrap().bounds,
            Some((-80.0, -60.0))
        );
        assert_eq!(
            Curve::Ni672.polynomial_form(-59.0).unwrap().bounds,
            Some((-60.0, -30.0))
        );
        assert!(Curve::Cvd(PT385).polynomial_form(100.0).is_none());
    }

    #[test]
    fn displayed_polynomial_matches_every_curve_and_boundary() {
        for curve in [
            Curve::Pt3916,
            Curve::Cu428,
            Curve::Cu427,
            Curve::Ni617,
            Curve::Ni618,
            Curve::Ni672,
            Curve::Ni1000,
        ] {
            for t in [
                -200.0, -80.0, -60.000001, -60.0, -59.999999, -50.000001, -50.0, -49.999999, -30.0,
                -0.000001, 0.0, 0.000001, 30.0, 60.0, 90.0, 99.999999, 100.0, 100.000001, 120.0,
                150.0, 150.000001, 180.0, 210.0, 240.0, 260.0, 500.0,
            ] {
                let form = curve.polynomial_form(t).unwrap();
                let (ratio, slope) = form.value_and_slope();
                assert_eq!((ratio, slope), curve.evaluate(t));
                let mut polynomial = 0.0;
                for coefficient in form.coefficients().iter().rev() {
                    polynomial = polynomial * form.argument + coefficient;
                }
                assert_eq!(ratio, form.scale * polynomial, "{curve:?}, {t}");
            }
        }
    }
}
