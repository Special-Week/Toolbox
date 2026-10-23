//! 保留原始表字段、单位与缺失标记；所有分档采用下界开、上界闭，近邻替代必须返回提示。
use serde_json::Value;
use std::sync::OnceLock;

pub(super) fn database() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut data: Value =
            serde_json::from_str(include_str!("../../../data/gauges/tables.json"))
                .expect("内置量规表必须是有效 JSON");
        // 原始数据保留不改写；这些孔基本偏差录入错误按 ISO 286-1 表值修正，单位 μm。
        for row in data["DATA"]["smooth"]["ebH"].as_array_mut().unwrap() {
            match n(row, "基本尺寸≤") {
                3. => row["F"] = 6.into(),
                30. => row["D"] = 65.into(),
                80. => row["D"] = 100.into(),
                120. => row["D"] = 120.into(),
                _ => {}
            }
        }
        data
    })
}
pub(super) fn rows(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub(super) fn number(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(f64::NAN)
}
pub(super) fn n(row: &Value, key: &str) -> f64 {
    number(&row[key])
}
pub(super) fn valid(value: f64) -> bool {
    value.is_finite() && value != 99999.0 && value != -100000.0
}
pub(super) fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}
pub(super) fn first(row: &Value, keys: &[&str]) -> f64 {
    keys.iter()
        .find_map(|k| (!row[*k].is_null()).then(|| n(row, k)))
        .unwrap_or(f64::NAN)
}
pub(super) fn pitch(row: &Value) -> f64 {
    first(row, &["螺距P", "螺距"])
}
fn lower(row: &Value) -> f64 {
    first(
        row,
        &["基本大径D＞", "基本大径d＞", "基本大径>", "公称直径d>"],
    )
}
fn upper(row: &Value) -> f64 {
    first(
        row,
        &["基本大径D≤", "基本大径d≤", "基本大径≤", "公称直径d≤"],
    )
}

pub(super) fn nearest(values: impl IntoIterator<Item = f64>, x: f64) -> Option<f64> {
    values
        .into_iter()
        .filter(|v| valid(*v))
        .min_by(|a, b| (a - x).abs().total_cmp(&(b - x).abs()))
}
/// 公差表取最接近螺距；等距离时优先较小螺距，与网页排序后的选择一致。
pub(super) fn tolerance(
    table: &Value,
    d: f64,
    p: f64,
    grade: &str,
    notes: &mut Vec<String>,
) -> Result<(f64, f64), String> {
    let table = rows(table);
    let mut pitches: Vec<_> = table.iter().map(pitch).filter(|x| valid(*x)).collect();
    pitches.sort_by(f64::total_cmp);
    let used = nearest(pitches, p).ok_or("公差表未收录该螺距")?;
    if !same(used, p) {
        notes.push(format!(
            "公差、偏差按 P={used} mm 档取值；几何尺寸按输入螺距计算。"
        ));
    }
    let min = table.iter().map(lower).fold(f64::INFINITY, f64::min);
    let clamped = d <= min;
    let hit = table.iter().find(|r| {
        same(pitch(r), used)
            && ((d > lower(r) && d <= upper(r)) || (clamped && same(lower(r), min)))
    });
    let row = if let Some(row) = hit {
        if clamped {
            notes.push(format!("直径 {d} mm 低于公差表下限，采用最小直径档。"));
        }
        row
    } else {
        notes.push(format!(
            "直径 {d} mm 与 P={used} mm 的标准档不匹配，采用该螺距首次出现的直径档，仅作参考。"
        ));
        table
            .iter()
            .find(|r| same(pitch(r), used))
            .ok_or("没有对应公差档")?
    };
    let value = n(row, &format!("公差等级{grade}"));
    if !valid(value) {
        return Err(format!(
            "P={used} mm 档没有 {grade} 级中径公差定义，请更换公差带。"
        ));
    }
    Ok((value, used))
}
pub(super) fn deviation(table: &Value, p: f64, key: &str) -> Result<f64, String> {
    let row = closest_pitch(table, p).ok_or("未收录基本偏差")?;
    let value = n(row, key);
    if valid(value) {
        Ok(value)
    } else {
        Err(format!("该螺距档没有 {key} 的定义"))
    }
}
pub(super) fn closest_pitch(table: &Value, p: f64) -> Option<&Value> {
    rows(table)
        .iter()
        .min_by(|a, b| (pitch(a) - p).abs().total_cmp(&(pitch(b) - p).abs()))
}
pub(super) fn minor_tolerance(table: &Value, p: f64, grade: &str) -> Option<f64> {
    let row = closest_pitch(table, p)?;
    [format!("公差等级{grade}"), format!("{grade}级公差")]
        .iter()
        .find(|key| !row[key.as_str()].is_null())
        .and_then(|key| {
            let x = n(row, key);
            valid(x).then_some(x)
        })
}

/// Tr 外螺纹小径公差同时依赖直径、螺距和中径公差带位置，不能固定取第一行或 c 列。
pub(super) fn trapezoidal_minor_tolerance(
    d: f64,
    p: f64,
    grade: &str,
    letter: &str,
    notes: &mut Vec<String>,
) -> Option<f64> {
    let table = rows(&database()["DATA"]["trap"]["tdo"]);
    let mut pitches: Vec<_> = table.iter().map(pitch).collect();
    pitches.sort_by(f64::total_cmp);
    let used = nearest(pitches, p)?;
    let candidates: Vec<_> = table.iter().filter(|r| same(pitch(r), used)).collect();
    let row = candidates
        .iter()
        .find(|r| d > lower(r) && d <= upper(r))
        .copied()
        .or_else(|| {
            notes.push("外螺纹小径公差采用该螺距最近的直径档，仅作参考。".into());
            candidates.into_iter().min_by(|a, b| {
                let distance = |r: &Value| (lower(r) - d).max(d - upper(r)).max(0.);
                distance(a).total_cmp(&distance(b))
            })
        })?;
    let value = n(row, &format!("{letter}{grade}"));
    valid(value).then_some(value)
}

#[derive(Clone, Copy)]
pub(super) struct Factors {
    pub tr: f64,
    pub tpl: f64,
    pub tcp: f64,
    pub m: f64,
    pub zr: f64,
    pub zpl: f64,
    pub rgo: f64,
    pub rng: f64,
    pub sgo: f64,
    pub sng: f64,
}
/// 位置要素统一转换为 mm，保持 ZR 的正负号，不对负值取绝对值。
pub(super) fn factors(trap: bool, tolerance: f64, negative_es: bool) -> Result<Factors, String> {
    let sys = if trap { "trap" } else { "metric" };
    let (lo, hi) = if trap {
        ("Td2/TD2>", "Td2/TD2≤")
    } else {
        ("Td2或者TD2＞", "Td2或者TD2≤")
    };
    let row = rows(&database()["DATA"][sys]["zf"])
        .iter()
        .find(|r| tolerance > n(r, lo) && tolerance <= n(r, hi))
        .ok_or_else(|| format!("工件中径公差 {tolerance:.5} μm 超出位置要素表范围"))?;
    let prefix = if trap { "环规" } else { "螺纹环规" };
    let plug = if trap { "塞规" } else { "螺纹塞规" };
    let zr = if trap {
        if negative_es { "ZR(es<0)" } else { "ZR(es=0)" }
    } else {
        "ZR"
    };
    Ok(Factors {
        tr: n(row, "TR") / 1000.,
        tpl: n(row, "TPL") / 1000.,
        tcp: n(row, "TCP") / 1000.,
        m: n(row, "m") / 1000.,
        zr: n(row, zr) / 1000.,
        zpl: n(row, "ZPL") / 1000.,
        rgo: n(row, &format!("{prefix}WGO")) / 1000.,
        rng: n(row, &format!("{prefix}WNG")) / 1000.,
        sgo: n(row, &format!("{plug}WGO")) / 1000.,
        sng: n(row, &format!("{plug}WNG")) / 1000.,
    })
}
pub(super) fn positive_geometry(d: f64, p: f64, d2: f64, d1: f64) -> Result<(), String> {
    if [d, p, d2, d1].iter().any(|v| !v.is_finite() || *v <= 0.) || d > 1e6 || p > 1e5 {
        Err("直径和螺距须为有效正数，并形成大径＞中径＞小径＞0 的几何关系".into())
    } else if !(d > d2 && d2 > d1) {
        Err("当前直径与螺距不能形成有效牙型".into())
    } else {
        Ok(())
    }
}

/// 表中的长度参数按原档位取值；梯形先匹配直径，再在该直径下匹配螺距。
pub(super) fn lengths(
    trap: bool,
    ring: bool,
    d: f64,
    p: f64,
    notes: &mut Vec<String>,
) -> (f64, f64) {
    let table =
        &database()["DATA"][if trap { "trap" } else { "metric" }][if ring { "rl" } else { "sl" }];
    let table = rows(table);
    let candidates: Vec<_> = if trap {
        let chosen = nearest(table.iter().map(|r| n(r, "公称直径")), d).unwrap_or(f64::NAN);
        if !same(chosen, d) {
            notes.push(format!("量规长度采用最近标准直径 Tr{chosen} 档。"));
        }
        table
            .iter()
            .filter(|r| same(n(r, "公称直径"), chosen))
            .collect()
    } else {
        let min = table.iter().map(lower).fold(f64::INFINITY, f64::min);
        let a: Vec<_> = table
            .iter()
            .filter(|r| (d > lower(r) && d <= upper(r)) || (d <= min && same(lower(r), min)))
            .collect();
        if a.is_empty() {
            notes.push("量规长度：直径超出对应档，采用该螺距首次出现的长度档参考。".into());
            table.iter().filter(|r| same(pitch(r), p)).take(1).collect()
        } else {
            if d <= min {
                notes.push("量规长度：直径低于表下限，采用最小直径档参考。".into());
            }
            a
        }
    };
    let used = nearest(candidates.iter().map(|r| pitch(r)), p).unwrap_or(f64::NAN);
    if valid(used) && !same(used, p) {
        notes.push(format!("量规长度采用最近标准螺距 P={used} mm 档。"));
    }
    if !valid(used) {
        notes.push("未收录该规格的量规长度，长度显示“—”。".into());
    }
    let hit = candidates.iter().find(|r| same(pitch(r), used));
    hit.map(|r| (n(r, "通端"), n(r, "止端")))
        .unwrap_or((f64::NAN, f64::NAN))
}

pub(super) fn metric_lengths(d: f64, p: f64, notes: &mut Vec<String>) -> (f64, f64, f64, String) {
    if d > 100. {
        notes.push("D＞100 mm：锥度锁紧式长度和手柄表不适用，需另定结构。".into());
        return (f64::NAN, f64::NAN, f64::NAN, "—".into());
    }
    let db = database();
    let pick = |key: &str| -> Option<&Value> {
        rows(&db[key])
            .iter()
            .find(|r| d.max(1.) >= number(&r[0]) && d.max(1.) <= number(&r[1]))
            .map(|r| &r[2])
    };
    let table5 = if same(d, 62.) {
        Some(&db["T5_62"])
    } else {
        pick("T5")
    };
    let table6 = if same(d, 62.) {
        Some(&db["T6_62"])
    } else {
        pick("T6")
    };
    let Some(t5) = table5.and_then(Value::as_object) else {
        return (f64::NAN, f64::NAN, f64::NAN, "—".into());
    };
    // 恢复长度表原来的键顺序：整数键优先，其余螺距升序；等距时取先出现的档。
    let mut pitches: Vec<f64> = t5.keys().filter_map(|k| k.parse().ok()).collect();
    pitches.sort_by(|a, b| {
        let ai = a.fract() == 0.;
        let bi = b.fract() == 0.;
        bi.cmp(&ai).then_with(|| a.total_cmp(b))
    });
    let used = nearest(pitches, p).unwrap_or(p);
    let key = t5
        .keys()
        .find(|k| same(k.parse().unwrap_or(f64::NAN), used))
        .cloned()
        .unwrap_or_default();
    if !same(p, used) {
        notes.push(format!("塞规长度采用该直径档最近螺距 P={used} mm。"));
    }
    if d < 1. {
        notes.push("D＜1 mm：塞规长度按 1～3 mm 直径档参考。".into());
    }
    let pair = &table6.unwrap_or(&Value::Null)[&key];
    let handle = rows(&db["HANDLE"])
        .iter()
        .find(|r| d.max(1.) >= number(&r[0]) && d.max(1.) <= number(&r[1]))
        .and_then(|r| r[2].as_str())
        .unwrap_or("—");
    (
        number(&pair[0]),
        number(&pair[1]),
        number(&t5[&key]),
        format!("{handle}#"),
    )
}
