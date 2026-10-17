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
    // base = value * factor + offset; kelvin for temperature, ppm for
    // concentration under the declared equal-density website convention.
    factor: f64,
    offset: f64,
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
    // Use a direct ratio for linear units: an unnecessary SI intermediate can
    // overflow or underflow even when the final result is representable.
    let result = if source == target {
        value
    } else if temperature {
        // Clamp only arithmetic cancellation at a validated absolute-zero input.
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
