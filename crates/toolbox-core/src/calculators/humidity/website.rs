//! 分段饱和压与湿空气参数；温度使用 °C、绝对压力与分压使用 Pa。
//! 湿球正反算共用所填压力，零湿度以 None 表示不存在有限露点或霜点。
use crate::calculators::{calibration::inverse, numerics::finite};

fn liquid_saturation(t: f64) -> f64 {
    let kelvin = t + 273.15;
    10.0_f64.powf((10.286 * kelvin - 2148.4909) / (kelvin - 35.85))
}

pub fn water_boundary() -> f64 {
    liquid_saturation(1.0)
}

pub fn low_water_coefficient() -> f64 {
    // Make the water branches meet at 1 °C while retaining 611.2 Pa at 0 °C.
    244.12 * (water_boundary() / 611.2).ln()
}

pub fn saturation(t: f64) -> Result<f64, String> {
    if !(-100.0..=100.0).contains(&t) {
        return Err("环境及湿球温度须在 −100～100 °C 内".into());
    }
    Ok(if t <= 0.0 {
        611.2 * (22.46 * t / (272.62 + t)).exp()
    } else if t < 1.0 {
        611.2 * (low_water_coefficient() * t / (243.12 + t)).exp()
    } else {
        liquid_saturation(t)
    })
}
pub struct ResultSet {
    pub rh: f64,
    pub dew: Option<f64>,
    pub frost: Option<f64>,
    pub wet: Option<f64>,
    pub saturated: f64,
    pub vapor: f64,
    pub raw_vapor: f64,
    pub volume_fraction: f64,
    /// 以干空气为分母的水汽体积比，单位 μL/L；不同于以总空气计的体积分数。
    pub ppm: f64,
    pub absolute: f64,
    pub mass: f64,
    pub specific: f64,
    pub mixing: f64,
    /// 按 0 °C、101325 Pa 状态表示的含水量，单位 g/Nm³。
    pub normal: f64,
}
/// known=0/1/2/3/4 分别为 %RH、露/霜点 °C、湿球 °C、混合比 g/kg、水面露点 °C。
pub fn calculate(t: f64, p: f64, known: usize, value: f64) -> Result<ResultSet, String> {
    finite(&[t, p, value])?;
    if p <= 0.0 {
        return Err("大气压必须大于 0".into());
    }
    if !matches!(known, 1..=4) && !(0.0..=100.0).contains(&value) {
        return Err("相对湿度必须在 0%～100% 内".into());
    }
    let saturated = saturation(t)?;
    let raw_vapor = match known {
        1 => saturation(value)?,
        2 => saturation(value)? - 0.000662 * p * (t - value),
        3 => {
            if value < 0.0 {
                return Err("混合比不能为负数".into());
            }
            let q = value / 1000.0 * 28.959 / 18.0152;
            p * q / (1.0 + q)
        }
        4 if value <= 0.0 => {
            if value <= -243.12 {
                return Err("水面露点须高于 −243.12 °C".into());
            }
            let vapor = (611.2_f64.ln() + 17.62 * value / (243.12 + value)).exp();
            if vapor == 0.0 {
                return Err("水面露点对应的水汽分压低于双精度可表示范围".into());
            }
            vapor
        }
        4 => saturation(value)?,
        _ if value == 100.0 => saturated,
        _ => saturated * value / 100.0,
    };
    let raw_rh = raw_vapor * 100.0 / saturated;
    // 低温时饱和压很小，湿球温度的舍入误差应在分压单位下估计。
    let wet_roundoff = if known == 2 {
        0.000662 * p * (t.abs().max(value.abs()).max(1.0) * (2.0 * f64::EPSILON))
    } else {
        0.0
    };
    // 仅修正反算时越过边界的微小数值误差，并让后续计算使用同一分压。
    let vapor = if raw_vapor < 0.0 && (raw_rh >= -1e-10 || raw_vapor >= -wet_roundoff) {
        0.0
    } else if raw_rh > 100.0 && raw_rh <= 100.0 + 1e-8 {
        saturated
    } else {
        raw_vapor
    };
    let rh = if vapor == 0.0 {
        0.0
    } else if vapor == saturated {
        100.0
    } else {
        vapor * 100.0 / saturated
    };
    if !(0.0..=100.0).contains(&rh) || vapor >= p {
        return Err(
            "输入应满足 0 ≤ 相对湿度 ≤ 100%，水汽分压小于大气压；露/霜点及湿球不高于环境温度"
                .into(),
        );
    }
    let (dew, frost) = if vapor > 0.0 {
        // 对极小分压先取对数，避免相除时下溢为零。
        let z = vapor.ln() - 611.2_f64.ln();
        let dew = if vapor < water_boundary() {
            let coefficient = if vapor <= 611.2 { 17.62 } else { low_water_coefficient() };
            243.12 * z / (coefficient - z)
        } else {
            (-2148.4909 + vapor.log10() * 35.85) / (vapor.log10() - 10.286) - 273.15
        };
        (Some(dew), Some(272.62 * z / (22.46 - z)))
    } else {
        (None, None)
    };
    let wet_equation = |w| Ok(saturation(w)? - 0.000662 * p * (t - w));
    let wet = if vapor == saturated {
        Some(t)
    } else if vapor < wet_equation(-100.0)? {
        None
    } else {
        Some(inverse(wet_equation, vapor, -100.0, t)?)
    };
    let ppm = vapor / (p - vapor) * 1e6;
    let mass = ppm * 18.0152 / 28.959;
    let absolute = vapor * 18.015 / ((t + 273.15) * 8.31451);
    let normal = absolute * (t + 273.15) * 101325.0 / (273.15 * p);
    finite(&[ppm, mass, absolute, normal])?;
    Ok(ResultSet {
        rh,
        dew,
        frost,
        wet,
        saturated,
        vapor,
        raw_vapor,
        volume_fraction: vapor / p,
        ppm,
        absolute,
        mass,
        specific: mass / (1e6 + mass) * 1000.0,
        mixing: mass / 1000.0,
        normal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dry_cold_wet_bulb_can_be_used_as_input() {
        let dry = calculate(-90.0, 85_000.0, 0, 0.0).unwrap();
        let wet = dry.wet.unwrap();
        let returned = calculate(-90.0, 85_000.0, 2, wet).unwrap();
        assert!(returned.rh.abs() < 1e-8);
        assert!(calculate(-90.0, 85_000.0, 2, wet - 1e-8).is_err());
    }

    #[test]
    fn liquid_saturation_and_its_inverse_agree_across_one_degree() {
        let p = 101325.0;
        let mut previous = saturation(0.9998).unwrap();
        for t in [0.9999, 1.0, 1.0001] {
            let pressure = saturation(t).unwrap();
            assert!(pressure > previous);
            assert!((pressure - previous) < 0.005);
            previous = pressure;
            let saturated = calculate(t, p, 0, 100.0).unwrap();
            assert_eq!(saturated.wet, Some(t));
            assert!((saturated.dew.unwrap() - t).abs() < 1e-10);
            let from_dew = calculate(1.0002, p, 1, t).unwrap();
            assert!((from_dew.dew.unwrap() - t).abs() < 1e-10);
            let from_wet = calculate(1.0002, p, 2, t).unwrap();
            assert!((from_wet.wet.unwrap() - t).abs() < 1e-10);
        }
        assert!(calculate(1.0, p, 1, 0.9999).unwrap().rh < 100.0);
    }
}
