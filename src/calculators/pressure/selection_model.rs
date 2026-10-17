//! 标准器选型的纯模型；量程预先统一单位，准确度统一使用百分数。
use super::model::{Range, Selection};
use crate::calculators::numerics::finite;
pub(crate) fn verify(
    dut: Range,
    reference: Range,
    da: f64,
    ra: f64,
    required: f64,
    piston: bool,
) -> Result<Selection, String> {
    let mut result = super::model::select_standard(dut, reference, da, ra, required)?;
    if piston {
        // 活塞标准器在低于其跨度 10% 时，以该 10% 下限计算允差。
        result.reference_tolerance = ra / 100.0 * dut.span.max(reference.span * 0.1);
        result.ratio = result.dut_tolerance / result.reference_tolerance;
        finite(&[result.reference_tolerance, result.ratio])?;
        result.meets = result.coverage && result.ratio >= required;
    }
    Ok(result)
}
pub(crate) struct ElectricalSelection {
    pub pressure: Selection,
    pub expanded: f64,
    pub allowance: f64,
    pub meets: bool,
}
/// 压力允差先折算成电信号，两分量按均匀分布合成，并使用 k=2 扩展。
/// reading 是读数百分数，fixed 与 output 使用相同信号单位；限值采用严格小于。
pub(crate) fn transmitter(
    dut: Range,
    reference: Range,
    output: Range,
    da: f64,
    ra: f64,
    reading: f64,
    fixed: f64,
    required: f64,
) -> Result<ElectricalSelection, String> {
    finite(&[reading, fixed])?;
    if reading < 0.0 || fixed < 0.0 {
        return Err("读数准确度和固定误差不能为负数".into());
    }
    let pressure = verify(dut, reference, da, ra, required, false)?;
    let pressure_contribution = (pressure.reference_tolerance / dut.span) * output.span;
    let electric = reading / 100.0 * output.high.abs() + fixed;
    let expanded = 2.0 * pressure_contribution.hypot(electric) / 3.0_f64.sqrt();
    let allowance = output.span * da / 100.0 / required;
    finite(&[expanded, allowance])?;
    let meets = pressure.meets && expanded < allowance;
    Ok(ElectricalSelection {
        pressure,
        expanded,
        allowance,
        meets,
    })
}
/// 常用量程均以 MPa 保存；页面转换到用户选择的单位后再做比较。
pub(crate) const METRIC_RANGES: &[(f64, f64)] = &[
    (0., 60.),
    (0., 40.),
    (0., 25.),
    (0., 16.),
    (0., 10.),
    (0., 6.),
    (0., 4.),
    (0., 2.5),
    (0., 1.6),
    (0., 1.),
    (0., 0.6),
    (0., 0.4),
    (0., 0.25),
    (0., 0.16),
    (0., 0.1),
    (0., 0.06),
    (-0.1, 0.),
];
