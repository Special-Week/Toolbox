//! 热电阻预设目录与有界双向换算；曲线统一表示 R/R₀，再按当前 R₀ 缩放。
use super::curves::{Curve, PT385, PolynomialForm};
use crate::calculators::common::format_number;

#[derive(Clone, Copy, PartialEq)]
pub enum ToleranceKind {
    Platinum,
    // 非 385 特性或自定义 CVD：允许展示公式参考值，不声称符合 IEC 标准特性。
    PlatinumReference,
    Copper,
    None,
}

pub struct Graduation {
    pub key: &'static str,
    pub name: &'static str,
    pub r0: f64,
    pub min: f64,
    pub max: f64,
    pub curve: Curve,
    pub tolerance: ToleranceKind,
}

impl Graduation {
    /// 稳定标识与显示名称分离，修改标签不会混用不同分度号的数值。
    pub fn field_key(&self, field: &str) -> String {
        format!("rtd.{}.{field}", self.key)
    }

    pub fn custom(&self) -> bool {
        self.key == "cvd"
    }
}

macro_rules! rtd {
    ($key:literal, $name:literal, $r0:expr, $min:expr, $max:expr, $curve:expr, $tolerance:ident) => {
        Graduation {
            key: $key,
            name: $name,
            r0: $r0,
            min: $min,
            max: $max,
            curve: $curve,
            tolerance: ToleranceKind::$tolerance,
        }
    };
}

pub const GRADUATIONS: [Graduation; 22] = [
    rtd!(
        "pt100_385",
        "Pt100 (385)",
        100.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt100_3910",
        "Pt100 (3910)",
        100.0,
        -200.0,
        850.0,
        Curve::Cvd([0.003969, -5.841e-7, -4.33e-12]),
        PlatinumReference
    ),
    rtd!(
        "pt100_3911",
        "Pt100 (3911)",
        100.0,
        -200.0,
        850.0,
        Curve::Cvd([0.0039692, -5.8495e-7, -4.2325e-12]),
        PlatinumReference
    ),
    rtd!(
        "pt100_3916",
        "Pt100 (3916)",
        100.0,
        -200.0,
        850.0,
        Curve::Pt3916,
        PlatinumReference
    ),
    rtd!(
        "pt100_3926",
        "Pt100 (3926)",
        100.0,
        -200.0,
        850.0,
        Curve::Cvd([0.0039848, -5.87e-7, -4.0e-12]),
        PlatinumReference
    ),
    rtd!(
        "pt10_385",
        "Pt10 (385)",
        10.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt25_385",
        "Pt25 (385)",
        25.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt50_385",
        "Pt50 (385)",
        50.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt200_385",
        "Pt200 (385)",
        200.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt300_385",
        "Pt300 (385)",
        300.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt400_385",
        "Pt400 (385)",
        400.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt500_385",
        "Pt500 (385)",
        500.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt1000_385",
        "Pt1000 (385)",
        1000.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "pt2000_385",
        "Pt2000 (385)",
        2000.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        Platinum
    ),
    rtd!(
        "cu50_428",
        "Cu50 (428)",
        50.0,
        -50.0,
        150.0,
        Curve::Cu428,
        Copper
    ),
    rtd!(
        "cu100_428",
        "Cu100 (428)",
        100.0,
        -50.0,
        150.0,
        Curve::Cu428,
        Copper
    ),
    rtd!(
        "cu10_427",
        "Cu10 (427)",
        9.035,
        -200.0,
        260.0,
        Curve::Cu427,
        Copper
    ),
    rtd!(
        "ni100_617",
        "Ni100 (617)",
        100.0,
        -60.0,
        180.0,
        Curve::Ni617,
        None
    ),
    rtd!(
        "ni100_618",
        "Ni100 (618)",
        100.0,
        -100.0,
        260.0,
        Curve::Ni618,
        None
    ),
    rtd!(
        "ni120_672",
        "Ni120 (672)",
        120.0,
        -80.0,
        260.0,
        Curve::Ni672,
        None
    ),
    rtd!(
        "ni1000",
        "Ni1000",
        1000.0,
        -50.0,
        150.0,
        Curve::Ni1000,
        None
    ),
    rtd!(
        "cvd",
        "CVD（自定义）",
        100.0,
        -200.0,
        850.0,
        Curve::Cvd(PT385),
        PlatinumReference
    ),
];

pub struct RtdModel {
    pub graduation: &'static Graduation,
    pub r0: f64,
    curve: Curve,
}

impl RtdModel {
    pub fn polynomial_form(&self, temperature: f64) -> Option<PolynomialForm> {
        self.curve.polynomial_form(temperature)
    }
    pub fn cvd_coefficients(&self) -> Option<[f64; 3]> {
        match self.curve {
            Curve::Cvd(coefficients) => Some(coefficients),
            _ => None,
        }
    }

    pub fn new(
        graduation: &'static Graduation,
        r0: f64,
        custom_coefficients: Option<[f64; 3]>,
    ) -> Result<Self, String> {
        if !r0.is_finite() || r0 <= 0.0 {
            return Err("R₀ 必须是大于 0 的有限数字".to_owned());
        }
        let curve = if graduation.custom() {
            Curve::Cvd(custom_coefficients.unwrap_or(PT385))
        } else {
            graduation.curve
        };
        curve.validate(graduation.min, graduation.max)?;
        let model = Self {
            graduation,
            r0,
            curve,
        };
        model.resistance(graduation.min)?;
        model.resistance(graduation.max)?;
        Ok(model)
    }

    pub fn resistance(&self, temperature: f64) -> Result<f64, String> {
        if !temperature.is_finite()
            || !(self.graduation.min..=self.graduation.max).contains(&temperature)
        {
            return Err(format!(
                "{} 的换算温度范围为 {}～{} °C",
                self.graduation.name,
                format_number(self.graduation.min),
                format_number(self.graduation.max),
            ));
        }
        let resistance = self.r0 * self.curve.evaluate(temperature).0;
        if !resistance.is_finite() || resistance <= 0.0 {
            return Err("电阻计算结果超出有效范围，请检查 R₀ 与系数".to_owned());
        }
        Ok(resistance)
    }

    pub fn temperature(&self, resistance: f64) -> Result<f64, String> {
        let low = self.graduation.min;
        let high = self.graduation.max;
        let low_r = self.resistance(low)?;
        let high_r = self.resistance(high)?;
        if !resistance.is_finite() || !(low_r..=high_r).contains(&resistance) {
            return Err(format!(
                "按当前 R₀，电阻应在 {}～{} Ω 之间（{}～{} °C）",
                format_number(low_r),
                format_number(high_r),
                format_number(low),
                format_number(high),
            ));
        }
        let mut solution: Option<f64> = None;
        for (mut start, mut end) in self.curve.inverse_intervals(low, high) {
            let (a, b) = (self.resistance(start)?, self.resistance(end)?);
            if !(a..=b).contains(&resistance) {
                continue;
            }
            let temperature = if resistance == a {
                start
            } else if resistance == b {
                end
            } else {
                for _ in 0..80 {
                    let mid = start + (end - start) * 0.5;
                    if self.resistance(mid)? < resistance {
                        start = mid;
                    } else {
                        end = mid;
                    }
                }
                start + (end - start) * 0.5
            };
            if (self.resistance(temperature)? - resistance).abs() > 1e-9 * resistance.max(1.0) {
                continue;
            }
            if solution.is_some_and(|previous| (previous - temperature).abs() > 1e-9) {
                return Err("该电阻在分段曲线中对应多个温度，无法唯一反算".to_owned());
            }
            solution = Some(temperature);
        }
        solution.ok_or_else(|| "该电阻位于分段曲线间隙，无法可靠反算温度".to_owned())
    }

    pub fn slope(&self, temperature: f64) -> Result<f64, String> {
        self.resistance(temperature)?;
        let slope = self.r0 * self.curve.evaluate(temperature).1;
        if !slope.is_finite() || slope <= 0.0 {
            return Err("当前温度处的电阻变化率无效".to_owned());
        }
        Ok(slope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_copper_and_nickel_segments_report_multiple_temperatures() {
        for (key, temperature) in [("cu10_427", -50.0), ("ni120_672", 240.0)] {
            let graduation = GRADUATIONS.iter().find(|g| g.key == key).unwrap();
            let model = RtdModel::new(graduation, graduation.r0, None).unwrap();
            let resistance = model.resistance(temperature).unwrap();
            assert!(model.temperature(resistance).unwrap_err().contains("多个温度"));
        }
    }

    #[test]
    fn nickel_inverse_rejects_a_gap_between_segments() {
        let graduation = GRADUATIONS.iter().find(|g| g.key == "ni120_672").unwrap();
        let model = RtdModel::new(graduation, graduation.r0, None).unwrap();
        assert!(model.temperature(219.289424).is_err());
        for temperature in [119.999, 120.0, 120.001] {
            let resistance = model.resistance(temperature).unwrap();
            assert!((model.temperature(resistance).unwrap() - temperature).abs() < 1e-8);
        }
    }
}

#[derive(Clone, Copy)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

impl TemperatureUnit {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Kelvin => "K",
        }
    }

    pub fn display_temperature(self, t: f64) -> f64 {
        match self {
            Self::Celsius => t,
            Self::Fahrenheit => t * 1.8 + 32.0,
            Self::Kelvin => t + 273.15,
        }
    }

    pub fn to_celsius(self, t: f64) -> f64 {
        match self {
            Self::Celsius => t,
            Self::Fahrenheit => (t - 32.0) / 1.8,
            Self::Kelvin => t - 273.15,
        }
    }

    pub fn span_factor(self) -> f64 {
        match self {
            Self::Fahrenheit => 1.8,
            _ => 1.0,
        }
    }
}
