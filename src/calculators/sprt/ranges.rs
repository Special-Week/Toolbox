//! 页面低温分区；换算区间与确定偏差系数所需的校准固定点分别保存。
pub(crate) struct LowRange {
    pub label: &'static str,
    pub minimum: f64,
    pub coefficients: Option<[&'static str; 2]>,
    pub fixed_points: &'static [f64],
}

pub(crate) fn high_fixed_points(range: usize) -> &'static [f64] {
    match range {
        0 | 1 => &[231.928, 419.527, 660.323],
        2 => &[231.928, 419.527],
        3 => &[156.599, 231.928],
        4 => &[156.599],
        5 => &[29.765],
        _ => &[],
    }
}

pub(crate) const LOW_RANGES: [LowRange; 3] = [
    LowRange {
        label: "a4, b4（−189.344～0 °C）",
        minimum: -189.344,
        coefficients: Some(["a4", "b4"]),
        fixed_points: &[-189.344, -38.834],
    },
    LowRange {
        label: "a5, b5（−38.834～0 °C）",
        minimum: -38.834,
        coefficients: Some(["a5", "b5"]),
        // 镓点参与 a5/b5 拟合，但页面低温换算上限仍为 0 °C。
        fixed_points: &[-38.834, 29.765],
    },
    LowRange {
        label: "不使用低温分区",
        minimum: 0.0,
        coefficients: None,
        fixed_points: &[],
    },
];
