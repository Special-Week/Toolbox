//! 将选型模型的实际结果整理为可复核的计算步骤。
#![allow(clippy::items_after_test_module)]
use super::{
    model::{Range, Selection},
    selection_model::ElectricalSelection,
};
use crate::calculators::common::{format_process_computed, format_process_number};

pub fn atmosphere_layer(
    base: f64,
    end: f64,
    temperature: f64,
    lapse: f64,
    start: f64,
    result: f64,
) -> [String; 2] {
    let n = format_process_number;
    let c = format_process_computed;
    let values = format!(
        "分层：h₁ = {} m，h₂ = {} m，分子尺度温度 T₁ = {} K，L = {} K/m，p₁ = {} Pa。",
        n(base),
        n(end),
        n(temperature),
        n(lapse),
        c(start)
    );
    let formula = if lapse == 0.0 {
        format!(
            "等温层：p₂ = p₁ × exp[−g₀(h₂−h₁)/(R T₁)] = {} × exp[−9.80665 × ({}−{}) / (287.05287 × {})] = {} Pa。",
            c(start),
            n(end),
            n(base),
            n(temperature),
            c(result)
        )
    } else {
        format!(
            "变温层：p₂ = p₁ × [(T₁ + L(h₂−h₁))/T₁]^(−g₀/(R L)) = {} × [({} + {} × ({}−{})) / {}]^(−9.80665 / (287.05287 × {})) = {} Pa。",
            c(start),
            n(temperature),
            n(lapse),
            n(end),
            n(base),
            n(temperature),
            n(lapse),
            c(result)
        )
    };
    [values, formula]
}

#[allow(clippy::too_many_arguments)]
pub fn selection(
    dut: Range,
    reference: Range,
    da: f64,
    ra: f64,
    required: f64,
    piston: bool,
    result: &Selection,
    unit: &str,
) -> Vec<String> {
    let n = format_process_number;
    let c = format_process_computed;
    let coverage = if result.coverage {
        "覆盖"
    } else {
        "未覆盖"
    };
    let tolerance = if piston {
        format!(
            "标准器允差 = {}% × max({}, {} × 10%) = {} {unit}（活塞压力计）",
            n(ra),
            c(dut.span),
            c(reference.span),
            c(result.reference_tolerance)
        )
    } else {
        format!(
            "标准器允差 = {} × {}% = {} {unit}",
            c(reference.span),
            n(ra),
            c(result.reference_tolerance)
        )
    };
    vec![
        format!(
            "被检范围 {}～{} {unit}；被检跨度 = {} − {} = {} {unit}。",
            n(dut.low),
            n(dut.high),
            n(dut.high),
            n(dut.low),
            c(dut.span)
        ),
        format!(
            "标准器范围 {}～{} {unit}；标准器跨度 = {} − {} = {} {unit}：{coverage}。",
            n(reference.low),
            n(reference.high),
            n(reference.high),
            n(reference.low),
            c(reference.span)
        ),
        format!(
            "被检允差 = {} × {}% = {} {unit}",
            c(dut.span),
            n(da),
            c(result.dut_tolerance)
        ),
        tolerance,
        format!(
            "量传比 = 被检允差 / 标准器允差 = {} / {} = {}；要求 ≥ {}。{}",
            c(result.dut_tolerance),
            c(result.reference_tolerance),
            c(result.ratio),
            n(required),
            if result.meets && result.ratio < required {
                "计算值与要求的差落在浮点舍入误差内，按相等判定。"
            } else {
                ""
            }
        ),
        format!(
            "通道判定：{}。",
            if result.meets {
                "量程覆盖且量传比满足要求"
            } else {
                "量程未覆盖或量传比未达到要求"
            }
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculators::pressure::selection_model;

    #[test]
    fn explanations_use_the_actual_selection_and_electrical_results() {
        let dut = Range::new(0.0, 1.0).unwrap();
        let reference = Range::new(0.0, 1.6).unwrap();
        let result = selection_model::verify(dut, reference, 0.4, 0.05, 3.0, false).unwrap();
        let lines = selection(dut, reference, 0.4, 0.05, 3.0, false, &result, "MPa")
            .join("\n")
            .replace('\u{2063}', "");
        assert!(lines.contains("被检允差 = 1 × 0.4% = 0.004 MPa"));
        assert!(lines.contains("标准器允差 = 1.6 × 0.05% = 0.0008 MPa"));
        assert!(lines.contains("量传比 = 被检允差 / 标准器允差 = 0.004 / 0.0008 = 5"));

        let short = Range::new(0.0, 0.02).unwrap();
        let piston = selection_model::verify(short, reference, 0.4, 0.05, 3.0, true).unwrap();
        assert!(
            selection(short, reference, 0.4, 0.05, 3.0, true, &piston, "MPa")
                .join("\n")
                .replace('\u{2063}', "")
                .contains(&format!(
                    "max(0.02, 1.6 × 10%) = {} MPa",
                    format_process_number(piston.reference_tolerance)
                ))
        );

        let signal = Range::new(4.0, 20.0).unwrap();
        let electric =
            selection_model::transmitter(dut, reference, signal, 0.1, 0.01, 0.02, 0.001, 3.0)
                .unwrap();
        let lines = electrical(dut, signal, 0.1, 0.02, 0.001, 3.0, &electric, "mA")
            .join("\n")
            .replace('\u{2063}', "");
        assert!(lines.contains(&format_process_number(electric.pressure_contribution)));
        assert!(lines.contains(&format_process_number(electric.electric_contribution)));
        assert!(lines.contains(&format_process_number(electric.expanded)));
        assert!(lines.contains(&format_process_number(electric.allowance)));
        let signal = Range::new(-20.0, 4.0).unwrap();
        let electric =
            selection_model::transmitter(dut, dut, signal, 1.0, 0.01, 1.0, 0.0, 3.0).unwrap();
        let lines = electrical(dut, signal, 1.0, 1.0, 0.0, 3.0, &electric, "mA")
            .join("\n")
            .replace('\u{2063}', "");
        assert!(lines.contains("max(|-20|, |4|)"));
        assert!(lines.contains("= 0.2 mA"));
        assert!(lines.contains("不符合"));
    }

    #[test]
    fn selection_details_match_coverage_ratio_and_both_piston_regions() {
        for (dut, reference) in [
            (
                Range::new(0.0, 0.02).unwrap(),
                Range::new(0.0, 1.6).unwrap(),
            ),
            (Range::new(0.0, 1.0).unwrap(), Range::new(0.0, 1.6).unwrap()),
            (
                Range::new(-1.0, 1.0).unwrap(),
                Range::new(0.0, 1.6).unwrap(),
            ),
        ] {
            for piston in [false, true] {
                for required in [1.0, 3.0, 20.0] {
                    let result =
                        selection_model::verify(dut, reference, 0.4, 0.05, required, piston)
                            .unwrap();
                    let lines =
                        selection(dut, reference, 0.4, 0.05, required, piston, &result, "MPa")
                            .join("\n")
                            .replace('\u{2063}', "");
                    assert!(lines.contains(&format!(
                        "= {} MPa",
                        format_process_number(result.dut_tolerance)
                    )));
                    assert!(lines.contains(&format!(
                        "= {} MPa",
                        format_process_number(result.reference_tolerance)
                    )));
                    assert!(lines.contains(&format!(
                        "= {}；要求 ≥ {}",
                        format_process_number(result.ratio),
                        format_process_number(required)
                    )));
                    assert!(lines.contains(if result.coverage {
                        "：覆盖。"
                    } else {
                        "：未覆盖。"
                    }));
                    assert!(lines.contains(if result.meets {
                        "量程覆盖且量传比满足要求"
                    } else {
                        "量程未覆盖或量传比未达到要求"
                    }));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn electrical(
    dut: Range,
    signal: Range,
    da: f64,
    reading: f64,
    fixed: f64,
    required: f64,
    result: &ElectricalSelection,
    unit: &str,
) -> Vec<String> {
    let n = format_process_number;
    let c = format_process_computed;
    vec![
        format!(
            "输出范围 {}～{} {unit}，跨度 {} {unit}。",
            n(signal.low),
            n(signal.high),
            c(signal.span)
        ),
        format!(
            "压力分量 = 标准压力允差 / 被检压力跨度 × 输出跨度 = {} / {} × {} = {} {unit}",
            c(result.pressure.reference_tolerance),
            c(dut.span),
            c(signal.span),
            c(result.pressure_contribution)
        ),
        format!(
            "电测分量 = 读数准确度 × max(|输出下限|, |输出上限|) + 固定误差 = {}% × max(|{}|, |{}|) + {} = {} {unit}",
            n(reading),
            n(signal.low),
            n(signal.high),
            n(fixed),
            c(result.electric_contribution)
        ),
        format!(
            "按均匀分布合成，k=2：U = 2 × √(压力分量² + 电测分量²) / √3 = {} {unit}",
            c(result.expanded)
        ),
        format!(
            "允许限值 = 输出跨度 × 被检准确度 / 量传比 = {} × {}% / {} = {} {unit}",
            c(signal.span),
            n(da),
            n(required),
            c(result.allowance)
        ),
        format!(
            "合成结果须严格小于允许限值，且压力通道满足要求：{}。",
            if result.meets { "符合" } else { "不符合" }
        ),
    ]
}
