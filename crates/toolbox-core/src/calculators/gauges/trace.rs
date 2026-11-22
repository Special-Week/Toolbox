//! 工作量规与校对规的公式说明；结果直接取自同一次模型运算。
use super::{
    lookup::{Factors, valid},
    report::Report,
    thread::Gauge,
};
use crate::calculators::common::format_process_computed as c;

#[allow(clippy::too_many_arguments)]
pub fn plugs(
    out: &mut Report,
    gauges: &[Gauge],
    major: f64,
    low: f64,
    high: f64,
    minor: f64,
    f: Factors,
    trap: bool,
    truncation: f64,
) {
    let (go, stop) = (&gauges[0], &gauges[1]);
    out.trace(String::new());
    out.trace(format!(
        "通端工作塞规：中径名义 = 工件中径下限 + ZPL = {} + {} = {} mm。",
        c(low),
        c(f.zpl),
        c(go.center)
    ));
    out.trace(format!("中径下限 = 名义 − TPL/2 = {} − {}/2 = {} mm；中径上限 = 名义 + TPL/2 = {} + {}/2 = {} mm。", c(go.center), c(f.tpl), c(go.pd[0]), c(go.center), c(f.tpl), c(go.pd[1])));
    out.trace(format!("大径下限 = 大径基准 + ZPL − TPL = {} + {} − {} = {} mm；大径上限 = 大径基准 + ZPL + TPL = {} + {} + {} = {} mm。", c(major), c(f.zpl), c(f.tpl), c(go.major[0]), c(major), c(f.zpl), c(f.tpl), c(go.major[1])));
    out.trace(format!(
        "小径上限 = 小径基准 = {} mm；磨损极限 = 通端中径下限 − 通端磨损储备 = {} − {} = {} mm。",
        c(minor),
        c(go.pd[0]),
        c(f.sgo),
        c(go.wear)
    ));
    out.trace(String::new());
    out.trace(format!("止端工作塞规：中径下限 = 工件中径上限 = {} mm；中径上限 = 工件中径上限 + TPL = {} + {} = {} mm。", c(high), c(high), c(f.tpl), c(stop.pd[1])));
    if trap {
        out.trace(format!("止端中径名义 = 工件中径上限 + TPL/2 = {} + {}/2 = {} mm；大径基准 = 中径名义 + 截短量 = {} + {} = {} mm。", c(high), c(f.tpl), c(stop.center), c(stop.center), c(truncation), c(stop.center + truncation)));
        out.trace(format!(
            "大径下限/上限 = 大径基准 ± TPL = {} ± {} = {}～{} mm。",
            c(stop.center + truncation),
            c(f.tpl),
            c(stop.major[0]),
            c(stop.major[1])
        ));
    } else {
        let dc = high + f.tpl + truncation + f.tpl / 2.0;
        out.trace(format!(
            "止端中径名义按本模型取工件上限 + TPL = {} + {} = {} mm；该名义值不是上下限中点。",
            c(high),
            c(f.tpl),
            c(stop.center)
        ));
        out.trace(format!("大径上限 = 工件上限 + TPL + 截短量 + TPL/2 = {} + {} + {} + {}/2 = {} mm；大径下限 = 大径上限 − TPL = {} − {} = {} mm。", c(high), c(f.tpl), c(truncation), c(f.tpl), c(stop.major[1]), c(dc), c(f.tpl), c(stop.major[0])));
    }
    out.trace(format!(
        "小径上限 = 小径基准 = {} mm；磨损极限 = 止端中径下限 − 止端磨损储备 = {} − {} = {} mm。",
        c(minor),
        c(stop.pd[0]),
        c(f.sng),
        c(stop.wear)
    ));
    if valid(go.length) && valid(stop.length) {
        out.trace(format!(
            "螺纹长度：按规格查表，通端 = {} mm，止端 = {} mm。",
            c(go.length),
            c(stop.length)
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn rings(
    out: &mut Report,
    work: &[Gauge],
    calibration: &[Gauge],
    d: f64,
    d2: f64,
    d1: f64,
    es: f64,
    td: f64,
    f: Factors,
    truncation: f64,
    rebar: bool,
) {
    let (go, stop) = (&work[0], &work[1]);
    out.trace(String::new());
    out.trace(format!(
        "通端工作环规：中径名义 = D₂ + es − ZR = {} + {} − {} = {} mm。",
        c(d2),
        c(es),
        c(f.zr),
        c(go.center)
    ));
    ring_limits(out, go, f.tr, f.rgo);
    out.trace(format!("大径下限 = D + es + TPL = {} + {} + {} = {} mm；小径界限 = D₁ + es ± TR/2 = {} + {} ± {}/2 = {}～{} mm。", c(d), c(es), c(f.tpl), c(go.major_min), c(d1), c(es), c(f.tr), c(go.minor[0]), c(go.minor[1])));
    out.trace(String::new());
    out.trace(format!(
        "止端工作环规：中径名义 = D₂ + es − Td − TR/2 = {} + {} − {} − {}/2 = {} mm。",
        c(d2),
        c(es),
        c(td),
        c(f.tr),
        c(stop.center)
    ));
    ring_limits(out, stop, f.tr, f.rng);
    out.trace(format!("大径下限 = D + es + TPL = {} + {} + {} = {} mm；小径界限 = 止端中径名义 − 截短量 ± TR = {} − {} ± {} = {}～{} mm。", c(d), c(es), c(f.tpl), c(stop.major_min), c(stop.center), c(truncation), c(f.tr), c(stop.minor[0]), c(stop.minor[1])));
    if valid(go.length) && valid(stop.length) {
        out.trace(format!(
            "螺纹长度：按规格查表，通端 = {} mm，止端 = {} mm。",
            c(go.length),
            c(stop.length)
        ));
    }
    let operands = [
        (
            format!("通端中径名义 − m = {} − {}", c(go.center), c(f.m)),
            format!("D + es = {} + {}", c(d), c(es)),
            f.tpl,
            format!(
                "D₁ + es − ZR − m = {} + {} − {} − {}",
                c(d1),
                c(es),
                c(f.zr),
                c(f.m)
            ),
        ),
        (
            format!("通端中径上限 = {}", c(go.pd[1])),
            format!(
                "通端中径上限 + 截短量 = {} + {}",
                c(go.pd[1]),
                c(truncation)
            ),
            f.tpl / 2.0,
            format!("D₁ + es − TR/2 = {} + {} − {}/2", c(d1), c(es), c(f.tr)),
        ),
        (
            format!("通端磨损极限 = {}", c(go.wear)),
            format!("通端磨损极限 + 截短量 = {} + {}", c(go.wear), c(truncation)),
            f.tpl / 2.0,
            format!("D₁ + es − TR/2 = {} + {} − {}/2", c(d1), c(es), c(f.tr)),
        ),
        (
            format!("止端中径名义 − m = {} − {}", c(stop.center), c(f.m)),
            format!("D + es = {} + {}", c(d), c(es)),
            f.tpl,
            format!(
                "D₁ + es − Td − TR/2 − m = {} + {} − {} − {}/2 − {}",
                c(d1),
                c(es),
                c(td),
                c(f.tr),
                c(f.m)
            ),
        ),
        (
            format!("D₂ + es − Td = {} + {} − {}", c(d2), c(es), c(td)),
            if rebar {
                format!("D₂ + es − Td = {} + {} − {}", c(d2), c(es), c(td))
            } else {
                format!("D + es − Td = {} + {} − {}", c(d), c(es), c(td))
            },
            f.tpl,
            format!("D₁ + es − Td = {} + {} − {}", c(d1), c(es), c(td)),
        ),
        (
            format!("止端磨损极限 = {}", c(stop.wear)),
            if rebar {
                format!(
                    "D₂ + es − Td − TR/2 + 止端磨损储备 = {} + {} − {} − {}/2 + {}",
                    c(d2),
                    c(es),
                    c(td),
                    c(f.tr),
                    c(f.rng)
                )
            } else {
                format!(
                    "D + es − Td − TR/2 + 止端磨损储备 = {} + {} − {} − {}/2 + {}",
                    c(d),
                    c(es),
                    c(td),
                    c(f.tr),
                    c(f.rng)
                )
            },
            f.tpl,
            format!("D₁ + es − Td = {} + {} − {}", c(d1), c(es), c(td)),
        ),
    ];
    for (g, (center, major, tolerance, minor)) in calibration.iter().zip(operands) {
        out.trace(String::new());
        out.trace(format!(
            "{} {}：中径名义 = {} = {} mm；中径下限/上限 = 名义 ± TCP/2 = {} ± {}/2 = {}～{} mm。",
            g.code,
            g.name,
            center,
            c(g.center),
            c(g.center),
            c(f.tcp),
            c(g.pd[0]),
            c(g.pd[1])
        ));
        out.trace(format!(
            "大径基准 = {}；大径下限/上限 = 大径基准 ± {} = {}～{} mm；小径上限 = {} = {} mm。",
            major,
            c(tolerance),
            c(g.major[0]),
            c(g.major[1]),
            minor,
            c(g.minor_max)
        ));
    }
}

fn ring_limits(out: &mut Report, gauge: &Gauge, tolerance: f64, wear: f64) {
    out.trace(format!("中径下限/上限 = 名义 ± TR/2 = {} ± {}/2 = {}～{} mm；磨损极限 = 名义 + 磨损储备 = {} + {} = {} mm。", c(gauge.center), c(tolerance), c(gauge.pd[0]), c(gauge.pd[1]), c(gauge.center), c(wear), c(gauge.wear)));
}

#[allow(clippy::too_many_arguments)]
pub fn unified_plugs(
    out: &mut Report,
    go: &Gauge,
    stop: &Gauge,
    d: f64,
    d1: f64,
    low: f64,
    high: f64,
    u: f64,
    ta: f64,
    pitch: f64,
) {
    out.trace(String::new());
    out.trace(format!("通端工作塞规：中径名义 = 工件下限 + U/2 = {} + {}/2 = {} mm；中径下限/上限 = 名义 ± U/2 = {} ± {}/2 = {}～{} mm。", c(low), c(u), c(go.center), c(go.center), c(u), c(go.pd[0]), c(go.pd[1])));
    if ta.is_finite() {
        out.trace(format!(
            "通端大径下限 = D − Ta = {} − {} = {} mm；上限 = D = {} mm；小径上限 = D₁ = {} mm。",
            c(d),
            c(ta),
            c(go.major[0]),
            c(go.major[1]),
            c(d1)
        ));
    } else {
        out.trace("通端大径公差未收录，下限不计算；大径上限为 D，小径上限为 D₁。".to_owned());
    }
    out.trace(String::new());
    out.trace(format!("止端工作塞规：中径名义 = 工件上限 − U/2 = {} − {}/2 = {} mm；中径下限/上限 = 名义 ± U/2 = {} ± {}/2 = {}～{} mm。", c(high), c(u), c(stop.center), c(stop.center), c(u), c(stop.pd[0]), c(stop.pd[1])));
    if ta.is_finite() {
        out.trace(format!("止端大径上限 = 止端中径上限 + 0.2P + U/2 = {} + 0.2 × {} + {}/2 = {} mm；下限 = 大径上限 − Ta = {} − {} = {} mm；小径上限 = D₁ = {} mm。", c(stop.pd[1]), c(pitch), c(u), c(stop.major[1]), c(stop.major[1]), c(ta), c(stop.major[0]), c(d1)));
    } else {
        out.trace(
            "止端大径公差未收录，下限不计算；大径上限由止端中径上限 + 0.2P + U/2 计算。".to_owned(),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn unified_rings(
    out: &mut Report,
    go: &Gauge,
    stop: &Gauge,
    calibration: &[Gauge],
    low: f64,
    high: f64,
    major: f64,
    root: f64,
    u: f64,
    tw: f64,
    pitch: f64,
) {
    out.trace(String::new());
    out.trace(format!("通端工作环规：中径名义 = 工件上限 − U/2 = {} − {}/2 = {} mm；中径下限/上限 = 名义 ± U/2 = {} ± {}/2 = {}～{} mm。", c(high), c(u), c(go.center), c(go.center), c(u), c(go.pd[0]), c(go.pd[1])));
    out.trace(format!(
        "通端大径下限 = 工件大径上限 = {} mm；小径下限 = 外螺纹基本小径 = {} mm。",
        c(major),
        c(root)
    ));
    out.trace(String::new());
    out.trace(format!("止端工作环规：中径名义 = 工件下限 + U/2 = {} + {}/2 = {} mm；中径下限/上限 = 名义 ± U/2 = {} ± {}/2 = {}～{} mm。", c(low), c(u), c(stop.center), c(stop.center), c(u), c(stop.pd[0]), c(stop.pd[1])));
    out.trace(format!("止端大径下限 = 工件大径上限 = {} mm；小径下限 = 止端中径名义 − 0.2P = {} − 0.2 × {} = {} mm。", c(major), c(stop.center), c(pitch), c(stop.minor[0])));
    let refs = [
        (
            go.pd[0],
            "通端环规中径下限",
            major,
            format!("工件大径上限 = {}", c(major)),
            u,
        ),
        (
            go.pd[1],
            "通端环规中径上限",
            go.pd[1] + 0.2 * pitch,
            format!(
                "通端环规中径上限 + 0.2P = {} + 0.2 × {}",
                c(go.pd[1]),
                c(pitch)
            ),
            u / 2.0,
        ),
        (
            stop.pd[0],
            "止端环规中径下限",
            major,
            format!("工件大径上限 = {}", c(major)),
            u,
        ),
        (
            stop.pd[1],
            "止端环规中径上限",
            low,
            format!("工件中径下限 = {}", c(low)),
            u,
        ),
    ];
    for (g, (upper, source, base, expression, half)) in calibration.iter().zip(refs) {
        out.trace(String::new());
        if tw.is_finite() {
            out.trace(format!("{} {}：中径名义 = {} − 校对公差/2 = {} − {}/2 = {} mm；中径下限/上限 = 名义 ± 校对公差/2 = {} ± {}/2 = {}～{} mm。", g.code, g.name, source, c(upper), c(tw), c(g.center), c(g.center), c(tw), c(g.pd[0]), c(g.pd[1])));
        } else {
            out.trace(format!(
                "{} {}：校对中径公差未收录，中径名义和上下限不计算。",
                g.code, g.name
            ));
        }
        out.trace(format!("大径基准 = {} = {} mm；大径下限/上限 = 基准 ± {} = {}～{} mm；小径上限 = 外螺纹基本小径 = {} mm。", expression, c(base), c(half), c(g.major[0]), c(g.major[1]), c(root)));
    }
}
