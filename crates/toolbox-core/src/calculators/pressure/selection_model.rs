//! 标准器选型的纯模型；量程预先统一单位，准确度统一使用百分数。
use super::model::{Range, Selection, meets_ratio};
use crate::calculators::numerics::{binary_parts, finite, scale_binary};
pub fn verify(
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
        result.meets = result.coverage && meets_ratio(result.ratio, required);
    }
    Ok(result)
}
pub struct ElectricalSelection {
    pub pressure: Selection,
    pub pressure_contribution: f64,
    pub electric_contribution: f64,
    pub expanded: f64,
    pub allowance: f64,
    pub meets: bool,
}

fn product_ratio(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let (a, ea) = binary_parts(a);
    let (b, eb) = binary_parts(b);
    let (c, ec) = binary_parts(c);
    let (d, ed) = binary_parts(d);
    scale_binary((a * b) / (c * d), ea + eb - ec - ed)
}
/// 压力允差先折算成电信号，两分量按均匀分布合成，并使用 k=2 扩展。
/// reading 是读数百分数，fixed 与 output 使用相同信号单位；限值采用严格小于。
#[allow(clippy::too_many_arguments)]
pub fn transmitter(
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
    let pressure_per_unit = pressure.reference_tolerance / dut.span;
    let pressure_contribution = if pressure_per_unit.is_normal() {
        pressure_per_unit * output.span
    } else {
        product_ratio(pressure.reference_tolerance, output.span, dut.span, 1.0)
    };
    let peak = output.low.abs().max(output.high.abs());
    let reading_fraction = reading / 100.0;
    let electric = fixed + if reading == 0.0 || reading_fraction.is_normal() {
        reading_fraction * peak
    } else {
        product_ratio(reading, peak, 100.0, 1.0)
    };
    let combined = pressure_contribution.hypot(electric);
    let doubled = 2.0 * combined;
    let expanded = if doubled.is_finite() { doubled / 3.0_f64.sqrt() } else { combined * (2.0 / 3.0_f64.sqrt()) };
    let unscaled_allowance = output.span * da;
    let allowance = if unscaled_allowance.is_normal() && (unscaled_allowance / 100.0).is_normal() {
        unscaled_allowance / 100.0 / required
    } else {
        product_ratio(output.span, da, 100.0, required)
    };
    finite(&[expanded, allowance])?;
    let meets = pressure.meets && expanded < allowance;
    Ok(ElectricalSelection {
        pressure,
        pressure_contribution,
        electric_contribution: electric,
        expanded,
        allowance,
        meets,
    })
}
/// 常用量程均以 MPa 保存；页面转换到用户选择的单位后再做比较。
pub const METRIC_RANGES: &[(f64, f64)] = &[
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn electrical_reading_error_uses_largest_absolute_endpoint() {
        let pressure = Range::new(0.0, 1.0).unwrap();
        for (low, high, expected) in [
            (-20.0, 4.0, 0.2),
            (-20.0, -4.0, 0.2),
            (-4.0, 20.0, 0.2),
            (4.0, 20.0, 0.2),
        ] {
            let result = transmitter(
                pressure,
                pressure,
                Range::new(low, high).unwrap(),
                1.0,
                0.01,
                1.0,
                0.0,
                3.0,
            )
            .unwrap();
            assert!((result.electric_contribution - expected).abs() < 1e-14);
            assert!(!result.meets);
            assert!(
                (result.expanded
                    - 2.0 * result.pressure_contribution.hypot(expected) / 3.0_f64.sqrt())
                .abs()
                    < 1e-14
            );
        }
    }
}
