//! M/Tr 共用公差带构造；几何、偏差和内外螺纹的尺寸基准分别处理，尺寸运算保留完整精度。
use super::{lookup::*, report::*};

/// 一行量规尺寸：未定义的界限保留 NaN，输出层显示“—”，不把缺失值当零。
#[derive(Clone)]
pub(super) struct Gauge {
    pub code: &'static str,
    pub name: &'static str,
    pub pd: [f64; 2],
    pub center: f64,
    pub major: [f64; 2],
    pub major_min: f64,
    pub minor_max: f64,
    pub minor: [f64; 2],
    pub wear: f64,
    pub length: f64,
}
impl Gauge {
    pub fn new(code: &'static str, name: &'static str, center: f64, tolerance: f64) -> Self {
        Self {
            code,
            name,
            pd: [center - tolerance / 2., center + tolerance / 2.],
            center,
            major: [f64::NAN; 2],
            major_min: f64::NAN,
            minor_max: f64::NAN,
            minor: [f64::NAN; 2],
            wear: f64::NAN,
            length: f64::NAN,
        }
    }
}

/// 塞规磨损方向为中径减小；止端的两种名义口径分别保留，实际界限不混淆。
pub(super) fn plug_set(
    major: f64,
    low: f64,
    high: f64,
    minor: f64,
    f: Factors,
    trap: bool,
    truncation: f64,
) -> Vec<Gauge> {
    let mut go = Gauge::new("T", "通端工作塞规", low + f.zpl, f.tpl);
    go.major = [major + f.zpl - f.tpl, major + f.zpl + f.tpl];
    go.minor_max = minor;
    go.wear = go.pd[0] - f.sgo;
    let center = high + if trap { f.tpl / 2. } else { f.tpl };
    let mut ng = Gauge::new("Z", "止端工作塞规", center, f.tpl);
    ng.pd = [high, high + f.tpl];
    let dc = if trap {
        center + truncation
    } else {
        high + f.tpl + truncation + f.tpl / 2.
    };
    ng.major = if trap {
        [dc - f.tpl, dc + f.tpl]
    } else {
        [dc - f.tpl, dc]
    };
    ng.minor_max = minor;
    ng.wear = ng.pd[0] - f.sng;
    vec![go, ng]
}

/// 环规磨损方向为中径增大；六种校对规的中径与环规上下限、磨损极限配套。
pub(super) fn ring_set(
    d: f64,
    d2: f64,
    d1: f64,
    es: f64,
    td: f64,
    f: Factors,
    truncation: f64,
) -> (Vec<Gauge>, Vec<Gauge>) {
    let mut go = Gauge::new("T环", "通端工作环规", d2 + es - f.zr, f.tr);
    go.wear = go.center + f.rgo;
    go.major_min = d + es + f.tpl;
    go.minor = [d1 + es - f.tr / 2., d1 + es + f.tr / 2.];
    let mut ng = Gauge::new("Z环", "止端工作环规", d2 + es - td - f.tr / 2., f.tr);
    ng.wear = ng.center + f.rng;
    ng.major_min = d + es + f.tpl;
    ng.minor = [ng.center - truncation - f.tr, ng.center - truncation + f.tr];
    let specs = [
        (
            "TT",
            "校通-通",
            go.center - f.m,
            d + es,
            f.tpl,
            d1 + es - f.zr - f.m,
        ),
        (
            "TZ",
            "校通-止",
            go.pd[1],
            go.pd[1] + truncation,
            f.tpl / 2.,
            d1 + es - f.tr / 2.,
        ),
        (
            "TS",
            "校通-损",
            go.wear,
            go.wear + truncation,
            f.tpl / 2.,
            d1 + es - f.tr / 2.,
        ),
        (
            "ZT",
            "校止-通",
            ng.center - f.m,
            d + es,
            f.tpl,
            d1 + es - td - f.tr / 2. - f.m,
        ),
        (
            "ZZ",
            "校止-止",
            d2 + es - td,
            d + es - td,
            f.tpl,
            d1 + es - td,
        ),
        (
            "ZS",
            "校止-损",
            ng.wear,
            d + es - td - f.tr / 2. + f.rng,
            f.tpl,
            d1 + es - td,
        ),
    ];
    let calibration = specs
        .into_iter()
        .map(|(code, name, pd, major, tolerance, minor)| {
            let mut g = Gauge::new(code, name, pd, f.tcp);
            g.major = [major - tolerance, major + tolerance];
            g.minor_max = minor;
            g
        })
        .collect();
    (vec![go, ng], calibration)
}

pub(super) fn gauge_table(title: &str, gauges: &[Gauge]) -> Table {
    let mut table = Table::new(
        title,
        &[
            "代号 / 名称",
            "中径下限 (mm)",
            "中径上限 (mm)",
            "中径名义 (mm)",
            "大径下限 (mm)",
            "大径上限 (mm)",
            "小径界限 (mm)",
            "磨损极限 (mm)",
            "螺纹长度 (mm)",
        ],
    );
    for g in gauges {
        let minor = if valid(g.minor_max) {
            Cell::limit(g.minor_max, "≤")
        } else if same(g.minor[0], g.minor[1]) {
            Cell::limit(g.minor[0], "≥")
        } else if valid(g.minor[0]) {
            let a = Cell::from(g.minor[0]);
            let b = Cell::from(g.minor[1]);
            Cell {
                display: format!("{} ～ {}", a.display, b.display),
                raw: format!("{} ～ {}", a.raw, b.raw),
            }
        } else {
            "—".into()
        };
        table.row(vec![
            format!("{} · {}", g.code, g.name).into(),
            g.pd[0].into(),
            g.pd[1].into(),
            g.center.into(),
            if valid(g.major_min) {
                Cell::limit(g.major_min, "≥")
            } else {
                g.major[0].into()
            },
            g.major[1].into(),
            minor,
            g.wear.into(),
            g.length.into(),
        ]);
    }
    table
}

pub(super) fn factor_metrics(out: &mut Report, f: Factors, ring: bool) {
    out.metric("工作塞规公差 TPL (μm)", f.tpl * 1000.);
    out.metric("塞规位置 ZPL (μm)", f.zpl * 1000.);
    if ring {
        out.metric("环规公差 TR (μm)", f.tr * 1000.);
        out.metric("校对规公差 TCP (μm)", f.tcp * 1000.);
        out.metric("环规位置 ZR (μm)", f.zr * 1000.);
        out.metric("校对位置 m (μm)", f.m * 1000.);
        out.metric(
            "环规磨损 WGO / WNG (μm)",
            Cell::pair(f.rgo * 1000., f.rng * 1000.),
        );
    }
    out.metric(
        "塞规磨损 WGO / WNG (μm)",
        Cell::pair(f.sgo * 1000., f.sng * 1000.),
    );
}

pub(super) fn half_metrics(out: &mut Report, trap: bool, p: f64) {
    let table = &database()["DATA"][if trap { "trap" } else { "metric" }]["half"];
    if let Some(r) = closest_pitch(table, p) {
        out.metric(
            "完整牙型半角偏差 (角分)",
            first(r, &["完整螺纹牙型的半角偏差", "完整牙型偏差"]),
        );
        out.metric(
            "截断牙型半角偏差 (角分)",
            first(r, &["截断螺纹牙型的半角偏差", "截断牙型偏差"]),
        );
    }
}

pub(super) fn minor_table(out: &mut Report, base: f64, lo: f64, hi: f64, tolerance: Option<f64>) {
    reference_table(out, "小径", base, lo, hi, tolerance);
}

/// 工件直径公差与量规制造公差分别列示；缺失公差时不伪造零宽度的上下限。
fn reference_table(
    out: &mut Report,
    diameter: &str,
    base: f64,
    lo: f64,
    hi: f64,
    tolerance: Option<f64>,
) {
    let mut t = Table::new(
        &format!("{diameter}参考界限（非独立制造公差）"),
        &[
            "基本尺寸 (mm)",
            "参考下界 (mm)",
            "参考上界 (mm)",
            &format!("采用{diameter}公差 (μm)"),
        ],
    );
    t.row(vec![
        base.into(),
        tolerance.map(|_| lo).unwrap_or(f64::NAN).into(),
        tolerance.map(|_| hi).unwrap_or(f64::NAN).into(),
        tolerance.unwrap_or(f64::NAN).into(),
    ]);
    out.tables.push(t);
    if tolerance.is_none() {
        out.note(format!(
            "该组合的{diameter}公差未收录，只列基本尺寸，不据此判定{diameter}合格。"
        ));
    }
}

pub(super) fn grades(trap: bool, d: f64, p: f64) -> Vec<String> {
    let mut result = Vec::new();
    let db = &database()["DATA"][if trap { "trap" } else { "metric" }];
    for external in [false, true] {
        let numbers: &[usize] = if trap {
            &[7, 8, 9]
        } else if external {
            &[3, 4, 5, 6, 7, 8, 9]
        } else {
            &[4, 5, 6, 7, 8]
        };
        let letters: &[&str] = if trap {
            if external { &["c", "e", "h"] } else { &["H"] }
        } else if external {
            &["e", "f", "g", "h"]
        } else {
            &["H", "G"]
        };
        for num in numbers {
            for letter in letters {
                // 普通 M 的 G 内螺纹从 5 级起列出，与公差带选择规则一致。
                if !trap && !external && *num < 5 && *letter == "G" {
                    continue;
                }
                if let Ok((_, used)) = tolerance(
                    &db[if external { "td2o" } else { "td2" }],
                    d,
                    p,
                    &num.to_string(),
                    &mut Vec::new(),
                ) {
                    let key = if trap {
                        format!("es({letter})")
                    } else {
                        format!("外螺纹{letter}-es")
                    };
                    if !external || deviation(&db["ei"], used, &key).is_ok() {
                        result.push(format!("{num}{letter}"));
                    }
                }
            }
        }
    }
    result
}

/// 按内外螺纹及所选公差带计算工作量规、小径参考界限和校对规。
pub(super) fn calculate(
    trap: bool,
    d: f64,
    p: f64,
    grade: &str,
    lines: usize,
) -> Result<Report, String> {
    if !grades(trap, d, p).iter().any(|s| s == grade) {
        return Err("当前直径、螺距没有该公差带的定义".into());
    }
    let angle = if trap { 30. } else { 60. };
    // 基本尺寸及后续量规界限均保留 f64 精度，只在输出层统一显示五位小数。
    let d2 = d - if trap { 0.5 } else { 3. * 3_f64.sqrt() / 8. } * p;
    let d1 = d - if trap { 1. } else { 5. * 3_f64.sqrt() / 8. } * p;
    positive_geometry(d, p, d2, d1)?;
    if !(1..=5).contains(&lines) || (!trap && lines != 1) {
        return Err("线数范围不正确".into());
    }
    let (num, letter) = grade.split_at(grade.len() - 1);
    let ring = letter.chars().all(char::is_lowercase);
    let db = &database()["DATA"][if trap { "trap" } else { "metric" }];
    let mut out = Report::new(format!(
        "{}{}×{} - {grade}",
        if trap { "Tr" } else { "M" },
        d,
        p
    ));
    geometry(&mut out, d, p, d2, d1, angle, lines);
    let (td, used) = tolerance(
        &db[if ring { "td2o" } else { "td2" }],
        d,
        p,
        num,
        &mut out.notes,
    )?;
    let kv = if trap {
        [1., 1.12, 1.25, 1.4, 1.6][lines - 1]
    } else {
        1.
    };
    let td = td * kv;
    let offset = if ring {
        deviation(
            &db["ei"],
            used,
            &if trap {
                format!("es({letter})")
            } else {
                format!("外螺纹{letter}-es")
            },
        )?
    } else if letter == "H" {
        0.
    } else {
        deviation(&db["ei"], used, "内螺纹G-EI")?
    };
    let f = factors(trap, td, offset < 0.)?;
    out.metric("工件中径公差 (μm)", td);
    out.metric(
        if ring {
            "基本偏差 es (μm)"
        } else {
            "基本偏差 EI (μm)"
        },
        offset,
    );
    out.parameter("公差取值螺距 (mm)", used);
    if trap {
        out.metric("多线公差系数", kv);
    }
    factor_metrics(&mut out, f, ring);
    half_metrics(&mut out, trap, used);
    let ac = if p <= 1.5 {
        0.15
    } else if p <= 5. {
        0.25
    } else if p <= 12. {
        0.5
    } else {
        1.
    };
    if trap {
        out.metric("牙顶间隙 ac (mm)", ac);
        out.metric("内螺纹基本大径 D₄ (mm)", d + 2. * ac);
    }
    let truncation = if trap { 2. * ac } else { 0.2 * p };
    if ring {
        let (mut work, cal) = ring_set(d, d2, d1, offset / 1000., td / 1000., f, truncation);
        let (lg, ln) = lengths(trap, true, d, p, &mut out.notes);
        work[0].length = lg;
        work[1].length = ln;
        out.tables.push(gauge_table("工作环规 · 检外螺纹", &work));
        out.tables.push(gauge_table("校对塞规 · 检工作环规", &cal));
        if trap {
            // 外螺纹小径 d₃ 的基本偏差恒为零，不能叠加中径的 es，也不能向基本尺寸以上加公差。
            let base = d - p - 2. * ac;
            let minor = if lines == 1 {
                trapezoidal_minor_tolerance(d, used, num, letter, &mut out.notes)
            } else {
                out.note("未收录多线外螺纹小径公差修正规则，不沿用单线小径极限。");
                None
            };
            minor_table(
                &mut out,
                base,
                base - minor.unwrap_or(0.) / 1000.,
                base,
                minor,
            );
        } else {
            // td 是外螺纹大径公差；不能将这张表误用于工件小径。
            let major_tolerance = minor_tolerance(&db["td"], used, num);
            let upper = d + offset / 1000.;
            reference_table(
                &mut out,
                "大径",
                d,
                upper - major_tolerance.unwrap_or(0.) / 1000.,
                upper,
                major_tolerance,
            );
            out.note("未收录普通外螺纹小径的独立极限规则，不使用大径公差推算小径。");
        }
    } else {
        let minor = d1 + offset / 1000.;
        let mut work = plug_set(
            if trap {
                d + 2. * ac
            } else {
                d + offset / 1000.
            },
            d2 + offset / 1000.,
            d2 + offset / 1000. + td / 1000.,
            minor,
            f,
            trap,
            truncation,
        );
        let (lg, ln) = if trap {
            lengths(true, false, d, p, &mut out.notes)
        } else {
            let (a, b, total, handle) = metric_lengths(d, p, &mut out.notes);
            out.metric("塞规总长 (mm)", total);
            out.metric("配套手柄", handle);
            (a, b)
        };
        work[0].length = lg;
        work[1].length = ln;
        out.tables.push(gauge_table("工作塞规 · 检内螺纹", &work));
        // Tr 内螺纹小径固定采用 4 级，与中径的 7/8/9 级分开。
        let td1 = minor_tolerance(&db["td1"], used, if trap { "4" } else { num });
        let base = if trap { d1 } else { d1 + offset / 1000. };
        minor_table(&mut out, d1, base, base + td1.unwrap_or(0.) / 1000., td1);
    }
    out.notes.dedup();
    out.checked()
}

/// 钢筋尺寸直接查表，借用工件公差与量规要素；不把 75° 的几何替换成公制 60°。
pub(super) fn rebar(angle: usize, index: usize, lines: usize) -> Result<Report, String> {
    let row = rows(&database()["REBAR"][angle.to_string()])
        .get(index)
        .ok_or("请选择有效钢筋规格")?;
    if !(1..=4).contains(&lines) {
        return Err("钢筋头数须为 1～4".into());
    }
    let (p, d, d2, d1) = (
        number(&row[2]),
        number(&row[3]),
        number(&row[4]),
        number(&row[5]),
    );
    let mut out = Report::new(format!(
        "钢筋直螺纹 {} · {} · {angle}°",
        row[0].as_str().unwrap_or(""),
        row[1].as_str().unwrap_or("")
    ));
    geometry(&mut out, d, p, d2, d1, angle as f64, lines);
    let db = &database()["DATA"]["metric"];
    let (ti, pi) = tolerance(&db["td2"], d, p, "6", &mut out.notes)?;
    let (te, pe) = tolerance(&db["td2o"], d, p, "6", &mut out.notes)?;
    let es = deviation(&db["ei"], pe, "外螺纹f-es")? / 1000.;
    let fi = factors(false, ti, false)?;
    let fe = factors(false, te, true)?;
    out.metric("套筒 6H 中径公差 (μm)", ti);
    out.metric("丝头 6f 中径公差 (μm)", te);
    out.metric("丝头 es (μm)", es * 1000.);
    let first_factor = out.metrics.len();
    factor_metrics(&mut out, fi, false);
    for (label, _) in &mut out.metrics[first_factor..] {
        *label = format!("套筒塞规 · {label}");
    }
    let ring_factor = out.metrics.len();
    factor_metrics(&mut out, fe, true);
    for (label, _) in &mut out.metrics[ring_factor..] {
        *label = format!("丝头环规 / 校对规 · {label}");
    }
    let plugs = plug_set(d, d2, d2 + ti / 1000., d1, fi, false, 0.2 * p);
    let (rings, mut cal) = ring_set(d, d2, d1, es, te / 1000., fe, 0.2 * p);
    // 钢筋尺寸表的 ZZ/ZS 大径以工件中径为基准，不沿用普通 M 的大径基准。
    let zz_major = d2 + es - te / 1000.;
    cal[4].major = [zz_major - fe.tpl, zz_major + fe.tpl];
    let zs_major = zz_major - fe.tr / 2. + fe.rng;
    cal[5].major = [zs_major - fe.tpl, zs_major + fe.tpl];
    out.tables
        .push(gauge_table("套筒内螺纹 6H · 工作塞规", &plugs));
    out.tables
        .push(gauge_table("丝头外螺纹 6f · 工作环规", &rings));
    out.tables.push(gauge_table("校对塞规", &cal));
    let td1 = minor_tolerance(&db["td1"], pi, "6");
    minor_table(&mut out, d1, d1, d1 + td1.unwrap_or(0.) / 1000., td1);
    out.note("工件公差和量规位置要素为借用体系；多头不另增公差系数。具体旋合判定按产品实际执行标准。复核牙型与标准后使用。");
    out.checked()
}
