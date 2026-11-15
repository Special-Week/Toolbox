//! 误差及不确定度适配；动态数据来自数组，不依赖分隔符解析。
use crate::api::{Input, Output};
use crate::calculators::{
    common::{format_process_computed as c, format_process_number as n},
    error_process,
};
use crate::calculators::{
    numerics::{finite, rss, statistics},
    range_method,
    uncertainty::model as u,
};
use crate::catalog::{Dimension, ToolKind};
use crate::units::{convert_unit, units_for};
use serde_json::{Value, json};

const SAMPLES: &[f64] = &[10.02, 10.01, 10.03, 10.02, 10.01];
fn average_count(i: &Input, key: &str) -> Result<Option<usize>, String> {
    match i.0.get(key) {
        None => Ok(None),
        Some(Value::String(text)) => u::average_count(text),
        Some(Value::Number(number)) => u::average_count(&number.to_string()),
        _ => Err("取测量次数 n 须为正整数".into()),
    }
}
fn rounding(i: &Input, digits: &str, mode: &str) -> Result<u::Rounding, String> {
    let digits = match i.0.get(digits) {
        None => None,
        Some(Value::String(text)) => u::significant_digits(text)?,
        Some(Value::Number(number)) => u::significant_digits(&number.to_string())?,
        _ => return Err("有效数字须为 1～17 的整数，留空保留原始计算精度".into()),
    };
    Ok(u::Rounding {
        digits,
        carry: i.index(mode, 1, 2)? == 1,
    })
}
fn percent(numerator: f64, denominator: f64) -> Result<Option<f64>, String> {
    if denominator == 0.0 {
        return Ok(None);
    }
    let value = numerator / denominator * 100.;
    finite(&[value])?;
    Ok(Some(value))
}
pub(crate) fn calculate(kind: ToolKind, i: &Input) -> Result<Output, String> {
    let mut o = Output::default();
    match kind {
        ToolKind::Repeatability => {
            let values = i.values("values", SAMPLES, 2, 50)?;
            let s = statistics(&values)?;
            o.process = error_process::samples(&values, &s, true);
            o.process.push(format!(
                "极差 = 最大值 − 最小值 = {} − {} = {}。",
                n(s.max),
                n(s.min),
                c(s.max - s.min)
            ));
            o.process.push(format!(
                "平均值标准差 = s/√n = {} / √{} = {}。",
                c(s.sd),
                values.len(),
                c(s.standard_error)
            ));
            o.process.push(match percent(s.sd, s.mean.abs())? {
                Some(value) => format!(
                    "相对标准偏差 = s/|平均值| × 100 = {} / |{}| × 100 = {}%。",
                    c(s.sd),
                    c(s.mean),
                    c(value)
                ),
                None => "平均值为零，相对标准偏差无定义。".into(),
            });
            o.metric("测量次数", values.len(), "次");
            o.metric("平均值", s.mean, "");
            o.metric("样本标准差 s", s.sd, "");
            o.metric("平均值标准差", s.standard_error, "");
            o.metric("极差", s.max - s.min, "");
            o.optional("相对标准偏差 RSD", percent(s.sd, s.mean.abs())?, "%");
        }
        ToolKind::RangeMethod => {
            let (min, max, count) = if i.index("mode", 0, 2)? == 1 {
                let min = i.number("min", 10.01)?;
                let max = i.number("max", 10.03)?;
                let n = i.index("count", 5, 10)?;
                if n < 2 || max < min {
                    return Err("次数须为 2～9，最大值不能小于最小值".into());
                }
                (min, max, n)
            } else {
                let v = i.values("values", SAMPLES, 2, 9)?;
                let s = statistics(&v)?;
                (s.min, s.max, v.len())
            };
            let d = range_method::coefficient(count)?;
            let range = max - min;
            finite(&[range])?;
            let s = range / d;
            o.process = vec![
                format!(
                    "测量次数 n = {count}；最大值 = {}，最小值 = {}。",
                    n(max),
                    n(min)
                ),
                format!(
                    "极差 R = 最大值 − 最小值 = {} − {} = {}；查表系数 d₂ = {}。",
                    n(max),
                    n(min),
                    c(range),
                    n(d)
                ),
                format!(
                    "重复性标准差 s ≈ R/d₂ = {} / {} = {}；平均值标准差 = s/√n = {} / √{} = {}。",
                    c(range),
                    n(d),
                    c(s),
                    c(s),
                    count,
                    c(s / (count as f64).sqrt())
                ),
            ];
            o.metric("测量次数", count, "次");
            o.metric("极差 R", range, "");
            o.metric("极差系数 d₂", d, "");
            o.metric("重复性标准差 R/d₂", s, "");
            o.metric("平均值标准差", s / (count as f64).sqrt(), "");
        }
        ToolKind::ResultantError => {
            let mut errors = vec![];
            let mut result_unit = "";
            if i.index("mode", 0, 2)? == 0 {
                let v = i.values("values", &[0.02, 0.03, 0.01], 1, 50)?;
                let c = i.values("sensitivities", &vec![1.; v.len()], v.len(), v.len())?;
                errors = v.iter().zip(c.iter()).map(|(v, c)| v * c).collect();
                for (j, ((value, sensitivity), contribution)) in
                    v.iter().zip(&c).zip(&errors).enumerate()
                {
                    if j > 0 {
                        o.process.push(String::new());
                    }
                    o.process.push(format!(
                        "分量 {}：贡献 = 误差 × 灵敏系数 = {} × {} = {}。",
                        j + 1,
                        n(*value),
                        n(*sensitivity),
                        crate::calculators::common::format_process_computed(*contribution)
                    ));
                }
            } else {
                let units = units_for(Dimension::Pressure);
                let target = i.index("pressureUnit", 0, units.len())?;
                result_unit = units[target].code;
                let rows =
                    i.0.get("pressureComponents")
                        .and_then(Value::as_array)
                        .ok_or("请添加压力误差分量")?;
                if rows.is_empty() || rows.len() > 50 {
                    return Err("压力误差分量须为 1～50 项".into());
                }
                for (index, row) in rows.iter().enumerate() {
                    let component = Input(row.clone());
                    let source = component.index("unit", 0, units.len())?;
                    let value = component.number("value", 0.1)?;
                    if !o.process.is_empty() {
                        o.process.push(String::new());
                    }
                    let error = if component.index("type", 0, 2)? == 0 {
                        let low = component.number("low", 0.)?;
                        let high = component.number("high", 100.)?;
                        let span = crate::calculators::numerics::span(low, high)?;
                        let error = span * value / 100.;
                        o.process.push(format!("分量 {}：量程跨度 = 上限 − 下限 = {} − {} = {} {}；绝对误差 = 跨度 × 相对误差 / 100 = {} × {} / 100 = {} {}。", index + 1, n(high), n(low), c(span), units[source].code, c(span), n(value), c(error), units[source].code));
                        error
                    } else {
                        o.process.push(format!(
                            "分量 {}：输入绝对误差 = {} {}。",
                            index + 1,
                            n(value),
                            units[source].code
                        ));
                        value
                    };
                    let converted = convert_unit(Dimension::Pressure, error, source, target)?;
                    errors.push(converted);
                    let (from, _) = units[source].conversion_coefficients();
                    let (to, _) = units[target].conversion_coefficients();
                    o.process.push(if source == target {
                        format!(
                            "统一单位：分量单位与目标单位相同，直接使用 {} {}。",
                            c(converted),
                            result_unit
                        )
                    } else {
                        format!(
                            "统一单位：{} {} × ({} / {}) = {} {}。",
                            c(error),
                            units[source].code,
                            n(from),
                            n(to),
                            c(converted),
                            result_unit
                        )
                    });
                }
            }
            finite(&errors)?;
            let combined = rss(errors.iter().copied())?;
            let sum = errors.iter().map(|e| e.abs()).sum::<f64>();
            o.process.push(String::new());
            o.process.push(format!("合成误差 = √Σ贡献² = {} {result_unit}；最坏情况界限 = Σ|贡献| = {} {result_unit}。",c(combined),c(sum)));
            finite(&[sum])?;
            o.metric("分量个数", errors.len(), "项");
            o.metric("合成误差（平方和根）", combined, result_unit);
            o.metric("最坏情况界限（绝对值和）", sum, result_unit);
            o.metric(
                "最大单项贡献",
                errors.iter().map(|e| e.abs()).fold(0., f64::max),
                result_unit,
            );
        }
        ToolKind::ErrorAnalysis => {
            let value = i.number("value", 10.02)?;
            let reference = i.number("reference", 10.)?;
            let base_uses_reference = match i.0.get("base") {
                None => true,
                Some(Value::String(s)) => s.trim().is_empty(),
                _ => false,
            };
            let base = if base_uses_reference {
                reference
            } else {
                i.number("base", reference)?
            };
            let d = value - reference;
            let relative = percent(d, reference)?;
            let quoted = percent(d, base)?;
            o.process = vec![
                format!(
                    "绝对误差 Δ = 测量值 − 真值 = {} − {} = {}。",
                    n(value),
                    n(reference),
                    c(d)
                ),
                format!("误差绝对值 |Δ| = {}。", c(d.abs())),
                String::new(),
                relative
                    .map(|v| {
                        format!(
                            "相对误差 = Δ/真值 × 100 = {} / {} × 100 = {}%。",
                            c(d),
                            n(reference),
                            c(v)
                        )
                    })
                    .unwrap_or("真值为零，相对误差未定义。".into()),
                format!(
                    "{}{}",
                    if base_uses_reference {
                        "引用值留空，使用真值；"
                    } else {
                        ""
                    },
                    quoted
                        .map(|v| {
                            format!(
                                "引用误差 = Δ/引用值 × 100 = {} / {} × 100 = {}%。",
                                c(d),
                                n(base),
                                c(v)
                            )
                        })
                        .unwrap_or("引用值为零，引用误差未定义。".into())
                ),
            ];
            finite(&[d])?;
            o.metric("绝对误差 Δ（保留正负号）", d, "");
            o.metric("绝对值 |Δ|", d.abs(), "");
            o.optional("相对误差", percent(d, reference)?, "%");
            o.optional("引用误差", percent(d, base)?, "%");
        }
        ToolKind::Uncertainty => return uncertainty(i),
        _ => return Err("统计工具不存在".into()),
    }
    Ok(o)
}

/// 树形预算支持四层分组，B 类输入始终为区间半宽。
fn component(v: &Value, depth: usize) -> Result<u::Component, String> {
    if depth > 4 {
        return Err("分量嵌套最多四层".into());
    }
    let i = Input(v.clone());
    let kind = match i.index("kind", 1, 3)? {
        0 => {
            let range_method = i.index("method", 0, 2)? == 1;
            u::Kind::TypeA {
                values: i.values("values", SAMPLES, 2, 50)?,
                range_method,
                average_count: if range_method {
                    None
                } else {
                    average_count(&i, "averageCount")?
                },
            }
        }
        1 => u::Kind::TypeB {
            half_width: i.number("halfWidth", 0.01)?,
            distribution: i.index("distribution", 0, 6)?,
        },
        _ => {
            if depth >= 4 {
                return Err("第四层分量不能继续分组".into());
            }
            let children =
                i.0.get("children")
                    .and_then(Value::as_array)
                    .ok_or("合成分量缺少子分量")?;
            if children.is_empty() || children.len() > 12 {
                return Err("每组需要 1～12 个子分量".into());
            }
            u::Kind::Combined {
                children: children
                    .iter()
                    .map(|c| component(c, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?,
                maximum: i.index("combine", 0, 2)? == 1,
            }
        }
    };
    Ok(u::Component {
        name: i.text("name", "分量").into(),
        sensitivity: i.number("sensitivity", 1.)?,
        kind,
        rounding: Some(rounding(&i, "digits", "rounding")?),
    })
}
fn detail(d: &u::Detail) -> Value {
    json!({"name":d.name,"rawStandard":d.raw_standard,"standard":d.standard,"formattedStandard":u::format_rounded(d.standard,d.rounding),"sensitivity":d.sensitivity,"contribution":d.contribution,"children":d.children.iter().map(detail).collect::<Vec<_>>()})
}
fn uncertainty(i: &Input) -> Result<Output, String> {
    let mut o = Output::default();
    let mut contributions = vec![];
    let mut details = vec![];
    if i.index("mode", 0, 2)? == 1 {
        let a = i.values("aValues", SAMPLES, 0, 50)?;
        let raw_ua = if a.is_empty() {
            0.0
        } else {
            let stats = statistics(&a)?;
            match average_count(i, "aAverageCount")? {
                Some(n) => stats.sd / (n as f64).sqrt(),
                None => stats.sd,
            }
        };
        let a_rounding = if a.is_empty() {
            None
        } else {
            Some(rounding(i, "aDigits", "aRounding")?)
        };
        let ua = match a_rounding {
            Some(r) => u::round_significant(raw_ua, r)?,
            None => 0.0,
        };
        let sensitivity = i.number("aSensitivity", 1.)?;
        if !a.is_empty() {
            let item = u::Component {
                name: "A 类重复测量".into(),
                sensitivity,
                kind: u::Kind::TypeA {
                    values: a.clone(),
                    range_method: false,
                    average_count: average_count(i, "aAverageCount")?,
                },
                rounding: a_rounding,
            };
            let detail = item.calculate()?;
            crate::calculators::uncertainty::process::component(&item, &detail, "", &mut o.process);
        } else {
            o.process.push("未使用 A 类重复测量，uA = 0。".into());
        }
        let a_contribution = ua * sensitivity;
        finite(&[a_contribution])?;
        contributions.push(a_contribution);
        details.push(json!({"name":"A 类重复测量","rawStandard":raw_ua,"standard":ua,"formattedStandard":u::format_rounded(ua,a_rounding),"sensitivity":sensitivity,"contribution":a_contribution,"children":[]}));
        o.metric("A 类标准不确定度", raw_ua, "");
        o.metric("A 类修约后标准不确定度", ua, "");
        if let Some(nodes) = i.0.get("bComponents") {
            let nodes = nodes.as_array().ok_or("B 类分量数据无效")?;
            if nodes.len() > 50 {
                return Err("B 类分量最多 50 项".into());
            }
            if nodes.is_empty() && a.is_empty() {
                return Err("至少需要一组 A 类数据或一个 B 类分量".into());
            }
            for node in nodes {
                if Input(node.clone()).index("kind", 1, 3)? != 1 {
                    return Err("快速评定中的 B 类列表只能包含 B 类分量".into());
                }
                let item = component(node, 0)?;
                let d = item.calculate()?;
                crate::calculators::uncertainty::process::component(&item, &d, "", &mut o.process);
                contributions.push(d.contribution);
                details.push(detail(&d));
            }
        } else {
            // 旧输入仍能计算；页面初始化后会迁移为逐项分布与灵敏系数。
            let b = i.values("bValues", &[0.01, 0.02], 1, 50)?;
            let distribution = i.index("distribution", 0, 6)?;
            for (j, v) in b.into_iter().enumerate() {
                let item = u::Component {
                    name: format!("B 类分量 {}", j + 1),
                    sensitivity: 1.,
                    kind: u::Kind::TypeB {
                        half_width: v,
                        distribution,
                    },
                    rounding: Some(u::Rounding {
                        digits: None,
                        carry: true,
                    }),
                };
                let d = item.calculate()?;
                crate::calculators::uncertainty::process::component(&item, &d, "", &mut o.process);
                contributions.push(d.contribution);
                details.push(detail(&d));
            }
        }
    } else {
        let defaults = json!([{"name":"重复测量","kind":0,"values":SAMPLES,"sensitivity":1},{"name":"标准器","kind":1,"halfWidth":0.02,"distribution":0,"sensitivity":1}]);
        let nodes =
            i.0.get("components")
                .unwrap_or(&defaults)
                .as_array()
                .ok_or("预算表数据无效")?;
        if nodes.is_empty() || nodes.len() > 20 {
            return Err("预算表顶层需要 1～20 个分量".into());
        }
        for node in nodes {
            let item = component(node, 0)?;
            let d = item.calculate()?;
            crate::calculators::uncertainty::process::component(&item, &d, "", &mut o.process);
            contributions.push(d.contribution);
            details.push(detail(&d));
        }
    }
    let uc = u::combine(&contributions, i.index("combine", 0, 2)? == 1)?;
    let k = i.number("coverage", 2.)?;
    if k <= 0. {
        return Err("包含因子 k 必须大于 0".into());
    }
    let expanded = uc * k;
    finite(&[expanded])?;
    let digits = i.index("digits", 1, 4)? + 1;
    let (rounded, ru) = u::rounded_combined(uc, k, digits, i.index("roundingV2", 1, 2)? == 1)?;
    o.process.push(String::new());
    o.process.push(format!(
        "总合成方法：{}；u_c = {}。",
        if i.index("combine", 0, 2)? == 1 {
            "取最大绝对贡献"
        } else {
            "各项贡献平方和根"
        },
        c(uc)
    ));
    error_process::combined(
        &mut o.process,
        uc,
        k,
        digits,
        i.index("roundingV2", 1, 2)? == 1,
        rounded,
        ru,
    );
    o.metric("合成标准不确定度 uc", uc, "");
    o.metric("扩展不确定度 U", expanded, "");
    o.metric("包含因子 k", k, "");
    o.metric("修约后 uc", rounded, "");
    o.metric("修约后 U", ru, "");
    o.details = json!({"budget":details});
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_components_explain_actual_substitutions_and_unit_conversion() {
        let input = Input(json!({"mode":1, "pressureUnit":1, "pressureComponents":[
            {"unit":0,"type":0,"low":-20,"high":80,"value":-0.5},
            {"unit":1,"type":1,"value":0.25}
        ]}));
        let output = calculate(ToolKind::ResultantError, &input).unwrap();
        let text = output.process.join("\n").replace('\u{2063}', "");
        assert!(text.contains("上限 − 下限 = 80 − -20 = 100"), "{text}");
        assert!(text.contains("100 × -0.5 / 100 = -0.5"), "{text}");
        assert!(text.contains("分量 2：输入绝对误差 = 0.25"), "{text}");
        assert!(
            text.contains("分量单位与目标单位相同，直接使用 0.25"),
            "{text}"
        );
        let converted = convert_unit(Dimension::Pressure, -0.5, 0, 1).unwrap();
        assert!(text.contains(&n(converted)), "{text}");
        let total = output
            .metrics
            .iter()
            .find(|metric| metric.label == "合成误差（平方和根）")
            .unwrap()
            .value
            .as_f64()
            .unwrap();
        assert!((total - converted.hypot(0.25)).abs() <= 1e-12 * total);
    }

    fn result(input: Value) -> Result<Output, String> {
        uncertainty(&Input(input))
    }

    #[test]
    fn average_count_works_for_quick_and_nested_budgets() {
        let a = json!({"kind":"0", "method":"0", "values":[1, 2, 3]});
        let mut budget = json!({"components":[{"kind":"2", "children":[a]}]});
        let mut quick =
            json!({"mode":"1", "aValues":[1,2,3], "bComponents":[{"kind":"1", "halfWidth":0}]});
        for (input, path) in [
            (&mut budget, "/components/0/children/0/averageCount"),
            (&mut quick, "/aAverageCount"),
        ] {
            let default = result(input.clone()).unwrap();
            let standard = default
                .metrics
                .iter()
                .find(|m| m.label == "合成标准不确定度 uc")
                .unwrap()
                .value
                .as_f64()
                .unwrap();
            assert!((standard - 1.0).abs() < 1e-12);
            let (parent, key) = path.rsplit_once('/').unwrap();
            let parent = if parent.is_empty() {
                &mut *input
            } else {
                input.pointer_mut(parent).unwrap()
            };
            parent[key] = json!("4");
            let custom = result(input.clone()).unwrap();
            let standard = custom
                .metrics
                .iter()
                .find(|m| m.label == "合成标准不确定度 uc")
                .unwrap()
                .value
                .as_f64()
                .unwrap();
            assert!((standard - 0.5).abs() < 1e-12);
            for invalid in [json!("0"), json!("2.5"), json!("-1"), json!(null)] {
                let parent = if parent_path(path).is_empty() {
                    &mut *input
                } else {
                    input.pointer_mut(parent_path(path)).unwrap()
                };
                parent[key] = invalid;
                assert!(result(input.clone()).is_err());
            }
        }
        let range = json!({"components":[{"kind":"0", "method":"1", "values":[1,2,3], "averageCount":"invalid"}]});
        assert!(result(range).is_ok());
    }

    fn parent_path(path: &str) -> &str {
        path.rsplit_once('/').unwrap().0
    }

    #[test]
    fn rounds_each_component_before_combining_and_supports_empty_quick_parts() {
        fn uc(input: Value) -> f64 {
            result(input)
                .unwrap()
                .metrics
                .iter()
                .find(|m| m.label == "合成标准不确定度 uc")
                .unwrap()
                .value
                .as_f64()
                .unwrap()
        }
        let mut budget = json!({"components":[{"kind":"1", "halfWidth":"1.245", "distribution":"4", "sensitivity":"2"}]});
        assert!((uc(budget.clone()) - 2.49).abs() < 1e-12);
        budget["components"][0]["digits"] = json!("3");
        assert!((uc(budget.clone()) - 2.5).abs() < 1e-12);
        budget["components"][0]["rounding"] = json!("0");
        assert!((uc(budget.clone()) - 2.48).abs() < 1e-12);
        let details = result(budget.clone()).unwrap().details;
        assert_eq!(details["budget"][0]["formattedStandard"], "1.24");
        budget["components"][0]["digits"] = json!("18");
        assert!(result(budget).is_err());

        let nested = json!({"components":[{"kind":2,"digits":"2","children":[
            {"kind":1,"distribution":4,"halfWidth":"1.235","sensitivity":2,"digits":"3","rounding":0},
            {"kind":1,"distribution":4,"halfWidth":"0.445","digits":"2","rounding":0}
        ]}]});
        assert_eq!(uc(nested), 2.6);

        let quick = json!({"mode":"1", "aValues":[0,1], "aDigits":"2", "aSensitivity":"2",
            "bComponents":[{"kind":"1", "halfWidth":"1.245", "distribution":"4", "digits":"3", "rounding":"0"}]});
        assert!((uc(quick) - 1.42_f64.hypot(1.24)).abs() < 1e-12);
        assert!((uc(json!({"mode":"1", "aValues":[], "bComponents":[{"kind":"1", "halfWidth":1, "distribution":4}]})) - 1.0).abs() < 1e-12);
        assert!((uc(json!({"mode":"1", "aValues":[1,2,3], "bComponents":[]})) - 1.0).abs() < 1e-12);
        assert!(result(json!({"mode":"1", "aValues":[], "bComponents":[]})).is_err());
        let mut final_round = json!({"coverage":1, "digits":1, "roundingV2":0,
            "components":[{"kind":1,"halfWidth":"2.45","distribution":4}]});
        let rounded = |value: Value| {
            result(value)
                .unwrap()
                .metrics
                .iter()
                .find(|m| m.label == "修约后 uc")
                .unwrap()
                .value
                .as_f64()
                .unwrap()
        };
        assert_eq!(rounded(final_round.clone()), 2.4);
        final_round["roundingV2"] = json!(1);
        assert_eq!(rounded(final_round.clone()), 2.5);
        final_round.as_object_mut().unwrap().remove("roundingV2");
        final_round["rounding"] = json!(0); // 旧设置不改变新默认的进位。
        assert_eq!(rounded(final_round), 2.5);
        let mut quick_round = json!({"mode":1, "aValues":[], "bComponents":[{"kind":1, "halfWidth":"2.45", "distribution":4}], "coverage":1, "digits":1});
        assert_eq!(rounded(quick_round.clone()), 2.5);
        quick_round["roundingV2"] = json!(0);
        assert_eq!(rounded(quick_round.clone()), 2.4);
        quick_round["digits"] = json!(2);
        assert_eq!(rounded(quick_round), 2.45);
    }
}
