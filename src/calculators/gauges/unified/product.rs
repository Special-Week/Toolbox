//! UN 工件中径等级模型。公差公式以英寸代入，输出统一转换为 mm；中途不修约。
use super::INCH;

#[derive(Clone, Copy)]
pub(super) enum ThreadClass {
    OneA,
    TwoA,
    ThreeA,
    OneB,
    TwoB,
    ThreeB,
}

impl ThreadClass {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "1A" => Ok(Self::OneA),
            "2A" => Ok(Self::TwoA),
            "3A" => Ok(Self::ThreeA),
            "1B" => Ok(Self::OneB),
            "2B" => Ok(Self::TwoB),
            "3B" => Ok(Self::ThreeB),
            _ => Err("请选择 1A、2A、3A 或 1B、2B、3B 工件公差等级".into()),
        }
    }

    pub fn external(self) -> bool {
        matches!(self, Self::OneA | Self::TwoA | Self::ThreeA)
    }

    /// 在 2A 基础公差上换算；采用计算值，不先截成六位英寸小数。
    fn tolerance_factor(self) -> f64 {
        match self {
            Self::OneA => 1.5,
            Self::TwoA => 1.,
            Self::ThreeA => 0.75,
            Self::OneB => 1.5 * 1.3,
            Self::TwoB => 1.3,
            Self::ThreeB => 0.75 * 1.3,
        }
    }
}

pub(super) struct ProductLimits {
    pub basic_pitch_mm: f64,
    pub lower_mm: f64,
    pub upper_mm: f64,
    pub tolerance_mm: f64,
    pub base_tolerance_mm: f64,
    pub allowance_mm: f64,
    pub engagement_mm: f64,
    pub external_major_max_mm: f64,
}

/// 旋合长度沿用模板按系列、等级选择的规则，直接匹配等级，消除 LEFT 文本比较问题。
fn engagement(diameter: f64, pitch: f64, series: Option<&str>, class: ThreadClass) -> f64 {
    match series {
        Some("UNC" | "UNF") => diameter,
        Some("4UN" | "6UN" | "8UN") => {
            if matches!(class, ThreadClass::OneA | ThreadClass::OneB) {
                9. * pitch
            } else {
                diameter
            }
        }
        _ => 9. * pitch,
    }
}

pub(super) fn calculate(
    diameter_in: f64,
    tpi: f64,
    series: Option<&str>,
    class: ThreadClass,
) -> Result<ProductLimits, String> {
    if !diameter_in.is_finite() || diameter_in <= 0. || !tpi.is_finite() || tpi <= 0. {
        return Err("公称直径和 TPI 须为有效正数".into());
    }
    let pitch_in = 1. / tpi;
    let engagement_in = engagement(diameter_in, pitch_in, series, class);
    let basic_pitch_in = diameter_in - 3. * 3_f64.sqrt() / 8. * pitch_in;
    let base_tolerance_in = 0.0015 * diameter_in.cbrt()
        + 0.0015 * engagement_in.sqrt()
        + 0.015 * pitch_in.powf(2. / 3.);
    let tolerance_in = base_tolerance_in * class.tolerance_factor();
    let allowance_in = if matches!(class, ThreadClass::OneA | ThreadClass::TwoA) {
        0.3 * base_tolerance_in
    } else {
        0.
    };
    let (lower_in, upper_in) = if class.external() {
        let upper = basic_pitch_in - allowance_in;
        (upper - tolerance_in, upper)
    } else {
        (basic_pitch_in, basic_pitch_in + tolerance_in)
    };
    if [
        basic_pitch_in,
        lower_in,
        upper_in,
        tolerance_in,
        engagement_in,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value <= 0.)
        || lower_in >= upper_in
        || upper_in >= diameter_in - allowance_in
    {
        return Err("当前直径、牙数和等级不能形成有效的工件中径公差带".into());
    }
    Ok(ProductLimits {
        basic_pitch_mm: basic_pitch_in * INCH,
        lower_mm: lower_in * INCH,
        upper_mm: upper_in * INCH,
        tolerance_mm: tolerance_in * INCH,
        base_tolerance_mm: base_tolerance_in * INCH,
        allowance_mm: allowance_in * INCH,
        engagement_mm: engagement_in * INCH,
        external_major_max_mm: (diameter_in - allowance_in) * INCH,
    })
}
