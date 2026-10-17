//! 压力量程、允差核验和静压差模型；高差以被检表高于标准器为正。
use crate::calculators::numerics::{finite, span};

#[derive(Clone, Copy)]
/// 上下限有限且 high > low 的量程；数值可使用任一一致的压力或信号单位。
pub(crate) struct Range {
    pub low: f64,
    pub high: f64,
    pub span: f64,
}
impl Range {
    pub(crate) fn new(low: f64, high: f64) -> Result<Self, String> {
        Ok(Self {
            low,
            high,
            span: span(low, high)?,
        })
    }
    pub(crate) fn covers(self, other: Self) -> bool {
        self.low <= other.low && self.high >= other.high
    }
}

pub(crate) struct Selection {
    pub dut_tolerance: f64,
    pub reference_tolerance: f64,
    pub ratio: f64,
    pub coverage: bool,
    pub meets: bool,
}
/// 准确度输入为百分数，例如 0.05 表示 0.05%；量程覆盖和允差比必须同时满足。
pub(crate) fn select_standard(
    dut: Range,
    reference: Range,
    dut_accuracy: f64,
    reference_accuracy: f64,
    required: f64,
) -> Result<Selection, String> {
    finite(&[dut_accuracy, reference_accuracy, required])?;
    if [dut_accuracy, reference_accuracy, required]
        .iter()
        .any(|x| *x <= 0.0)
    {
        return Err("准确度百分数和量传比要求必须大于 0".to_owned());
    }
    let dut_tolerance = dut.span * (dut_accuracy / 100.0);
    let reference_tolerance = reference.span * (reference_accuracy / 100.0);
    if dut_tolerance <= 0.0 || reference_tolerance <= 0.0 {
        return Err("允差低于双精度可表示范围".to_owned());
    }
    let ratio = dut_tolerance / reference_tolerance;
    finite(&[dut_tolerance, reference_tolerance, ratio])?;
    let coverage = reference.covers(dut);
    Ok(Selection {
        dut_tolerance,
        reference_tolerance,
        ratio,
        coverage,
        meets: coverage && ratio >= required,
    })
}

/// 返回 p标准−p被检（Pa），而移至被检高度的修正量取其相反数。
pub(crate) fn hydrostatic_difference(
    height: f64,
    density: f64,
    gravity: f64,
) -> Result<f64, String> {
    finite(&[height, density, gravity])?;
    if density <= 0.0 || gravity <= 0.0 {
        return Err("密度与重力加速度必须大于 0".to_owned());
    }
    let difference = density * gravity * height;
    finite(&[difference])?;
    Ok(difference)
}

/// 按 0 °C、101325 Pa 的密度折算当地气体密度，温度用 °C、压力用 Pa。
/// 表压读数还须扣除两高度间环境气柱；这是局部密度近似，不积分气体自重。
pub(crate) fn gas_difference(
    height: f64,
    normal_density: f64,
    gravity: f64,
    temperature: f64,
    pressure: f64,
    ambient: f64,
    gauge: bool,
) -> Result<f64, String> {
    finite(&[
        height,
        normal_density,
        gravity,
        temperature,
        pressure,
        ambient,
    ])?;
    let absolute = pressure + if gauge { ambient } else { 0.0 };
    if absolute <= 0.0 || ambient <= 0.0 || temperature <= -273.15 {
        return Err("绝对压力和环境压力须大于 0，温度须高于绝对零度".into());
    }
    let density = normal_density * (absolute / 101325.0) * 273.15 / (temperature + 273.15);
    let gas = hydrostatic_difference(height, density, gravity)?;
    if !gauge {
        return Ok(gas);
    }
    let h =
        crate::calculators::math::isa_altitude(ambient).ok_or("环境压力超出标准大气模型范围")?;
    let upper =
        crate::calculators::math::isa_pressure(h + height).ok_or("高度超出标准大气模型范围")?;
    Ok(gas - (ambient - upper))
}
