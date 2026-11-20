//! CVD 基函数与三元方程，以及连续分层的标准大气正反算。
/// C 项仅作用于负摄氏温区；非负温区不能用来独立确定 C。
pub(crate) fn cvd_basis(t: f64) -> [f64; 3] {
    [
        t,
        t * t,
        if t < 0.0 {
            (t - 100.0) * t.powi(3)
        } else {
            0.0
        },
    ]
}

pub(crate) fn cvd_resistance(t: f64, r0: f64, a: f64, b: f64, c: f64) -> f64 {
    r0 * (1.0
        + a * t
        + b * t * t
        + if t < 0.0 {
            c * (t - 100.0) * t.powi(3)
        } else {
            0.0
        })
}

/// 列缩放加部分选主元的消元；退化校准点返回 None，不输出不可靠系数。
pub(crate) fn solve_3x3(rows: Vec<[f64; 3]>, rhs: Vec<f64>) -> Option<[f64; 3]> {
    if rows.len() != 3 || rhs.len() != 3 {
        return None;
    }
    if rows
        .iter()
        .flatten()
        .chain(rhs.iter())
        .any(|x| !x.is_finite())
    {
        return None;
    }
    let scales: [f64; 3] =
        std::array::from_fn(|column| rows.iter().map(|r| r[column].abs()).fold(0.0, f64::max));
    if scales.contains(&0.0) {
        return None;
    }
    let mut a = [[0.0; 4]; 3];
    for row in 0..3 {
        for column in 0..3 {
            a[row][column] = rows[row][column] / scales[column];
        }
        a[row][3] = rhs[row];
    }
    for col in 0..3 {
        let pivot = (col..3).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))?;
        if a[pivot][col].abs() < 1.0e-12 {
            return None;
        }
        a.swap(col, pivot);
        let divisor = a[col][col];
        for cell in &mut a[col][col..] {
            *cell /= divisor;
        }
        let pivot_row = a[col];
        for (row, r) in a.iter_mut().enumerate() {
            if row == col {
                continue;
            }
            let factor = r[col];
            for (cell, &p) in r[col..].iter_mut().zip(&pivot_row[col..]) {
                *cell -= factor * p;
            }
        }
    }
    let result = std::array::from_fn(|i| a[i][3] / scales[i]);
    result.iter().all(|x| x.is_finite()).then_some(result)
}

/// 位势高度 m 对应的标准温度 K；各层基准温度与压力模型采用相同分界。
pub(crate) fn isa_temperature(altitude_m: f64) -> f64 {
    let layers = [
        (0.0, 288.15, -0.0065),
        (11000.0, 216.65, 0.0),
        (20000.0, 216.65, 0.001),
        (32000.0, 228.65, 0.0028),
        (47000.0, 270.65, 0.0),
        (51000.0, 270.65, -0.0028),
        (71000.0, 214.65, -0.002),
    ];
    let &(base, t, lapse) = layers
        .iter()
        .rev()
        .find(|(h, _, _)| altitude_m >= *h)
        .unwrap_or(&layers[0]);
    t + lapse * (altitude_m - base)
}

/// −1000～86000 m 的标准绝对压力 Pa；逐层传递压力，保证边界连续。
pub(crate) fn isa_pressure(altitude_m: f64) -> Option<f64> {
    isa_pressure_with_steps(altitude_m, |_, _, _, _, _, _| {})
}

/// 可选接收每层的计算值；正算、反算和过程展示使用同一分层模型。
pub(crate) fn isa_pressure_with_steps(
    altitude_m: f64,
    mut step: impl FnMut(f64, f64, f64, f64, f64, f64),
) -> Option<f64> {
    if !(-1000.0..=86000.0).contains(&altitude_m) {
        return None;
    }
    const G: f64 = 9.806_65;
    const R: f64 = 287.052_87;
    let mut pressure = 101325.0;
    let mut base = 0.0;
    for (top, lapse) in [
        (11000., -0.0065),
        (20000., 0.),
        (32000., 0.001),
        (47000., 0.0028),
        (51000., 0.),
        (71000., -0.0028),
        (86000., -0.002),
    ] {
        let end = altitude_m.min(top);
        let temp = isa_temperature(base);
        let start_pressure = pressure;
        pressure *= if lapse == 0.0 {
            (-G * (end - base) / (R * temp)).exp()
        } else {
            ((temp + lapse * (end - base)) / temp).powf(-G / (R * lapse))
        };
        step(base, end, temp, lapse, start_pressure, pressure);
        if altitude_m <= top {
            return Some(pressure);
        }
        base = top;
    }
    Some(pressure)
}

/// 在同一分层模型上二分反算，返回位势高度，不额外解释为几何海拔。
pub(crate) fn isa_altitude(pressure_pa: f64) -> Option<f64> {
    if !pressure_pa.is_finite() || pressure_pa <= 0.0 {
        return None;
    }
    if pressure_pa < isa_pressure(86000.)? || pressure_pa > isa_pressure(-1000.)? {
        return None;
    }
    let (mut low, mut high) = (-1000., 86000.);
    for _ in 0..90 {
        let mid = (low + high) / 2.;
        if isa_pressure(mid)? > pressure_pa {
            low = mid;
        } else {
            high = mid;
        }
    }
    Some((low + high) / 2.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atmosphere_steps_match_forward_and_inverse_models() {
        for height in [
            -1000.0, 0.0, 10_999.999, 11_000.0, 11_000.001, 15_000.0, 20_000.0, 32_000.0, 47_000.0,
            51_000.0, 71_000.0, 86_000.0,
        ] {
            let mut steps = Vec::new();
            let pressure =
                isa_pressure_with_steps(height, |base, end, temp, lapse, start, end_pressure| {
                    steps.push((base, end, temp, lapse, start, end_pressure));
                })
                .unwrap();
            assert_eq!(pressure, isa_pressure(height).unwrap());
            assert_eq!(steps.last().unwrap().1, height);
            assert_eq!(steps.last().unwrap().5, pressure);
            for adjacent in steps.windows(2) {
                assert_eq!(adjacent[0].1, adjacent[1].0);
                assert_eq!(adjacent[0].5, adjacent[1].4);
            }
            assert!((isa_altitude(pressure).unwrap() - height).abs() < 1e-6);
        }
        assert!(
            isa_pressure_with_steps(86_001.0, |_, _, _, _, _, _| panic!("无效高度不能输出过程"))
                .is_none()
        );
    }
}
