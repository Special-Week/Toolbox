//! 光滑极限量规。公差等级与基本偏差分开查询，未收录的等级修正不作猜测。
use super::{lookup::*, report::*};

pub(super) fn letters(hole: bool) -> Vec<String> {
    rows(&database()[if hole { "SM_H_LETTERS" } else { "SM_S_LETTERS" }])
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

pub(super) fn calculate(d: f64, mode: usize, letter: &str, grade: usize) -> Result<Report, String> {
    if !d.is_finite() || d <= 0. || d > 500. {
        return Err("基本尺寸范围为 0＜D≤500 mm".into());
    }
    if !(6..=16).contains(&grade) || mode > 2 {
        return Err("请选择 IT6～IT16 和有效量规类型".into());
    }
    let hole = mode == 0;
    if !letters(hole).iter().any(|s| s == letter) {
        return Err("孔/轴公差带字母与量规类型不匹配".into());
    }
    let db = &database()["DATA"]["smooth"];
    let band = |r: &&serde_json::Value| d > n(r, "基本尺寸＞") && d <= n(r, "基本尺寸≤");
    let row = rows(&db["it"])
        .iter()
        .find(band)
        .ok_or("基本尺寸没有公差档")?;
    let (itu, t1u, z1u) = (
        n(row, &format!("IT{grade}公差")),
        n(row, &format!("IT{grade}T1")),
        n(row, &format!("IT{grade}Z1")),
    );
    if [itu, t1u, z1u].iter().any(|v| !valid(*v)) {
        return Err("该尺寸档的工件公差或量规位置要素未定义".into());
    }
    let (it, t1, z1) = (itu / 1000., t1u / 1000., z1u / 1000.);
    let eb = rows(&db[if hole { "ebH" } else { "ebS" }])
        .iter()
        .find(band)
        .ok_or("没有对应基本偏差档")?;
    let symmetric = letter.eq_ignore_ascii_case("js");
    if hole
        && grade <= 7
        && [
            "P", "R", "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC",
        ]
        .contains(&letter)
    {
        return Err(format!(
            "{letter}{grade} 需要等级相关的 Δ 修正，内置表未完整收录，不能直接使用该字母的通用基本偏差。"
        ));
    }
    if hole && d <= 1. && ["A", "B"].contains(&letter) {
        return Err("基本尺寸不大于 1 mm 时不采用 A、B 孔公差带".into());
    }
    let v = n(eb, letter);
    if !symmetric && !valid(v) {
        return Err(format!(
            "{letter}{grade} 在 D={d} mm 档没有可直接使用的基本偏差；需等级/Δ 修正的组合未完整收录，不能推算。"
        ));
    }
    let (ei, es) = if symmetric {
        (-it / 2., it / 2.)
    } else if hole {
        if "ABCDEFGH".contains(letter) {
            (v / 1000., v / 1000. + it)
        } else {
            (v / 1000. - it, v / 1000.)
        }
    } else if "abcdefgh".contains(letter) {
        (v / 1000. - it, v / 1000.)
    } else {
        (v / 1000., v / 1000. + it)
    };
    let (go, ng, wear) = if hole {
        (
            [d + ei + z1 - t1 / 2., d + ei + z1 + t1 / 2.],
            [d + es - t1, d + es],
            d + ei,
        )
    } else {
        (
            [d + es - z1 - t1 / 2., d + es - z1 + t1 / 2.],
            [d + ei, d + ei + t1],
            d + es,
        )
    };
    if go
        .iter()
        .chain(ng.iter())
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err("该尺寸与公差组合产生非正量规尺寸，请核对参数".into());
    }
    let label = if hole {
        "孔用塞规"
    } else if mode == 2 {
        "轴用卡规"
    } else {
        "轴用环规"
    };
    let mut out = Report::new(format!("光滑量规 · {label} · {d} {letter}{grade}"));
    out.parameter("基本尺寸 D (mm)", d);
    out.metric("工件公差带", format!("{letter}{grade}"));
    out.metric("工件公差 IT (μm)", itu);
    out.metric(
        if hole {
            "下偏差 EI (mm)"
        } else {
            "下偏差 ei (mm)"
        },
        ei,
    );
    out.metric(
        if hole {
            "上偏差 ES (mm)"
        } else {
            "上偏差 es (mm)"
        },
        es,
    );
    out.metric("制造公差 T1 (μm)", t1u);
    out.metric("位置要素 Z1 (μm)", z1u);
    let mut table = Table::new(
        label,
        &["量规", "尺寸下限 (mm)", "尺寸上限 (mm)", "磨损极限 (mm)"],
    );
    table.row(vec![
        "GO · 通端".into(),
        go[0].into(),
        go[1].into(),
        wear.into(),
    ]);
    table.row(vec![
        "NO GO · 止端".into(),
        ng[0].into(),
        ng[1].into(),
        "—".into(),
    ]);
    out.tables.push(table);
    out.metric("工件尺寸下限 (mm)", d + ei);
    out.metric("工件尺寸上限 (mm)", d + es);
    if mode == 2 {
        out.note("卡规与环规使用同组制造公差，卡规为两平行面、非全形检验，应结合工件形状要求。");
    }
    out.note("基本偏差按内置表直接取值；特殊字母的完整等级与 Δ 修正规则未收录。结果须结合实际执行标准核验。");
    out.checked()
}
