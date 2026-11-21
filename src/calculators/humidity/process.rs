//! 将湿度模型的实际分支和结果整理为可复核的计算过程。
use super::website::{self, ResultSet};
use crate::calculators::{
    calibration::TemperatureUnit,
    common::{
        format_process_computed as c, format_process_number as n, temperature_conversion_note,
    },
};

fn saturation_steps(t: f64, label: &str, converted: bool) -> Result<Vec<String>, String> {
    let pressure = website::saturation(t)?;
    let shown_t = if converted { c(t) } else { n(t) };
    let formula = if t <= 0.0 {
        format!(
            "冰面（t ≤ 0 °C）：pₛ(t) = 611.2 × exp[22.46t / (272.62 + t)] = 611.2 × exp[22.46 × {} / (272.62 + {})] = {} Pa。",
            shown_t,
            shown_t,
            c(pressure)
        )
    } else if t < 1.0 {
        let coefficient = c(website::low_water_coefficient());
        format!(
            "水面（0 < t < 1 °C）：a₁ = 244.12 × ln[pₛ(1)/611.2] = {coefficient}；pₛ(t) = 611.2 × exp[a₁t / (243.12 + t)] = 611.2 × exp[{coefficient} × {} / (243.12 + {})] = {} Pa。",
            shown_t,
            shown_t,
            c(pressure)
        )
    } else {
        format!(
            "水面（t ≥ 1 °C）：T = t + 273.15 = {} K；pₛ(t) = 10^[(10.286T − 2148.4909) / (T − 35.85)] = 10^[(10.286 × {} − 2148.4909) / ({} − 35.85)] = {} Pa。",
            c(t + 273.15),
            c(t + 273.15),
            c(t + 273.15),
            c(pressure)
        )
    };
    Ok(vec![format!("{label}：t = {} °C。", shown_t), formula])
}

pub(crate) fn steps(
    t: f64,
    p: f64,
    known: usize,
    value: f64,
    unit: TemperatureUnit,
    r: &ResultSet,
) -> Result<Vec<String>, String> {
    let mut lines = vec![
        format!(
            "环境温度 t = {} °C = {} {}；绝对气压 p = {} Pa。",
            if unit.0 == 0 { n(t) } else { c(t) },
            n(unit.display(t)),
            unit.symbol(),
            c(p)
        ),
        temperature_conversion_note(unit.symbol()).to_owned(),
        "气压输入为 kPa，计算时乘 1000 转为 Pa；温度公式使用 °C 或 K。".to_owned(),
        String::new(),
    ];
    lines.extend(saturation_steps(t, "环境饱和水汽压", unit.0 != 0)?);
    lines.push(String::new());
    match known {
        1 => {
            lines.extend(saturation_steps(
                value,
                if value <= 0.0 {
                    "输入霜点的饱和水汽压"
                } else {
                    "输入露点的饱和水汽压"
                },
                unit.0 != 0,
            )?);
            lines.push(format!(
                "水汽分压 e = pₛ(输入露点或霜点) = {} Pa。",
                c(r.raw_vapor)
            ));
        }
        2 => {
            let wet_saturation = website::saturation(value)?;
            lines.extend(saturation_steps(
                value,
                "输入湿球温度的饱和水汽压",
                unit.0 != 0,
            )?);
            lines.push(format!("湿球系数 A = 0.000662；e = pₛ(t湿) − A × p × (t − t湿) = {} − 0.000662 × {} × ({} − {}) = {} Pa。", c(wet_saturation), c(p), if unit.0 == 0 { n(t) } else { c(t) }, if unit.0 == 0 { n(value) } else { c(value) }, c(r.raw_vapor)));
        }
        3 => {
            let q = value / 1000.0 * 28.959 / 18.0152;
            lines.push(format!(
                "输入混合比 m = {} g/kg；q = (m / 1000) × 28.959 / 18.0152 = {}。",
                n(value),
                c(q)
            ));
            lines.push(format!(
                "水汽分压 e = p × q / (1 + q) = {} × {} / (1 + {}) = {} Pa。",
                c(p),
                c(q),
                c(q),
                c(r.raw_vapor)
            ));
        }
        _ => lines.push(format!(
            "输入相对湿度 RH = {} %；水汽分压 e = pₛ(t) × RH / 100 = {} × {} / 100 = {} Pa。",
            n(value),
            c(r.saturated),
            n(value),
            c(r.raw_vapor)
        )),
    }
    lines.push(String::new());
    let raw_rh = r.raw_vapor * 100.0 / r.saturated;
    lines.push(format!(
        "相对湿度 RH = e × 100 / pₛ(t) = {} × 100 / {} = {} %。",
        c(r.raw_vapor),
        c(r.saturated),
        c(raw_rh)
    ));
    if r.raw_vapor != r.vapor {
        lines.push(format!(
            "计算得到的相对湿度因数值误差略超界，将水汽分压由 {} Pa 调整为 {} Pa，对应相对湿度 {} %；后续计算使用调整后的分压。",
            c(r.raw_vapor), c(r.vapor), c(r.rh)
        ));
    }
    lines.push(String::new());
    if let (Some(dew), Some(frost)) = (r.dew, r.frost) {
        let z = r.vapor.ln() - 611.2_f64.ln();
        lines.push(format!(
            "露点与霜点：z = ln(e) − ln(611.2) = ln({}) − ln(611.2) = {}（先取对数可避免极小分压下溢）。",
            c(r.vapor),
            c(z)
        ));
        let boundary = website::water_boundary();
        if r.vapor < boundary {
            let coefficient = if r.vapor <= 611.2 { 17.62 } else { website::low_water_coefficient() };
            lines.push(format!("e < pₛ(1) = {} Pa：水面露点 = 243.12z / (a − z)；a = {}，代入得 243.12 × {} / ({} − {}) = {} °C = {} {}。", c(boundary), c(coefficient), c(z), c(coefficient), c(z), c(dew), c(unit.display(dew)), unit.symbol()));
        } else {
            let log = r.vapor.log10();
            lines.push(format!("e ≥ pₛ(1) = {} Pa：log₁₀(e) = {}；水面露点 = [−2148.4909 + 35.85 log₁₀(e)] / [log₁₀(e) − 10.286] − 273.15 = [−2148.4909 + 35.85 × {}] / ({} − 10.286) − 273.15 = {} °C = {} {}。", c(boundary), c(log), c(log), c(log), c(dew), c(unit.display(dew)), unit.symbol()));
        }
        if frost <= 0.0 {
            lines.push(String::new());
            lines.push(format!(
                "冰面霜点 = 272.62z / (22.46 − z) = 272.62 × {} / (22.46 − {}) = {} °C = {} {}。",
                c(z),
                c(z),
                c(frost),
                c(unit.display(frost)),
                unit.symbol()
            ));
        } else {
            lines.push(format!(
                "冰面公式反算温度为 {} °C，高于 0 °C；霜点仅在反算温度不高于 0 °C 时显示。",
                c(frost)
            ));
        }
    } else {
        lines.push(format!(
            "水汽分压 {} Pa 非正：模型不输出有限露点或霜点。",
            c(r.vapor)
        ));
    }
    lines.push(String::new());
    if r.vapor == r.saturated {
        lines.push(format!("湿球温度：水汽分压等于饱和压，t湿 等于环境温度 = {} °C = {} {}。", c(r.wet), c(unit.display(r.wet)), unit.symbol()));
    } else {
        lines.push(format!("湿球温度：在 −100～{} °C 内对 pₛ(t湿) − 0.000662 × p × (t − t湿) = e 执行 100 次二分，并核对反算残差；得到 t湿 = {} °C = {} {}。", n(t), c(r.wet), c(unit.display(r.wet)), unit.symbol()));
    }
    lines.extend(saturation_steps(r.wet, "反算湿球温度的饱和水汽压", true)?);
    let wet_check = website::saturation(r.wet)? - 0.000662 * p * (t - r.wet);
    lines.push(format!(
        "代入湿球方程，水汽分压校验 = {} Pa（与输入分压末位可能有数值误差）。",
        c(wet_check)
    ));
    lines.push(String::new());
    lines.push(format!(
        "水汽体积分数 = e / p = {} / {} = {} L/L。",
        c(r.vapor),
        c(p),
        c(r.volume_fraction)
    ));
    lines.push(String::new());
    lines.push(format!(
        "干空气基准体积比 = e / (p − e) × 10⁶ = {} / ({} − {}) × 10⁶ = {} μL/L。",
        c(r.vapor),
        c(p),
        c(r.vapor),
        c(r.ppm)
    ));
    lines.push(String::new());
    lines.push(format!("绝对湿度 = e × 18.015 / [(t + 273.15) × 8.31451] = {} × 18.015 / ({} × 8.31451) = {} g/m³。", c(r.vapor), c(t + 273.15), c(r.absolute)));
    lines.push(String::new());
    lines.push(format!(
        "干空气基准质量比 = 体积比 × 18.0152 / 28.959 = {} × 18.0152 / 28.959 = {} μg/g。",
        c(r.ppm),
        c(r.mass)
    ));
    lines.push(String::new());
    lines.push(format!(
        "比湿 = 质量比 / (10⁶ + 质量比) × 1000 = {} / (10⁶ + {}) × 1000 = {} g/kg。",
        c(r.mass),
        c(r.mass),
        c(r.specific)
    ));
    lines.push(String::new());
    lines.push(format!(
        "混合比 = 质量比 / 1000 = {} / 1000 = {} g/kg。",
        c(r.mass),
        c(r.mixing)
    ));
    lines.push(String::new());
    lines.push(format!("标准状态（0 °C、101325 Pa）含水量 = 绝对湿度 × (t + 273.15) × 101325 / (273.15 × p) = {} × {} × 101325 / (273.15 × {}) = {} g/Nm³。", c(r.absolute), c(t + 273.15), c(p), c(r.normal)));
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_all_inputs_saturation_branches_and_output_values() {
        for (t, known, value, source) in [
            (20.0, 0, 50.0, "输入相对湿度"),
            (20.0, 1, 10.0, "输入露点的饱和水汽压"),
            (20.0, 1, -10.0, "输入霜点的饱和水汽压"),
            (20.0, 2, 15.0, "输入湿球温度的饱和水汽压"),
            (20.0, 3, 7.5, "输入混合比"),
            (-20.0, 0, 50.0, "冰面（t ≤ 0 °C）"),
            (0.0, 0, 50.0, "冰面（t ≤ 0 °C）"),
            (0.5, 0, 50.0, "水面（0 < t < 1 °C）"),
            (1.0, 0, 50.0, "水面（t ≥ 1 °C）"),
            (20.0, 0, 0.0, "模型不输出有限露点或霜点"),
        ] {
            let p = 101325.0;
            let result = website::calculate(t, p, known, value).unwrap();
            for unit in [TemperatureUnit(0), TemperatureUnit(1), TemperatureUnit(2)] {
                let text = steps(t, p, known, value, unit, &result)
                    .unwrap()
                    .join("\n")
                    .replace('\u{2063}', "");
                assert!(text.contains(source), "{t} {known}: {text}");
                for actual in [
                    result.saturated,
                    result.vapor,
                    result.rh,
                    result.wet,
                    result.volume_fraction,
                    result.ppm,
                    result.absolute,
                    result.mass,
                    result.specific,
                    result.mixing,
                    result.normal,
                ] {
                    assert!(text.contains(&n(actual)), "{t} {known}: {actual}");
                }
                if let Some(dew) = result.dew {
                    assert!(text.contains(&n(dew)));
                    assert!(text.contains(if result.vapor < website::water_boundary() {
                        "e < pₛ(1)"
                    } else {
                        "e ≥ pₛ(1)"
                    }));
                }
                assert!(text.contains("100 次二分"));
                assert!(text.contains("\n\n"), "不同结果之间应留空行");
            }
        }
    }

    #[test]
    fn frost_display_condition_is_explained() {
        for (t, rh, shown) in [(20.0, 50.0, false), (-20.0, 50.0, true)] {
            let result = website::calculate(t, 101325.0, 0, rh).unwrap();
            assert_eq!(result.frost.unwrap() <= 0.0, shown);
            let lines = steps(t, 101325.0, 0, rh, TemperatureUnit(0), &result)
                .unwrap()
                .join("\n");
            assert!(lines.contains(if shown {
                "冰面霜点 ="
            } else {
                "霜点仅在反算温度不高于 0 °C 时显示"
            }));
        }
        let zero = website::calculate(20.0, 101325.0, 1, 0.0).unwrap();
        assert_eq!(zero.frost, Some(0.0));
        assert!(
            steps(20.0, 101325.0, 1, 0.0, TemperatureUnit(0), &zero)
                .unwrap()
                .join("\n")
                .contains("冰面霜点 =")
        );
    }

    #[test]
    fn displayed_values_reconstruct_every_humidity_path_and_rounding_boundary() {
        let mut checked = 0;
        for t in [
            -80.0, -20.0, -0.000001, 0.0, 0.000001, 0.5, 0.999999, 1.0, 20.0, 70.0,
        ] {
            for (known, value) in [
                (0, -1e-11),
                (0, 0.0),
                (0, 25.0),
                (0, 50.0),
                (0, 99.999999995),
                (0, 100.0),
                (1, -30.0),
                (1, 0.0),
                (1, 0.5),
                (1, 1.0),
                (1, 10.0),
                (2, -30.0),
                (2, 0.0),
                (2, 0.5),
                (2, 1.0),
                (2, 10.0),
                (3, 0.0),
                (3, 1.0),
                (3, 7.5),
            ] {
                let pressure = 101325.0;
                let Ok(r) = website::calculate(t, pressure, known, value) else {
                    continue;
                };
                let sat = website::saturation(t).unwrap();
                let vapor = match known {
                    1 => website::saturation(value).unwrap(),
                    2 => website::saturation(value).unwrap() - 0.000662 * pressure * (t - value),
                    3 => {
                        let q = value / 1000.0 * 28.959 / 18.0152;
                        pressure * q / (1.0 + q)
                    }
                    _ => sat * value / 100.0,
                };
                assert_eq!(r.raw_vapor, vapor);
                let vapor = r.vapor;
                let volume = vapor / pressure;
                let ppm = vapor / (pressure - vapor) * 1e6;
                let mass = ppm * 18.0152 / 28.959;
                let absolute = vapor * 18.015 / ((t + 273.15) * 8.31451);
                assert_eq!(r.volume_fraction, volume);
                assert_eq!(r.ppm, ppm);
                assert_eq!(r.mass, mass);
                assert_eq!(r.specific, mass / (1e6 + mass) * 1000.0);
                assert_eq!(r.mixing, mass / 1000.0);
                assert_eq!(r.absolute, absolute);
                assert_eq!(
                    r.normal,
                    absolute * (t + 273.15) * 101325.0 / (273.15 * pressure)
                );
                let text = steps(t, pressure, known, value, TemperatureUnit(0), &r)
                    .unwrap()
                    .join("\n")
                    .replace('\u{2063}', "");
                let raw_rh = r.raw_vapor * 100.0 / sat;
                assert!(text.contains(&format!(
                    "相对湿度 RH = e × 100 / pₛ(t) = {} × 100 / {} = {} %。",
                    n(r.raw_vapor),
                    n(sat),
                    n(raw_rh)
                )));
                assert!(text.contains(&format!(
                    "水汽体积分数 = e / p = {} / {} = {} L/L。",
                    n(vapor),
                    n(pressure),
                    n(volume)
                )));
                assert_eq!(
                    text.contains("后续计算使用调整后的分压"),
                    r.raw_vapor != r.vapor
                );
                let check = website::saturation(r.wet).unwrap() - 0.000662 * pressure * (t - r.wet);
                assert!((check - vapor).abs() <= 1e-9 * vapor.abs().max(1.0));
                checked += 1;
            }
        }
        assert!(checked > 80);
    }

    #[test]
    fn humidity_boundaries_are_consistent() {
        assert!(website::calculate(20.0, 101325.0, 0, -1e-11).is_err());
        assert!(website::calculate(20.0, 101325.0, 0, 100.0 + 1e-11).is_err());
        let positive = website::calculate(20.0, 101325.0, 0, 1e-11).unwrap();
        assert!(positive.rh > 0.0 && positive.vapor > 0.0 && positive.absolute > 0.0);
        let tiny = website::calculate(20.0, 101325.0, 0, f64::from_bits(1)).unwrap();
        assert!(tiny.vapor > 0.0 && tiny.rh > 0.0);
        assert!(tiny.dew.unwrap().is_finite() && tiny.frost.unwrap().is_finite());
        let value = 20.0 + 1e-11;
        let result = website::calculate(20.0, 101325.0, 1, value).unwrap();
        assert!(result.raw_vapor > result.vapor);
        assert_eq!(result.vapor, result.saturated);
        assert_eq!(result.rh, 100.0);
        let text = steps(20.0, 101325.0, 1, value, TemperatureUnit(0), &result)
            .unwrap()
            .join("\n")
            .replace('\u{2063}', "");
        assert!(text.contains("后续计算使用调整后的分压"));
        assert!(text.contains(&n(result.raw_vapor)));
        assert!(text.contains(&n(result.vapor)));

        // 寻找湿球方程零点附近最靠近的负分压，检查边界归整后的所有派生量。
        let (mut low, mut high) = (-100.0_f64, 20.0_f64);
        for _ in 0..100 {
            let mid = low + (high - low) / 2.0;
            let vapor = website::saturation(mid).unwrap() - 0.000662 * 101325.0 * (20.0 - mid);
            if vapor < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        }
        let dry = website::calculate(20.0, 101325.0, 2, low).unwrap();
        assert!(dry.raw_vapor < 0.0);
        assert_eq!(dry.vapor, 0.0);
        assert_eq!(dry.rh, 0.0);
        assert_eq!(dry.absolute, 0.0);
        assert!(dry.dew.is_none() && dry.frost.is_none());
        let text = steps(20.0, 101325.0, 2, low, TemperatureUnit(0), &dry)
            .unwrap()
            .join("\n");
        assert!(text.contains("后续计算使用调整后的分压"));
    }
}
