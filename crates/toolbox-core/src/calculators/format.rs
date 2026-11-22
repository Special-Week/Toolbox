//! 桌面与网页共用的数值解析和普通小数格式化。
/// 输入阶段拒绝 NaN 和无穷大，避免非有限值进入计算或坐标映射。
pub fn parse_number(text: &str) -> Result<f64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("请填写数值".to_owned());
    }
    let value = text
        .parse::<f64>()
        .map_err(|_| "请输入有效数字".to_owned())?;
    if !value.is_finite() {
        return Err("数值必须为有限数字".to_owned());
    }
    Ok(value)
}

/// 通用结果使用普通小数；极小值按有效数字展开，不因固定小数位被显示成零。
/// 该函数只控制显示精度，文件导出直接写入未经过显示修约的 f64。
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.abs() >= 1.0e10 || value.abs() < 1.0e-4 {
        // 保留原先的 11 位有效数字；显示时将修约结果
        // 展开为普通小数，包括极小系数。
        let rounded = format!("{value:.10e}").parse::<f64>().unwrap_or(value);
        // 接近 f64::MAX 的值修约时可能溢出，此时保留原来的有限值。
        return if rounded.is_finite() { rounded } else { value }.to_string();
    }
    format!("{value:.8}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

pub fn format_unit(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value == 0.0 {
        return "0.0000000".to_owned();
    }
    if value.abs() >= 1.0e9 || (value != 0.0 && value.abs() < 1.0e-4) {
        return format_number(value);
    }
    format!("{value:.7}")
}
