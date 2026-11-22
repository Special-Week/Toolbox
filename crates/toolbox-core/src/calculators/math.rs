//! CVD 基函数与三元方程，以及连续分层的标准大气正反算。
/// C 项仅作用于负摄氏温区；非负温区不能用来独立确定 C。
pub fn cvd_basis(t: f64) -> [f64; 3] {
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

pub fn cvd_resistance(t: f64, r0: f64, a: f64, b: f64, c: f64) -> f64 {
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
pub fn solve_3x3(rows: Vec<[f64; 3]>, rhs: Vec<f64>) -> Option<[f64; 3]> {
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
pub fn isa_molecular_temperature(altitude_m: f64) -> f64 {
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

/// 1976 标准大气表 8，在位势高度节点之间线性插值 M/M₀。
pub fn isa_kinetic_temperature(altitude_m: f64) -> f64 {
    const RATIOS: [(f64, f64); 13] = [
        (79005.7, 1.0), (79493.3, 0.999996), (79980.8, 0.999989),
        (80468.2, 0.999971), (80955.7, 0.999941), (81443.0, 0.999909),
        (81930.2, 0.999870), (82417.3, 0.999829), (82904.4, 0.999786),
        (83391.4, 0.999741), (83878.4, 0.999694), (84365.2, 0.999641),
        (84852.0, 0.9995788),
    ];
    let ratio = RATIOS.windows(2).find_map(|pair| {
        let [(lo, low_ratio), (hi, high_ratio)] = [pair[0], pair[1]];
        (altitude_m <= hi).then(|| {
            let fraction = ((altitude_m - lo) / (hi - lo)).clamp(0.0, 1.0);
            (high_ratio - low_ratio).mul_add(fraction, low_ratio)
        })
    }).unwrap_or(RATIOS[RATIOS.len() - 1].1);
    isa_molecular_temperature(altitude_m) * ratio
}

/// −1000～84852 m 的标准绝对压力 Pa；逐层传递压力，保证边界连续。
pub fn isa_pressure(altitude_m: f64) -> Option<f64> {
    isa_pressure_with_steps(altitude_m, |_, _, _, _, _, _| {})
}

/// 可选接收每层的计算值；正算、反算和过程展示使用同一分层模型。
pub fn isa_pressure_with_steps(
    altitude_m: f64,
    mut step: impl FnMut(f64, f64, f64, f64, f64, f64),
) -> Option<f64> {
    if !(-1000.0..=84852.0).contains(&altitude_m) {
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
        (84852., -0.002),
    ] {
        let end = altitude_m.min(top);
        let temp = isa_molecular_temperature(base);
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
pub fn isa_altitude(pressure_pa: f64) -> Option<f64> {
    if !pressure_pa.is_finite() || pressure_pa <= 0.0 {
        return None;
    }
    if pressure_pa < isa_pressure(84852.)? || pressure_pa > isa_pressure(-1000.)? {
        return None;
    }
    let (mut low, mut high) = (-1000., 84852.);
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

/// 只把界面实际显示的端点压力识别为该端点，其他压力保持严格的模型边界。
pub fn isa_altitude_from_display(
    input: f64,
    pascals_per_unit: f64,
    display: impl Fn(f64) -> String,
) -> Option<(f64, f64)> {
    for height in [-1000., 84852.] {
        let pressure = isa_pressure(height)?;
        if display(pressure / pascals_per_unit).parse::<f64>().ok() == Some(input) {
            return Some((height, pressure));
        }
    }
    let pressure = input * pascals_per_unit;
    Some((isa_altitude(pressure)?, pressure))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayed_atmosphere_endpoints_round_trip() {
        use crate::calculators::format::format_number;
        for (height, direction) in [(-1000., 1.), (84852., -1.)] {
            let pressure = isa_pressure(height).unwrap();
            let displayed = format_number(pressure / 1000.).parse::<f64>().unwrap();
            assert_eq!(isa_altitude_from_display(displayed, 1000., format_number),
                       Some((height, pressure)));
            let outside = pressure / 1000. + direction * 1e-6;
            assert_eq!(isa_altitude_from_display(outside, 1000., format_number), None);
        }
    }

    #[test]
    fn atmosphere_steps_match_forward_and_inverse_models() {
        for height in [
            -1000.0, 0.0, 10_999.999, 11_000.0, 11_000.001, 15_000.0, 20_000.0, 32_000.0, 47_000.0,
            51_000.0, 71_000.0, 84_852.0,
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
            isa_pressure_with_steps(84_852.001, |_, _, _, _, _, _| panic!("无效高度不能输出过程"))
                .is_none()
        );
    }
}
