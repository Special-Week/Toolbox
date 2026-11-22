//! Planck 辐射平衡与灰体反射；对外温度用 °C，辐射计算内部使用 K、波长用 μm。
use crate::calculators::numerics::finite;

// c2 = h*c/k，采用 SI 精确定义常数，单位为 µm·K。
pub const C2: f64 = 6.62607015e-34 * 299792458.0 / 1.380649e-23 * 1e6;

pub enum Response {
    Single(f64),
    Band(f64, f64),
}
pub struct Spectrum {
    nodes: Vec<(f64, f64)>,
}

fn log_expm1(x: f64) -> f64 {
    if x > 50.0 { x } else { x.exp_m1().ln() }
}

fn log_sum(logs: &[f64]) -> f64 {
    let maximum = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !maximum.is_finite() {
        return maximum;
    }
    maximum + logs.iter().map(|l| (l - maximum).exp()).sum::<f64>().ln()
}

impl Spectrum {
    pub fn new(response: Response) -> Result<Self, String> {
        let valid = |w: f64| w.is_finite() && (0.1..=100.0).contains(&w);
        let nodes = match response {
            Response::Single(w) if valid(w) => vec![(w, -5.0 * w.ln())],
            Response::Band(low, high) if valid(low) && valid(high) && low < high => {
                // 光谱响应视为平坦，采用复合辛普森积分。
                let intervals = 256;
                let h = (high - low) / intervals as f64;
                (0..=intervals)
                    .map(|i| {
                        let wavelength = low + i as f64 * h;
                        let weight = if i == 0 || i == intervals {
                            1.0
                        } else if i % 2 == 0 {
                            2.0
                        } else {
                            4.0
                        };
                        (wavelength, (weight * h / 3.0).ln() - 5.0 * wavelength.ln())
                    })
                    .collect()
            }
            _ => return Err("波长须在 0.1～100 µm 内，波段上限必须大于下限".to_owned()),
        };
        Ok(Self { nodes })
    }

    /// 对数域累加光谱信号，避免低温短波长时直接计算指数或微小辐射而溢出/下溢。
    /// 比较与反算中共同的辐射常数可约去，保留 λ⁻⁵ 和响应积分权重即可。
    pub fn log_radiance(&self, kelvin: f64) -> f64 {
        if kelvin <= 0.0 {
            return f64::NEG_INFINITY;
        }
        let logs: Vec<_> = self
            .nodes
            .iter()
            .map(|&(w, weight)| weight - log_expm1((C2 / w) / kelvin))
            .collect();
        log_sum(&logs)
    }

    fn inverse(&self, logarithm: f64) -> Result<f64, String> {
        finite(&[logarithm])?;
        let (mut low, mut high) = (0.0, 512.0);
        while self.log_radiance(high) < logarithm {
            if high == f64::MAX {
                return Err("辐射温度超出双精度范围".to_owned());
            }
            high = (2.0 * high).min(f64::MAX);
        }
        for _ in 0..85 {
            let middle = low + (high - low) * 0.5;
            if self.log_radiance(middle) < logarithm {
                low = middle;
            } else {
                high = middle;
            }
        }
        Ok(low + (high - low) * 0.5)
    }

    pub(super) fn log_balance(&self, temperature: f64, background: f64, numerator: f64, denominator: f64) -> Result<f64, String> {
        let (t, bg) = (kelvin(temperature)?, kelvin(background)?);
        let lt = self.log_radiance(t);
        let lb = self.log_radiance(bg);
        let log_weight = numerator.ln() - denominator.ln();
        if numerator < denominator {
            let log_complement = (denominator - numerator).ln() - denominator.ln();
            return Ok(log_sum(&[lt + log_weight, lb + log_complement]));
        }
        if numerator == denominator || temperature == background {
            return Ok(lt);
        }

        // (B热 − B冷) / B热 = [1 − exp(−Δx)] / [1 − exp(−x冷)]。
        // 先按波长算辐射差，避免接近等温时相减两个已舍入的积分值。
        let hotter = t.max(bg);
        let colder = t.min(bg);
        let delta_t = (temperature - background).abs();
        let log_hot = lt.max(lb);
        let logs: Vec<_> = self.nodes.iter().map(|&(w, weight)| {
            let c = C2 / w;
            let x_cold = c / colder;
            let x_hot = c / hotter;
            let dx = x_cold * (delta_t / hotter);
            let log_step = if dx.is_normal() {
                (-(-dx).exp_m1()).ln()
            } else {
                x_cold.ln() + delta_t.ln() - hotter.ln()
            };
            weight - log_expm1(x_hot) - log_hot + log_step - (-(-x_cold).exp_m1()).ln()
        }).collect();
        let log_change = log_weight + log_sum(&logs);
        if temperature > background {
            return Ok(log_hot + log_sum(&[lb - log_hot, log_change]));
        }
        if log_change >= 0.0 {
            return Err("当前发射率、示值与背景温度无法得到有效辐射信号".to_owned());
        }
        let log_remaining = if log_change < -std::f64::consts::LN_2 {
            (-log_change.exp()).ln_1p()
        } else {
            (-log_change.exp_m1()).ln()
        };
        Ok(lb + log_remaining)
    }

    fn transform(&self, temperature: f64, background: f64, numerator: f64, denominator: f64) -> Result<f64, String> {
        if temperature == background || numerator == denominator {
            kelvin(temperature)?;
            kelvin(background)?;
            return Ok(temperature);
        }
        Ok(self.inverse(self.log_balance(temperature, background, numerator, denominator)?)? - 273.15)
    }

    pub fn apparent(
        &self,
        true_c: f64,
        background: f64,
        emissivity: f64,
    ) -> Result<f64, String> {
        valid_emissivity(emissivity)?;
        self.transform(true_c, background, emissivity, 1.0)
    }

    pub fn corrected(
        &self,
        indicated: f64,
        background: f64,
        instrument_emissivity: f64,
        source_emissivity: f64,
    ) -> Result<f64, String> {
        valid_emissivity(instrument_emissivity)?;
        valid_emissivity(source_emissivity)?;
        // εs B(Ts) + (1-εs)B(Tr) = εi B(Ti) + (1-εi)B(Tr).
        self.transform(indicated, background, instrument_emissivity, source_emissivity)
    }
}

fn kelvin(celsius: f64) -> Result<f64, String> {
    let value = celsius + 273.15;
    if !value.is_finite() || value <= 0.0 {
        return Err("温度必须高于绝对零度".to_owned());
    }
    Ok(value)
}

fn valid_emissivity(value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 || value > 1.0 {
        Err("发射率须满足 0 < ε ≤ 1".to_owned())
    } else {
        Ok(())
    }
}

/// 一次反射截断近似，返回亮温 °C；等温时仍可能存在截断造成的小偏差。
pub fn ear_single_reflection(
    blackbody_c: f64,
    blackbody_e: f64,
    probe_c: f64,
    probe_e: f64,
) -> Result<f64, String> {
    valid_emissivity(blackbody_e)?;
    valid_emissivity(probe_e)?;
    let b = kelvin(blackbody_c)?;
    let p = kelvin(probe_c)?;
    let scale = b.max(p);
    let signal = blackbody_e * (b / scale).powi(4)
        + (1.0 - blackbody_e)
            * (probe_e * (p / scale).powi(4) + (1.0 - probe_e) * blackbody_e * (b / scale).powi(4));
    let result = scale * signal.sqrt().sqrt() - 273.15;
    finite(&[result])?;
    Ok(result)
}

/// 两灰体完全相互可见的多次反射平衡，返回亮温 °C，不直接返回温差修正。
pub fn ear_blackbody(
    blackbody_c: f64,
    blackbody_e: f64,
    probe_c: f64,
    probe_e: f64,
) -> Result<f64, String> {
    valid_emissivity(blackbody_e)?;
    valid_emissivity(probe_e)?;
    let blackbody = kelvin(blackbody_c)?;
    let probe = kelvin(probe_c)?;
    let scale = blackbody.max(probe);
    // 累加灰体表面之间的多次反射；只算一次会损失能量，
    // 并在两个表面等温时错误地产生非零修正值。
    let denominator = blackbody_e + (1.0 - blackbody_e) * probe_e;
    let blackbody_weight = blackbody_e / denominator;
    let probe_weight = (1.0 - blackbody_e) * probe_e / denominator;
    let signal =
        blackbody_weight * (blackbody / scale).powi(4) + probe_weight * (probe / scale).powi(4);
    let light_c = scale * signal.sqrt().sqrt() - 273.15;
    finite(&[light_c])?;
    Ok(light_c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planck_unit_weight_ignores_background_radiance() {
        let spectrum = Spectrum::new(Response::Single(0.1)).unwrap();
        assert_eq!(spectrum.apparent(-180.0, 30.0, 1.0).unwrap(), -180.0);
        assert_eq!(spectrum.corrected(-180.0, 30.0, 0.9, 0.9).unwrap(), -180.0);
        assert!(spectrum.apparent(-180.0, -273.15, 1.0).is_err());
    }
}
