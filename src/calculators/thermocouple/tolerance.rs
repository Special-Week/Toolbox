//! IEC 60584-1 classes, checked against WIKA IN 00.23.
//! 等级范围独立于分度曲线范围；没有内置标准数据的型号返回空列表。
pub(crate) struct Tolerance {
    pub name: &'static str,
    pub min: f64,
    pub max: f64,
    pub formula: &'static str,
    pub delta: Option<f64>,
}
pub(crate) fn calculate(kind: &str, t: f64) -> Vec<Tolerance> {
    let definitions: &[(&str, f64, f64, f64, f64, &str)] = match kind {
        "K" | "N" => &[
            ("I 级", -40., 1000., 1.5, 0.004, "±max(1.5, 0.004|t|) °C"),
            ("II 级", -40., 1200., 2.5, 0.0075, "±max(2.5, 0.0075|t|) °C"),
            ("III 级", -200., 40., 2.5, 0.015, "±max(2.5, 0.015|t|) °C"),
        ],
        "E" => &[
            ("I 级", -40., 800., 1.5, 0.004, "±max(1.5, 0.004|t|) °C"),
            ("II 级", -40., 900., 2.5, 0.0075, "±max(2.5, 0.0075|t|) °C"),
            ("III 级", -200., 40., 2.5, 0.015, "±max(2.5, 0.015|t|) °C"),
        ],
        "J" => &[
            ("I 级", -40., 750., 1.5, 0.004, "±max(1.5, 0.004|t|) °C"),
            ("II 级", -40., 750., 2.5, 0.0075, "±max(2.5, 0.0075|t|) °C"),
        ],
        "T" => &[
            ("I 级", -40., 350., 0.5, 0.004, "±max(0.5, 0.004|t|) °C"),
            ("II 级", -40., 350., 1., 0.0075, "±max(1, 0.0075|t|) °C"),
            ("III 级", -200., 40., 1., 0.015, "±max(1, 0.015|t|) °C"),
        ],
        "B" => &[
            ("II 级", 600., 1700., 0., 0.0025, "±0.0025|t| °C"),
            ("III 级", 600., 1700., 0., 0.005, "±0.005|t| °C"),
        ],
        "S" | "R" => &[
            ("I 级", 0., 1600., 1., 0., "±[1 + 0.003max(t−1100, 0)] °C"),
            ("II 级", 0., 1600., 1.5, 0.0025, "±max(1.5, 0.0025|t|) °C"),
        ],
        _ => &[],
    };
    definitions
        .iter()
        .map(|&(name, min, max, a, b, formula)| Tolerance {
            name,
            min,
            max,
            formula,
            delta: if (min..=max).contains(&t) {
                Some(if (kind == "S" || kind == "R") && name == "I 级" {
                    1.0 + 0.003 * (t - 1100.0).max(0.0)
                } else {
                    a.max(b * t.abs())
                })
            } else {
                None
            },
        })
        .collect()
}
