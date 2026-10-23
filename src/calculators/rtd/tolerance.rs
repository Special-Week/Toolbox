//! 温度等级允差；value=None 表示超出适用范围，不用边界值冒充当前有效允差。
//! 此处返回摄氏温差，电阻界限由页面按当前曲线的 R(t±Δt) 计算。
use super::model::ToleranceKind;

pub(crate) struct Tolerance {
    pub name: &'static str,
    pub formula: &'static str,
    pub min: f64,
    pub max: f64,
    pub value: Option<f64>,
}

pub(crate) fn tolerances(kind: ToleranceKind, t: f64, film: bool) -> Vec<Tolerance> {
    let definitions: &[(&str, &str, f64, f64, f64, f64)] = match kind {
        ToleranceKind::Platinum | ToleranceKind::PlatinumReference if film => &[
            ("AA 级", "±(0.10 + 0.0017|t|) °C", 0.0, 150.0, 0.10, 0.0017),
            ("A 级", "±(0.15 + 0.002|t|) °C", -30.0, 300.0, 0.15, 0.002),
            ("B 级", "±(0.30 + 0.005|t|) °C", -50.0, 500.0, 0.30, 0.005),
            ("C 级", "±(0.60 + 0.01|t|) °C", -50.0, 600.0, 0.60, 0.01),
        ],
        ToleranceKind::Platinum | ToleranceKind::PlatinumReference => &[
            (
                "AA 级",
                "±(0.10 + 0.0017|t|) °C",
                -50.0,
                250.0,
                0.10,
                0.0017,
            ),
            ("A 级", "±(0.15 + 0.002|t|) °C", -100.0, 450.0, 0.15, 0.002),
            ("B 级", "±(0.30 + 0.005|t|) °C", -196.0, 600.0, 0.30, 0.005),
            ("C 级", "±(0.60 + 0.01|t|) °C", -196.0, 600.0, 0.60, 0.01),
        ],
        ToleranceKind::Copper => &[(
            "铜热电阻允差",
            "±(0.30 + 0.006|t|) °C",
            -50.0,
            150.0,
            0.30,
            0.006,
        )],
        ToleranceKind::None => &[],
    };
    definitions
        .iter()
        .map(|&(name, formula, min, max, constant, factor)| Tolerance {
            name,
            formula,
            min,
            max,
            value: if t.is_finite() && (min..=max).contains(&t) {
                Some(constant + factor * t.abs())
            } else {
                None
            },
        })
        .collect()
}
