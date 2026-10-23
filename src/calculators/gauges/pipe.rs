//! R 与 NPT 密封锥管螺纹按各自内置规格表查询尺寸、公差和磨损偏差。
use super::{lookup::*, report::*};
use serde_json::Value;

pub(super) fn sizes(npt: bool) -> Vec<String> {
    let table = if npt {
        &database()["NPT_PGS"]
    } else {
        &database()["DATA"]["pipe"]["dim"]
    };
    let key = if npt { "规格" } else { "尺寸代号" };
    rows(table)
        .iter()
        .filter_map(|r| r[key].as_str().map(str::to_owned))
        .collect()
}

fn find<'a>(table: &'a Value, key: &str, code: &str) -> Result<&'a Value, String> {
    rows(table)
        .iter()
        .find(|r| r[key].as_str() == Some(code))
        .ok_or_else(|| format!("没有收录尺寸代号 {code}，请从标准系列选择"))
}

/// 所有表均显式列单位，避免同一行同时出现 mm、μm 而误解数量级。
fn properties(title: &str, row: &Value, fields: &[(&str, &str, &str)]) -> Table {
    let mut table = Table::new(title, &["参数", "数值", "单位"]);
    for &(key, label, unit) in fields {
        table.row(vec![label.into(), Cell::value(&row[key]), unit.into()]);
    }
    table
}

pub(super) fn calculate(npt_mode: bool, code: &str) -> Result<Report, String> {
    let code = code
        .trim()
        .trim_start_matches("NPT")
        .trim_start_matches(['R', 'r'])
        .trim()
        .replace(' ', "-");
    if npt_mode {
        return npt(&code);
    }
    let db = &database()["DATA"]["pipe"];
    let dim = find(&db["dim"], "尺寸代号", &code)?;
    let tol = find(&db["tol"], "尺寸代号", &code)?;
    let wear = find(&db["wear"], "尺寸代号", &code)?;
    let mut out = Report::new(format!("R {code}"));
    geometry(
        &mut out,
        n(dim, "D"),
        n(dim, "P"),
        n(dim, "D2"),
        n(dim, "D1"),
        55.,
        1,
    );
    out.parameter("TPI (牙/inch)", n(dim, "n"));
    out.metric("锥度", "1:16");
    out.tables.push(properties(
        "螺纹量规结构尺寸",
        dim,
        &[
            ("L0", "基准距离 L0", "mm"),
            ("L1", "管端至基面 L1", "mm"),
            ("L2", "装板距离 L2", "mm"),
            ("b3", "塞规台阶宽 b3", "mm"),
            ("b3偏差", "b3 对称偏差", "mm"),
            ("c", "沉孔 c", "mm"),
            ("L3尺寸", "L3", "mm"),
            ("L4", "L4", "mm"),
            ("T1尺寸", "台阶检验 T1", "mm"),
            ("g", "基面长 g", "mm"),
            ("L5", "L5", "mm"),
        ],
    ));
    let dev = find(&db["dev"], "尺寸代号", &code)?;
    out.tables.push(properties(
        "螺距、半角与锥度偏差",
        dev,
        &[
            ("螺距公差", "螺距公差", "mm"),
            ("牙型半角偏差", "牙型半角偏差 ±", "角分"),
            ("塞规锥度偏差", "塞规锥度上偏差（下偏差 0）", "mm"),
            ("环规锥度偏差", "环规锥度下偏差（上偏差 0）", "mm"),
            ("塞规光滑锥度偏差", "光滑塞规锥度偏差", "mm"),
            ("环规光滑锥度偏差", "光滑环规锥度偏差", "mm"),
        ],
    ));
    let sp = find(&db["sp"], "尺寸代号", &code)?;
    let sr = find(&db["sr"], "尺寸代号", &code)?;
    out.tables.push(properties(
        "光滑圆锥塞规 · 检内螺纹小径",
        sp,
        &[
            ("D1", "D₁", "mm"),
            ("L", "L", "mm"),
            ("k", "k", "mm"),
            ("j", "j", "mm"),
            ("c", "c", "mm"),
        ],
    ));
    out.tables.push(properties(
        "光滑圆锥环规 · 检外螺纹大径",
        sr,
        &[
            ("d", "d", "mm"),
            ("D4", "D₄", "mm"),
            ("L6", "L6", "mm"),
            ("L7", "L7", "mm"),
            ("L8", "L8", "mm"),
            ("L9", "L9", "mm"),
        ],
    ));
    out.note("R 中径以基准平面为准，锥度按 1:16；台阶判定及配作要求按实际标准执行，不能把锥螺纹当作直螺纹通止规。");
    out.tables.push(properties(
        "制造公差与位置要素",
        tol,
        &[
            ("TPL", "工作塞规 TPL", "μm"),
            ("TR", "工作环规 TR", "μm"),
            ("TCP", "校对塞规 TCP", "μm"),
        ],
    ));
    out.tables.push(properties(
        "中径磨损偏差（保留表中符号）",
        wear,
        &[("塞规", "塞规", "μm"), ("环规", "环规", "μm")],
    ));
    out.checked()
}

fn npt(code: &str) -> Result<Report, String> {
    let pg = find(&database()["NPT_PGS"], "规格", code)?;
    let rg = find(&database()["NPT_RNG"], "规格", code)?;
    let mut ring = rg.clone();
    // 原始 6 inch 环规 D₂=167.252，与同基面 D/D₁ 的牙型关系明显不符。
    // 尚无已核实的专用量规表依据，不擅自填成猜测值，保留其余表项。
    if code == "6" {
        ring["D2"] = Value::Null;
    }
    let mut out = Report::new(format!("NPT {code}"));
    geometry(
        &mut out,
        n(pg, "d"),
        n(pg, "P"),
        n(pg, "d2"),
        n(pg, "d1"),
        60.,
        1,
    );
    out.parameter("TPI (牙/inch)", n(pg, "tpi"));
    out.metric("锥度", "1:16");
    out.tables.push(properties(
        "NPT 工作塞规 · 基面尺寸",
        pg,
        &[
            ("L1", "基面距离 L1", "mm"),
            ("L", "螺纹长 L", "mm"),
            ("L2", "螺纹上限 L2", "mm"),
            ("L3", "螺纹下限 L3", "mm"),
            ("d", "基面大径 d", "mm"),
            ("d2", "基面中径 d₂", "mm"),
            ("d1", "基面小径 ≤d₁", "mm"),
            ("L0", "总长 L0", "mm"),
            ("xh", "型号", ""),
        ],
    ));
    out.tables.push(properties(
        "NPT 工作环规 · 基面尺寸",
        &ring,
        &[
            ("D", "基面大径 D", "mm"),
            ("D2", "基面中径 D₂", "mm"),
            ("D1", "基面小径 ≤D₁", "mm"),
            ("OD", "环规外径", "mm"),
            ("TH", "环规厚", "mm"),
            ("xh", "型号", ""),
        ],
    ));
    out.note("按内置规格表查询，基面为管端起 L1 处；螺距和尺寸保留表中修约口径，不从 TPI 重新推算已列尺寸。查表覆盖 1/16～6，包含 3-1/2。");
    if code == "6" {
        out.note("NPT 6 环规中径原表值存在疑点，暂不输出；请按实际执行标准或量规图纸确认该中径。");
    }
    out.checked()
}
