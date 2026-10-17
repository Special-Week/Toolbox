//! Planck 辐射平衡与灰体反射；对外温度用 °C，辐射计算内部使用 K、波长用 μm。
use crate::calculators::numerics::finite;

// c2 = h*c/k, using the exact SI defining constants, expressed in µm·K.
const C2: f64 = 6.62607015e-34 * 299792458.0 / 1.380649e-23 * 1e6;

pub(crate) enum Response {
    Single(f64),
    Band(f64, f64),
}
pub(crate) struct Spectrum {
    nodes: Vec<(f64, f64)>,
}
impl Spectrum {
    pub(crate) fn new(response: Response) -> Result<Self, String> {
        let valid = |w: f64| w.is_finite() && (0.1..=100.0).contains(&w);
        let nodes = match response {
            Response::Single(w) if valid(w) => vec![(w, -5.0 * w.ln())],
            Response::Band(low, high) if valid(low) && valid(high) && low < high => {
                // Composite Simpson quadrature with flat spectral response.
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
    fn log_radiance(&self, kelvin: f64) -> f64 {
        if kelvin <= 0.0 {
            return f64::NEG_INFINITY;
        }
        let logs: Vec<_> = self
            .nodes
            .iter()
            .map(|&(w, weight)| {
                let x = C2 / (w * kelvin);
                let denominator = if x > 50.0 { x } else { x.exp_m1().ln() };
                weight - denominator
            })
            .collect();
        let maximum = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if !maximum.is_finite() {
            return maximum;
        }
        maximum + logs.iter().map(|l| (l - maximum).exp()).sum::<f64>().ln()
    }

    fn inverse(&self, logarithm: f64) -> Result<f64, String> {
        finite(&[logarithm])?;
        let (mut low, mut high) = (0.0, 512.0);
        while self.log_radiance(high) < logarithm {
            high *= 2.0;
            if !high.is_finite() {
                return Err("辐射温度超出双精度范围".to_owned());
            }
        }
        for _ in 0..85 {
            let middle = (low + high) * 0.5;
            if self.log_radiance(middle) < logarithm {
                low = middle;
            } else {
                high = middle;
            }
        }
        Ok((low + high) * 0.5)
    }

    fn transform(&self, temperature: f64, background: f64, weight: f64) -> Result<f64, String> {
        let (t, bg) = (kelvin(temperature)?, kelvin(background)?);
        let lt = self.log_radiance(t);
        let lb = self.log_radiance(bg);
        let scale = lt.max(lb);
        finite(&[lt, lb, scale, weight])?;
        let signal = weight.mul_add((lt - scale).exp(), (1.0 - weight) * (lb - scale).exp());
        if !signal.is_finite() || signal <= 0.0 {
            return Err("当前发射率、示值与背景温度无法得到有效辐射信号".to_owned());
        }
        Ok(self.inverse(scale + signal.ln())? - 273.15)
    }

    pub(crate) fn apparent(
        &self,
        true_c: f64,
        background: f64,
        emissivity: f64,
    ) -> Result<f64, String> {
        valid_emissivity(emissivity)?;
        self.transform(true_c, background, emissivity)
    }

    pub(crate) fn corrected(
        &self,
        indicated: f64,
        background: f64,
        instrument_emissivity: f64,
        source_emissivity: f64,
    ) -> Result<f64, String> {
        valid_emissivity(instrument_emissivity)?;
        valid_emissivity(source_emissivity)?;
        // εs B(Ts) + (1-εs)B(Tr) = εi B(Ti) + (1-εi)B(Tr).
        self.transform(
            indicated,
            background,
            instrument_emissivity / source_emissivity,
        )
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
pub(crate) fn ear_single_reflection(
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
pub(crate) fn ear_blackbody(
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
    // Sum repeated grey-surface reflections. A one-reflection truncation loses
    // energy and gives a nonzero correction even when both surfaces are isothermal.
    let denominator = blackbody_e + (1.0 - blackbody_e) * probe_e;
    let blackbody_weight = blackbody_e / denominator;
    let probe_weight = (1.0 - blackbody_e) * probe_e / denominator;
    let signal =
        blackbody_weight * (blackbody / scale).powi(4) + probe_weight * (probe / scale).powi(4);
    let light_c = scale * signal.sqrt().sqrt() - 273.15;
    finite(&[light_c])?;
    Ok(light_c)
}
