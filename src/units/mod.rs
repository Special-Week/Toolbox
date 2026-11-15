//! 22 类单位的显式线性/仿射换算；温度零点与其他物理量的纯比例换算分开处理。
mod data;
pub(crate) mod notes;

use crate::catalog::Dimension;
pub(crate) use data::units_for;

#[derive(Clone, Copy)]
pub(crate) struct UnitDef {
    pub(crate) reference_id: u32,
    pub(crate) name: &'static str,
    pub(crate) code: &'static str,
    // base = value * factor + offset；温度基准为开尔文，
    // 浓度基准为 ppm，遵循页面声明的等密度约定。
    factor: f64,
    offset: f64,
}

impl UnitDef {
    pub(crate) fn conversion_coefficients(&self) -> (f64, f64) {
        (self.factor, self.offset)
    }
}

/// source/target 是该物理量目录内的索引；禁止跨类别使用索引。
/// 浓度按页面声明的等密度约定折算到 ppm，不是任意介质的通用质量/体积换算。
pub(crate) fn convert_unit(
    dimension: Dimension,
    value: f64,
    source: usize,
    target: usize,
) -> Result<f64, String> {
    let units = units_for(dimension);
    let from = units.get(source).ok_or("输入单位不存在")?;
    let to = units.get(target).ok_or("输出单位不存在")?;
    if !value.is_finite() {
        return Err("输入必须是有限数字".to_owned());
    }
    let temperature = matches!(dimension, Dimension::Temperature);
    if temperature {
        let minimum = match from.reference_id {
            1001 => -273.15,
            1000 | 1003 => 0.0,
            1002 => -459.67,
            999 => -218.52,
            _ => unreachable!("temperature unit lacks an absolute-zero definition"),
        };
        if value < minimum {
            return Err("绝对温度不能低于 0 K".to_owned());
        }
    }
    // 线性单位直接按系数比换算，避免 SI 中间值溢出或下溢，
    // 即使最终结果本身可以表示也不能忽略这一问题。
    let result = if source == target {
        value
    } else if temperature {
        // 仅对已验证的绝对零度输入修正相消误差。
        let kelvin = value.mul_add(from.factor, from.offset).max(0.0);
        (kelvin - to.offset) / to.factor
    } else {
        value * (from.factor / to.factor)
    };
    if !result.is_finite() {
        return Err("单位换算结果超出双精度范围".to_owned());
    }
    if !temperature && value != 0.0 && result == 0.0 {
        return Err("单位换算结果过小，超出双精度可表示范围".to_owned());
    }
    Ok(result)
}

#[cfg(all(test, feature = "desktop-ui"))]
mod tests {
    use super::*;
    use crate::catalog::{TOOLS, ToolKind};

    #[test]
    fn displayed_conversion_formula_matches_every_unit_pair() {
        let mut dimensions = 0;
        for dimension in TOOLS.iter().filter_map(|tool| match tool.kind {
            ToolKind::Unit(dimension) => Some(dimension),
            _ => None,
        }) {
            dimensions += 1;
            let units = units_for(dimension);
            for (source, from) in units.iter().enumerate() {
                for (target, to) in units.iter().enumerate() {
                    for value in [0.0_f64, 1.0, 100.0] {
                        let actual = convert_unit(dimension, value, source, target).unwrap();
                        let expected = if source == target {
                            value
                        } else if matches!(dimension, Dimension::Temperature) {
                            (value.mul_add(from.factor, from.offset).max(0.0) - to.offset)
                                / to.factor
                        } else {
                            value * (from.factor / to.factor)
                        };
                        assert_eq!(actual, expected, "{} → {}, {value}", from.code, to.code);
                    }
                }
            }
        }
        assert_eq!(dimensions, 22);
        let celsius = units_for(Dimension::Temperature)
            .iter()
            .position(|unit| unit.code == "°C")
            .unwrap();
        assert!(convert_unit(Dimension::Temperature, -273.16, celsius, celsius).is_err());
    }

    #[test]
    fn temperature_note_matches_absolute_zero_for_every_scale() {
        let units = units_for(Dimension::Temperature);
        let note =
            crate::calculators::explanations::formula(ToolKind::Unit(Dimension::Temperature));
        assert!(note.contains("绝对零度"));
        for (source, unit) in units.iter().enumerate() {
            let zero = match unit.reference_id {
                1001 => -273.15,
                1000 | 1003 => 0.0,
                1002 => -459.67,
                999 => -218.52,
                _ => panic!("缺少绝对零度：{}", unit.code),
            };
            for target in 0..units.len() {
                assert!(
                    convert_unit(Dimension::Temperature, zero, source, target).is_ok(),
                    "{}",
                    unit.code
                );
                assert!(
                    convert_unit(Dimension::Temperature, zero - 0.000001, source, target).is_err(),
                    "{}",
                    unit.code
                );
            }
        }
    }
}
