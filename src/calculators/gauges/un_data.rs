//! UN 的规格系列和中径制造公差数据；数值保留原始单位，不在查表过程中修约。
use super::{imperial::INCH, lookup::*};
use serde_json::Value;
use std::sync::OnceLock;

pub(super) const SERIES: [&str; 11] = [
    "UNC", "UNF", "UNEF", "4UN", "6UN", "8UN", "12UN", "16UN", "20UN", "28UN", "32UN",
];

pub(super) fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../data/gauges/unified.json"))
            .expect("内置 UN 系列和制造公差表必须是有效 JSON")
    })
}

/// 塞规与环规使用同一制造公差 U。直径分档为下界开、上界闭；未定义时拒绝补值。
pub(super) fn manufacturing_tolerance_mm(diameter_in: f64, tpi: f64) -> Result<f64, String> {
    if !diameter_in.is_finite() || diameter_in <= 0. || !tpi.is_finite() || tpi <= 0. {
        return Err("公称直径与每英寸牙数须为有效正数".into());
    }
    let row = rows(&data()["manufacturing"])
        .iter()
        .find(|r| {
            same(n(r, "tpi"), tpi)
                && diameter_in > n(r, "diameter_lower_in")
                && diameter_in <= n(r, "diameter_upper_in")
        })
        .ok_or("未收录该牙数或直径档的中径制造公差 U")?;
    let tolerance = n(row, "tolerance_in");
    if !valid(tolerance) || tolerance <= 0. {
        return Err("该牙数与直径组合的中径制造公差 U 未定义，请核对规格".into());
    }
    Ok(tolerance * INCH)
}
