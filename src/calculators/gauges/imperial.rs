//! 美制统一螺纹共用的英寸尺寸解析、标准系列与量规公差分档查询。
use super::{lookup::*, un_data};
use serde_json::Value;

pub(super) const INCH: f64 = 25.4;

/// 分数和带分数严格解析，不接受部分可解析的尾随字符或零分母。
pub(super) fn fraction(text: &str) -> Result<f64, String> {
    let cleaned = text.trim().trim_end_matches(['″', '"']).replace('-', " ");
    if text.trim().starts_with('-') {
        return Err("公称直径须大于零".into());
    }
    if text.matches('-').count() > 1 {
        return Err("英寸带分数格式无效".into());
    }
    let parts: Vec<_> = cleaned.split_whitespace().collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err("英寸直径可填 0.5、1/2 或 1 1/4".into());
    }
    if parts.len() == 2
        && (!parts[0].chars().all(|c| c.is_ascii_digit()) || !parts[1].contains('/'))
    {
        return Err("带分数须为整数加真分数，例如 1 1/4".into());
    }
    let mut value = 0.;
    for part in &parts {
        let term = if let Some((a, b)) = part.split_once('/') {
            let a = a.parse::<f64>().map_err(|_| "分数分子无效")?;
            let b = b.parse::<f64>().map_err(|_| "分数分母无效")?;
            if a < 0. || b <= 0. {
                return Err("分数须为非负数且分母大于零".into());
            }
            if parts.len() == 2 && a >= b {
                return Err("带分数的小数部分须小于 1".into());
            }
            a / b
        } else {
            part.parse::<f64>().map_err(|_| "无法识别英寸直径")?
        };
        value += term;
    }
    if value.is_finite() && value > 0. && value < 1e5 {
        Ok(value)
    } else {
        Err("英寸直径超出有效范围".into())
    }
}
pub(super) fn un_diameter(size: &str) -> Result<f64, String> {
    // 编号规格兼容常见的 #8 与已有的 No.8 写法。
    let normalized = size
        .trim()
        .strip_prefix('#')
        .map(|digits| format!("No.{}", digits.trim()));
    let size = normalized.as_deref().unwrap_or(size.trim());
    rows(&un_data::data()["series"])
        .iter()
        .find(|r| {
            r["size"]
                .as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case(size.trim()))
        })
        .map(|r| Ok(n(r, "diameter_in")))
        .unwrap_or_else(|| fraction(size))
}
pub(super) fn un_sizes() -> Vec<String> {
    let mut list = Vec::new();
    for row in rows(&un_data::data()["series"]) {
        if let Some(s) = row["size"].as_str() {
            if !list.iter().any(|x| x == s) {
                list.push(s.into());
            }
        }
    }
    list
}
pub(super) fn un_series() -> Vec<(String, f64, f64, String)> {
    let mut list = Vec::new();
    for r in rows(&un_data::data()["series"]) {
        for variant in rows(&r["variants"]) {
            let tpi = n(variant, "tpi");
            if let Some(series) = variant["series"]
                .as_str()
                .filter(|_| valid(tpi) && tpi > 0.)
            {
                list.push((
                    r["size"].as_str().unwrap_or("").into(),
                    n(r, "diameter_in"),
                    tpi,
                    series.into(),
                ));
            }
        }
    }
    list
}

/// 按系列表查询直径对应的系列和牙数；等值的小数、分数及编号写法共用同一尺寸档。
pub(super) fn un_variants(size: &str) -> Vec<(String, f64)> {
    let Ok(diameter) = un_diameter(size) else {
        return Vec::new();
    };
    let entries = un_series();
    let mut result = Vec::new();
    // 与标准系列表一致，优先 UNC、UNF、UNEF，再列各恒定牙数系列。
    for class in un_data::SERIES {
        for (_, d, tpi, series) in &entries {
            if same(*d, diameter)
                && series == class
                && !result
                    .iter()
                    .any(|(name, value)| name == series && same(*value, *tpi))
            {
                result.push((series.clone(), *tpi));
            }
        }
    }
    result
}

/// 系列由“直径＋TPI”共同确定，不将非标准组合误标成 UNC 或 UNF。
pub(super) fn un_series_class(size: &str, tpi: f64) -> Option<String> {
    un_variants(size)
        .into_iter()
        .find(|(_, value)| same(*value, tpi))
        .map(|(series, _)| series)
}
pub(super) fn band(table: &Value, d_in: f64, tpi: f64) -> Option<&Value> {
    rows(table).iter().find(|r| {
        // 公差表包含 11-1/2、4-1/2 等带分数牙数，不能仅按浮点字符串读取。
        let teeth = n(r, "牙数");
        let teeth = if teeth.is_finite() {
            teeth
        } else {
            r["牙数"]
                .as_str()
                .and_then(|s| fraction(s).ok())
                .unwrap_or(f64::NAN)
        };
        same(teeth, tpi)
            && d_in > first(r, &["公称直径>", "公称直径＞"])
            && d_in <= n(r, "公称直径≤")
    })
}
