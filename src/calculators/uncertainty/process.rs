//! 将分量树与同一次模型计算结果整理为计算过程。
use super::model::{self, Component, Detail, Kind};
use crate::calculators::{
    common::{format_process_computed as c, format_process_number as n, format_process_rounded},
    error_process,
    numerics::statistics,
    range_method,
};

pub(crate) fn component(input: &Component, detail: &Detail, parent: &str, lines: &mut Vec<String>) {
    let path = if parent.is_empty() {
        input.name.clone()
    } else {
        format!("{parent} / {}", input.name)
    };
    lines.push(String::new());
    lines.push(format!("分量：{path}。"));
    match &input.kind {
        Kind::TypeA {
            values,
            range_method,
            average_count,
        } => {
            if let Ok(stats) = statistics(values) {
                lines.extend(error_process::samples(values, &stats, false));
                if *range_method {
                    if let Ok(d2) = range_method::coefficient(values.len()) {
                        lines.push(format!("极差法：R = {} − {} = {}；d₂ = {}，u = R/(d₂√m) = {} / ({} × √{}) = {}，m 为测量值个数。", n(stats.max), n(stats.min), c(stats.max - stats.min), n(d2), c(stats.max - stats.min), n(d2), values.len(), c(detail.raw_standard)));
                    }
                } else {
                    let count = average_count.unwrap_or(1);
                    lines.push(format!(
                        "贝塞尔法：取测量次数 n = {}{}（此 n 可不同于上方估计 s 的测量值个数）；u = s/√n = {} / √{} = {}。",
                        count,
                        if average_count.is_none() {
                            "（留空默认 1 次）"
                        } else {
                            ""
                        },
                        c(stats.sd),
                        count,
                        c(detail.raw_standard)
                    ));
                }
            }
        }
        Kind::TypeB {
            half_width,
            distribution,
        } => {
            let name = match distribution {
                1 => "三角分布",
                2 => "正态分布 k=2",
                3 => "正态分布 k=3",
                4 => "两点分布",
                5 => "反正弦分布",
                _ => "均匀分布",
            };
            lines.push(format!(
                "B 类{}：{} = {}；除数 d = {}，u = 输入值/d = {} / {} = {}。",
                name,
                if matches!(distribution, 2 | 3) {
                    "证书扩展不确定度"
                } else {
                    "区间半宽"
                },
                n(*half_width),
                n(model::divisor(*distribution)),
                n(*half_width),
                n(model::divisor(*distribution)),
                c(detail.raw_standard)
            ));
        }
        Kind::Combined { children, maximum } => {
            lines.push(format!(
                "合成分量：{}。",
                if *maximum {
                    "取子分量绝对贡献的最大值"
                } else {
                    "对子分量贡献取平方和根"
                }
            ));
            for (child, child_detail) in children.iter().zip(&detail.children) {
                component(child, child_detail, &path, lines);
            }
            lines.push(String::new());
            lines.push(format!(
                "分量 {path}：子分量合成后的原始标准不确定度 = {}。",
                c(detail.raw_standard)
            ));
        }
    }
    lines.push(format!(
        "分量 {path}：{}；修约前 u = {}，修约后 u = {}。",
        error_process::rounding(detail.rounding),
        c(detail.raw_standard),
        format_process_rounded(detail.standard, detail.rounding.and_then(|r| r.digits))
    ));
    lines.push(format!(
        "贡献 q = 灵敏系数 × 修约后 u = {} × {} = {}。",
        n(detail.sensitivity),
        format_process_rounded(detail.standard, detail.rounding.and_then(|r| r.digits)),
        c(detail.contribution)
    ));
}

#[cfg(test)]
mod tests {
    use super::super::model::Rounding;
    use super::*;

    #[test]
    fn process_uses_actual_nested_values_methods_and_rounding() {
        for maximum in [false, true] {
            for range in [false, true] {
                for distribution in 0..6 {
                    let input = Component {
                        name: "合成".into(),
                        sensitivity: -2.0,
                        rounding: None,
                        kind: Kind::Combined {
                            maximum,
                            children: vec![
                                Component {
                                    name: "A".into(),
                                    sensitivity: 1.5,
                                    rounding: None,
                                    kind: Kind::TypeA {
                                        values: vec![1.0, 2.0, 3.0],
                                        range_method: range,
                                        average_count: if range { None } else { Some(4) },
                                    },
                                },
                                Component {
                                    name: "B".into(),
                                    sensitivity: 2.0,
                                    rounding: Some(Rounding {
                                        digits: Some(3),
                                        carry: false,
                                    }),
                                    kind: Kind::TypeB {
                                        half_width: 1.245,
                                        distribution,
                                    },
                                },
                            ],
                        },
                    };
                    let detail = input.calculate().unwrap();
                    let mut lines = Vec::new();
                    component(&input, &detail, "", &mut lines);
                    let text = lines.join("\n").replace('\u{2063}', "");
                    assert!(text.contains(if range {
                        "极差法：R ="
                    } else {
                        "取测量次数 n = 4"
                    }));
                    if !range {
                        assert!(text.contains("可不同于上方估计 s 的测量值个数"));
                    }
                    assert!(text.contains(if maximum {
                        "取子分量绝对贡献的最大值"
                    } else {
                        "对子分量贡献取平方和根"
                    }));
                    assert!(text.contains(&format!(
                        "子分量合成后的原始标准不确定度 = {}",
                        n(detail.raw_standard)
                    )));
                    for child in &detail.children {
                        assert!(text.contains(&format!(
                            "修约前 u = {}，修约后 u = {}",
                            n(child.raw_standard),
                            n(child.standard)
                        )));
                        assert!(text.contains(&format!(
                            "= {} × {} = {}。",
                            n(child.sensitivity),
                            n(child.standard),
                            n(child.contribution)
                        )));
                    }
                    assert!(
                        text.contains(&format!("除数 d = {}", n(model::divisor(distribution))))
                    );
                }
            }
        }
    }
}
