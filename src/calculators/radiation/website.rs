//! 有效波长近似模型，c₂=14388 μm·K；温度输入为 °C，波长为 μm。
//! 本层保留计算精度，由页面在组合温度前按 0.01 °C 修约。
use crate::calculators::numerics::finite;
pub(crate) fn wavelength(response: usize, lambda: f64, t: f64) -> f64 {
    match response {
        1 => 9.6 + 221.0 / (t + 273.15),
        2 => 8.7 + 525.0 / (t + 273.15),
        _ => lambda,
    }
}
/// 返回真实温度减去该模型求得的温度；背景超过 200 °C 时明确拒绝近似。
pub(crate) fn error(emissivity: f64, lambda: f64, t: f64, background: f64) -> Result<f64, String> {
    finite(&[emissivity, lambda, t, background])?;
    if emissivity <= 0.0
        || emissivity > 1.0
        || lambda <= 0.0
        || t <= -273.15
        || background <= -273.15
    {
        return Err("发射率须满足 0 < ε ≤ 1，波长须大于 0，温度须高于绝对零度".into());
    }
    if background > 200.0 {
        return Err("有效波长法仅支持背景温度不高于 200 °C；请改选 Planck 辐射平衡模型".into());
    }
    if emissivity == 1.0 {
        return Ok(0.0);
    }
    let c = 14388.0 / lambda;
    let occupation = 1.0 / (c / (t + 273.15)).exp_m1()
        - (1.0 - emissivity) / (c / (background + 273.15)).exp_m1();
    if occupation <= 0.0 {
        return Err("当前参数无实数解，请检查背景温度和发射率".into());
    }
    let brightness = c / (emissivity / occupation).ln_1p();
    let delta = t + 273.15 - brightness;
    finite(&[delta])?;
    Ok(delta)
}
pub(crate) fn correction(
    instrument: f64,
    source: f64,
    lambda: f64,
    t: f64,
    bg: f64,
) -> Result<f64, String> {
    Ok(error(instrument, lambda, t, bg)? - error(source, lambda, t, bg)?)
}
