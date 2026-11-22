//! β 热敏电阻模型；内部温度为 K，对外温度参数和返回值为 °C，电阻为 Ω。
use super::numerics::finite;
#[path = "thermistor/polynomial.rs"]
pub mod polynomial;

pub struct BetaThermistor {
    r0: f64,
    t0: f64,
    beta: f64,
}

impl BetaThermistor {
    pub fn inverse_temperature(&self, resistance: f64) -> f64 {
        1.0 / self.t0 + (resistance.ln() - self.r0.ln()) / self.beta
    }

    pub fn log_resistance(&self, kelvin: f64) -> f64 {
        self.r0.ln() + self.beta * (1.0 / kelvin - 1.0 / self.t0)
    }

    pub fn new(r0: f64, t0_c: f64, beta: f64) -> Result<Self, String> {
        let t0 = t0_c + 273.15;
        finite(&[r0, t0, beta])?;
        if r0 <= 0.0 || t0 <= 0.0 || beta <= 0.0 {
            return Err("R₀、β 必须大于 0，参考温度必须高于绝对零度".to_owned());
        }
        Ok(Self { r0, t0, beta })
    }

    pub fn temperature(&self, resistance: f64) -> Result<f64, String> {
        if !resistance.is_finite() || resistance <= 0.0 {
            return Err("电阻必须是大于 0 的有限数字".to_owned());
        }
        // 两个对数相减可避免直接计算 R/R0 时上溢或下溢。
        let inverse = self.inverse_temperature(resistance);
        if inverse <= 0.0 {
            return Err("电阻对应的温度超出 β 模型范围".to_owned());
        }
        let temperature = 1.0 / inverse - 273.15;
        finite(&[temperature])?;
        Ok(temperature)
    }

    pub fn resistance(&self, temperature: f64) -> Result<f64, String> {
        let kelvin = temperature + 273.15;
        if !kelvin.is_finite() || kelvin <= 0.0 {
            return Err("温度必须高于绝对零度".to_owned());
        }
        let resistance = self.log_resistance(kelvin).exp();
        finite(&[resistance])?;
        if resistance <= 0.0 {
            return Err("电阻低于双精度可表示范围".to_owned());
        }
        Ok(resistance)
    }

    pub fn slope(&self, temperature: f64) -> Result<f64, String> {
        let resistance = self.resistance(temperature)?;
        self.slope_at_resistance(temperature, resistance)
    }

    pub fn slope_at_resistance(&self, temperature: f64, resistance: f64) -> Result<f64, String> {
        resistance_slope(resistance, self.beta, temperature + 273.15)
    }
}

fn resistance_slope(resistance: f64, coefficient: f64, kelvin: f64) -> Result<f64, String> {
    finite(&[resistance, coefficient, kelvin])?;
    if coefficient == 0.0 {
        return Ok(0.0);
    }
    let a = resistance / kelvin;
    let b = coefficient / kelvin;
    let mut slope = -a * b;
    if !a.is_normal() || !b.is_normal() || !slope.is_finite() {
        // 回退到对数域，避免两个因子量级相反时中间值先溢出或下溢。
        let log_magnitude = resistance.ln() + coefficient.abs().ln() - 2.0 * kelvin.ln();
        slope = log_magnitude.exp().copysign(-coefficient);
    }
    finite(&[slope])?;
    Ok(slope)
}
