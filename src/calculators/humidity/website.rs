//! 分段饱和压与湿空气参数；温度使用 °C、绝对压力与分压使用 Pa。
//! 湿球正反算共用所填压力，零湿度以 None 表示不存在有限露点或霜点。
use crate::calculators::{calibration::inverse, numerics::finite};
pub(crate) fn saturation(t: f64) -> Result<f64, String> {
    if !(-100.0..=100.0).contains(&t) {
        return Err("环境及湿球温度须在 −100～100 °C 内".into());
    }
    Ok(if t <= 0.0 {
        611.2 * (22.46 * t / (272.62 + t)).exp()
    } else if t < 1.0 {
        611.2 * (17.62 * t / (243.12 + t)).exp()
    } else {
        10.0_f64.powf((10.286 * (t + 273.15) - 2148.4909) / (t + 273.15 - 35.85))
    })
}
pub(crate) struct ResultSet {
    pub rh: f64,
    pub dew: Option<f64>,
    pub frost: Option<f64>,
    pub wet: f64,
    pub saturated: f64,
    pub vapor: f64,
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
/// known=0/1/2/3 分别为 %RH、露/霜点 °C、湿球 °C、混合比 g/kg。
pub(crate) fn calculate(t: f64, p: f64, known: usize, value: f64) -> Result<ResultSet, String> {
    finite(&[t, p, value])?;
    if p <= 0.0 {
        return Err("大气压必须大于 0".into());
    }
    let saturated = saturation(t)?;
    let vapor = match known {
        1 => saturation(value)?,
        2 => saturation(value)? - 0.000662 * p * (t - value),
        3 => {
            if value < 0.0 {
                return Err("混合比不能为负数".into());
            }
            let q = value / 1000.0 * 28.959 / 18.0152;
            p * q / (1.0 + q)
        }
        _ => saturated * value / 100.0,
    };
    let mut rh = vapor / saturated * 100.0;
    if rh.abs() < 1e-10 {
        rh = 0.0;
    }
    if (rh - 100.0).abs() < 1e-8 {
        rh = 100.0;
    }
    if !(0.0..=100.0).contains(&rh) || vapor >= p {
        return Err(
            "输入应满足 0 ≤ 相对湿度 ≤ 100%，水汽分压小于大气压；露/霜点及湿球不高于环境温度"
                .into(),
        );
    }
    let (dew, frost) = if vapor > 0.0 {
        let z = (vapor / 611.2).ln();
        let dew = if vapor < 657.0 {
            243.12 * z / (17.62 - z)
        } else {
            (-2148.4909 + vapor.log10() * 35.85) / (vapor.log10() - 10.286) - 273.15
        };
        (Some(dew), Some(272.62 * z / (22.46 - z)))
    } else {
        (None, None)
    };
    let wet = inverse(
        |w| Ok(saturation(w)? - 0.000662 * p * (t - w)),
        vapor,
        -100.0,
        t,
    )?;
    let ppm = vapor / (p - vapor) * 1e6;
    let mass = ppm * 18.0152 / 28.959;
    let absolute = vapor * 18.015 / ((t + 273.15) * 8.31451);
    let normal = absolute * (t + 273.15) * 101325.0 / (273.15 * p);
    finite(&[wet, ppm, mass, absolute, normal])?;
    Ok(ResultSet {
        rh,
        dew,
        frost,
        wet,
        saturated,
        vapor,
        volume_fraction: vapor / p,
        ppm,
        absolute,
        mass,
        specific: mass / (1e6 + mass) * 1000.0,
        mixing: mass / 1000.0,
        normal,
    })
}
