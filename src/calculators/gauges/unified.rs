//! 美制统一螺纹：按工件等级自动计算中径极限，再生成对应工作量规与校对规。
use super::{
    imperial::{INCH, band, un_diameter, un_series_class},
    lookup::*,
    report::*,
    thread::{self, Gauge},
    un_data,
};
mod product;
use product::ThreadClass;

/// UN 的工件极限由等级模型生成，不接受手填中径或大径上下限。
pub(super) struct Input<'a> {
    pub size: &'a str,
    pub tpi: f64,
    pub class: &'a str,
}

pub(super) fn calculate(input: &Input<'_>) -> Result<Report, String> {
    if !input.tpi.is_finite() || input.tpi <= 0. {
        return Err("TPI 须大于零".into());
    }
    let d_in = un_diameter(input.size)?;
    let class = ThreadClass::parse(input.class)?;
    let series = un_series_class(input.size, input.tpi);
    let product = product::calculate(d_in, input.tpi, series.as_deref(), class)?;
    let d = d_in * INCH;
    let p = INCH / input.tpi;
    let d2 = product.basic_pitch_mm;
    let d1 = d - 5. * 3_f64.sqrt() / 8. * p;
    let root = d - 17. * 3_f64.sqrt() / 24. * p;
    positive_geometry(d, p, d2, root)?;
    let (lo, hi) = (product.lower_mm, product.upper_mm);
    let major = product.external_major_max_mm;
    let db = &database()["DATA"]["un"];
    let mut out = Report::new(format!(
        "UN {}-{} {}-{}",
        input.size.trim(),
        input.tpi,
        series.as_deref().unwrap_or("非标"),
        input.class,
    ));
    // 自动生成的工件极限置于摘要区，基本几何与查表明细随后列出。
    out.metric("工件公差等级", input.class);
    out.metric("工件中径下限 (mm)", lo);
    out.metric("工件中径上限 (mm)", hi);
    out.metric("工件中径公差 (mm)", product.tolerance_mm);
    out.metric("工件容隙 (mm)", product.allowance_mm);
    out.metric("旋合长度 Le (mm)", product.engagement_mm);
    geometry(&mut out, d, p, d2, d1, 60., 1);
    if class.external() {
        out.metric("基本外螺纹小径 (mm)", root);
    }
    out.parameter("公称直径 (inch)", d_in);
    out.parameter("TPI (牙/inch)", input.tpi);
    out.metric("螺纹系列", series.as_deref().unwrap_or("非标"));
    out.metric("2A 基础中径公差 (mm)", product.base_tolerance_mm);
    if class.external() {
        out.metric("工件大径上限 (mm)", major);
    }
    // 工作量规中径统一采用 U；补充的顶径/校对公差不反向改变工件或工作量规中径。
    let u = un_data::manufacturing_tolerance_mm(d_in, input.tpi)?;
    let x = band(&db["am"]["x"], d_in, input.tpi);
    let w = band(&db["am"]["w"], d_in, input.tpi);
    let optional_mm = |row: Option<&serde_json::Value>, key: &str| {
        row.map(|r| n(r, key))
            .filter(|v| valid(*v) && *v > 0.)
            .map(|v| v * INCH)
            .unwrap_or(f64::NAN)
    };
    let ta = optional_mm(x, "顶径公差");
    let tw = optional_mm(w, "中径公差");
    out.metric("中径制造公差 U (mm)", u);
    out.metric("顶径公差 Ta · 参考 (mm)", ta);
    out.note("工作量规中径按 U 计算；大径、小径及校对尺寸供参考。");
    if !class.external() {
        // 模板方向：通端 [工件下限, 下限+U]；止端 [上限-U, 工件上限]。
        let mut go = Gauge::new("GO", "通端工作塞规", lo + u / 2., u);
        go.major = [d - ta, d];
        go.minor_max = d1;
        let mut ng = Gauge::new("NO GO", "止端工作塞规", hi - u / 2., u);
        // 止端已采用负公差，其大径参考基准也必须使用当前中径上限。
        let ng_major = ng.pd[1] + 0.2 * p + u / 2.;
        ng.major = [ng_major - ta, ng_major];
        ng.minor_max = d1;
        out.tables
            .push(gauge_table("工作塞规 · 检内螺纹", &[go, ng]));
        out.show_inches();
        return out.checked();
    }
    out.metric("校对中径公差 · 参考 (mm)", tw);
    // 模板方向：通端 [工件上限-U, 上限]；止端 [工件下限, 下限+U]。
    let mut rg = Gauge::new("GO", "通端工作环规", hi - u / 2., u);
    rg.major_min = major;
    rg.minor = [root, root];
    let mut rn = Gauge::new("NO GO", "止端工作环规", lo + u / 2., u);
    rn.major_min = major;
    rn.minor = [rn.center - 0.2 * p, rn.center - 0.2 * p];
    out.tables.push(gauge_table(
        "工作环规 · 检外螺纹",
        &[rg.clone(), rn.clone()],
    ));
    let specs = [
        ("TT", "校通-通", rg.pd[0], major, u),
        ("TZ", "校通-止", rg.pd[1], rg.pd[1] + 0.2 * p, u / 2.),
        ("ZT", "校止-通", rn.pd[0], major, u),
        ("ZZ", "校止-止", rn.pd[1], lo, u),
    ];
    let cal: Vec<_> = specs
        .into_iter()
        .map(|(code, name, up, maj, half)| {
            let mut g = Gauge::new(code, name, up - tw / 2., tw);
            g.major = [maj - half, maj + half];
            g.minor_max = root;
            g
        })
        .collect();
    if !valid(tw) {
        out.note("该规格未收录校对中径公差，相应界限不计算。");
    }
    out.tables.push(gauge_table("校对塞规 · 参考", &cal));
    out.show_inches();
    out.checked()
}

/// UN 未收录独立磨损储备；在报告数据中移除空列，使界面与复制结果保持一致。
fn gauge_table(title: &str, gauges: &[Gauge]) -> Table {
    let mut table = thread::gauge_table(title, gauges);
    if let Some(column) = table.headings.iter().position(|h| h == "磨损极限 (mm)") {
        table.headings.remove(column);
        for row in &mut table.rows {
            row.remove(column);
        }
    }
    table
}
