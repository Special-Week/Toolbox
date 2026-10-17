//! 热电阻预设目录与有界双向换算；曲线统一表示 R/R₀，再按当前 R₀ 缩放。
use super::curves::{Curve, PT385};
use crate::calculators::common::format_number;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ToleranceKind {
    Platinum,
    // 非 385 特性或自定义 CVD：允许展示公式参考值，不声称符合 IEC 标准特性。
    PlatinumReference,
    Copper,
    None,
}

pub(super) struct Graduation {
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
    pub(super) fn field_key(&self, field: &str) -> String {
        format!("rtd.{}.{field}", self.key)
    }

    pub(super) fn custom(&self) -> bool {
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

pub(super) const GRADUATIONS: [Graduation; 22] = [
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

pub(super) struct RtdModel {
    pub graduation: &'static Graduation,
    pub r0: f64,
    curve: Curve,
}

impl RtdModel {
    pub(super) fn new(
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

    pub(super) fn resistance(&self, temperature: f64) -> Result<f64, String> {
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

    pub(super) fn temperature(&self, resistance: f64) -> Result<f64, String> {
        let mut low = self.graduation.min;
        let mut high = self.graduation.max;
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
        if resistance == low_r {
            return Ok(low);
        }
        if resistance == high_r {
            return Ok(high);
        }
        // Bounded bisection also handles B=0 CVDs, negative temperatures and
        // the source's copper/nickel piecewise curves without Newton overflow.
        for _ in 0..80 {
            let mid = low + (high - low) * 0.5;
            if self.resistance(mid)? < resistance {
                low = mid;
            } else {
                high = mid;
            }
        }
        Ok(low + (high - low) * 0.5)
    }

    pub(super) fn slope(&self, temperature: f64) -> Result<f64, String> {
        self.resistance(temperature)?;
        let slope = self.r0 * self.curve.evaluate(temperature).1;
        if !slope.is_finite() || slope <= 0.0 {
            return Err("当前温度处的电阻变化率无效".to_owned());
        }
        Ok(slope)
    }
}

#[derive(Clone, Copy)]
pub(super) enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

impl TemperatureUnit {
    pub(super) fn symbol(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Kelvin => "K",
        }
    }

    pub(super) fn display_temperature(self, t: f64) -> f64 {
        match self {
            Self::Celsius => t,
            Self::Fahrenheit => t * 1.8 + 32.0,
            Self::Kelvin => t + 273.15,
        }
    }

    pub(super) fn to_celsius(self, t: f64) -> f64 {
        match self {
            Self::Celsius => t,
            Self::Fahrenheit => (t - 32.0) / 1.8,
            Self::Kelvin => t - 273.15,
        }
    }

    pub(super) fn span_factor(self) -> f64 {
        match self {
            Self::Fahrenheit => 1.8,
            _ => 1.0,
        }
    }
}
