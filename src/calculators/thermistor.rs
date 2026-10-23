//! β 热敏电阻模型；内部温度为 K，对外温度参数和返回值为 °C，电阻为 Ω。
use super::numerics::finite;
#[cfg(feature = "desktop-ui")]
#[path = "thermistor/page.rs"]
mod page;
#[path = "thermistor/polynomial.rs"]
pub(crate) mod polynomial;

pub(crate) struct BetaThermistor {
    r0: f64,
    t0: f64,
    beta: f64,
}

impl BetaThermistor {
    pub(crate) fn new(r0: f64, t0_c: f64, beta: f64) -> Result<Self, String> {
        let t0 = t0_c + 273.15;
        finite(&[r0, t0, beta])?;
        if r0 <= 0.0 || t0 <= 0.0 || beta <= 0.0 {
            return Err("R₀、β 必须大于 0，参考温度必须高于绝对零度".to_owned());
        }
        Ok(Self { r0, t0, beta })
    }

    pub(crate) fn temperature(&self, resistance: f64) -> Result<f64, String> {
        if !resistance.is_finite() || resistance <= 0.0 {
            return Err("电阻必须是大于 0 的有限数字".to_owned());
        }
        // Difference of logarithms avoids an overflowing/underflowing R/R0.
        let inverse = 1.0 / self.t0 + (resistance.ln() - self.r0.ln()) / self.beta;
        if inverse <= 0.0 {
            return Err("电阻对应的温度超出 β 模型范围".to_owned());
        }
        let temperature = 1.0 / inverse - 273.15;
        finite(&[temperature])?;
        Ok(temperature)
    }

    pub(crate) fn resistance(&self, temperature: f64) -> Result<f64, String> {
        let kelvin = temperature + 273.15;
        if !kelvin.is_finite() || kelvin <= 0.0 {
            return Err("温度必须高于绝对零度".to_owned());
        }
        let resistance = (self.r0.ln() + self.beta * (1.0 / kelvin - 1.0 / self.t0)).exp();
        finite(&[resistance])?;
        if resistance <= 0.0 {
            return Err("电阻低于双精度可表示范围".to_owned());
        }
        Ok(resistance)
    }
}
