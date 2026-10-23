//! 各量规页面共用的标准直径、螺距和规格建议查询。
use super::lookup::*;

pub(super) fn metric_pitches(d: f64) -> Vec<f64> {
    let table = rows(&database()["DATA"]["metric"]["combo"]);
    let hit: Vec<_> = table.iter().filter(|r| same(n(r, "公称直径"), d)).collect();
    let mut out = Vec::new();
    if hit.is_empty() {
        out.extend(table.iter().map(|r| n(r, "粗牙")));
    } else {
        for row in hit {
            if let Some(obj) = row.as_object() {
                for (key, value) in obj {
                    if key == "粗牙" || key.starts_with("细牙") {
                        out.push(number(value));
                    }
                }
            }
        }
    }
    out.retain(|v| valid(*v) && *v > 0.);
    out.sort_by(|a, b| b.total_cmp(a));
    out.dedup_by(|a, b| same(*a, *b));
    out
}
pub(super) fn trap_pitches() -> Vec<f64> {
    let mut out = Vec::new();
    for row in rows(&database()["DATA"]["trap"]["series"]) {
        for key in ["螺距P1", "螺距P2", "螺距P3"] {
            let p = n(row, key);
            if valid(p) && p > 0. {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| b.total_cmp(a));
    out.dedup_by(|a, b| same(*a, *b));
    out
}

/// 用于规格建议而非自动改写用户输入，筛选不合法或缺失表值。
pub(super) fn diameters(trap: bool) -> Vec<String> {
    let table = if trap { "trap" } else { "metric" };
    let key = if trap { "series" } else { "combo" };
    let mut values: Vec<_> = rows(&database()["DATA"][table][key])
        .iter()
        .map(|r| n(r, "公称直径"))
        .filter(|v| valid(*v) && *v > 0.)
        .collect();
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| same(*a, *b));
    values.iter().map(|v| v.to_string()).collect()
}
