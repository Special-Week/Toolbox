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

/// Returns R(t)/R0 and its analytic derivative with respect to degrees Celsius.
/// Horner evaluation calculates both using the same polynomial coefficients.
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
        match self {
            Self::Cvd([a, b, c]) => {
                if t < 0.0 {
                    polynomial(&[1.0, a, b, -100.0 * c, c], t)
                } else {
                    polynomial(&[1.0, a, b], t)
                }
            }
            Self::Pt3916 if t < 0.0 => polynomial(
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
                t,
            ),
            Self::Pt3916 => Self::Cvd([3.974973e-3, -5.8973e-7, -4.146e-12]).evaluate(t),
            Self::Cu428 => {
                // R/R0 = 1 + a*t + b*t*(t-100) + c*t²*(t-100).
                let (a, b, c) = (0.00428, -9.31e-8, 1.23e-9);
                polynomial(&[1.0, a - 100.0 * b, b - 100.0 * c, c], t)
            }
            Self::Cu427 if t < -50.0 => {
                let (value, slope) = polynomial(&[1.0, 0.0392313, -7.45044e-6], t + 200.0);
                (0.117058 * value, 0.117058 * slope)
            }
            Self::Cu427 if t <= 150.0 => polynomial(&[1.0, 0.0042743], t),
            Self::Cu427 => {
                let (value, slope) = polynomial(&[1.0, 0.00262628, 2.43732e-8], t - 150.0);
                (1.641145 * value, 1.641145 * slope)
            }
            Self::Ni617 if t < 100.0 => polynomial(&[1.0, 0.0054963, 6.7556e-6], t),
            Self::Ni617 => polynomial(
                &[1.0, 0.0054963, 6.7556e-6 - 100.0 * 9.2004e-9, 9.2004e-9],
                t,
            ),
            Self::Ni618 => polynomial(&[1.0, 0.005485, 6.65e-6, 0.0, 2.805e-11, 0.0, -2.0e-17], t),
            Self::Ni672 => {
                let segment = NI672
                    .iter()
                    .find(|segment| t <= segment.1)
                    .unwrap_or(&NI672[NI672.len() - 1]);
                polynomial(&segment.0, t)
            }
            Self::Ni1000 => polynomial(&[1.0, 0.004427, 5.172e-6, 5.585e-9], t),
        }
    }

    /// CVD inversion needs a strictly increasing, positive characteristic over
    /// the entire supported range. Check the extrema of dR/dt analytically.
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

// Piecewise cubic fits for Ni120 with alpha = 0.00672. Each row is
// ([constant, t, t², t³], upper temperature). The lower limit is -80 °C.
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
