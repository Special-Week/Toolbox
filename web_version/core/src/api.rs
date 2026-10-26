//! 前端输入只在此转换，空白与非有限值必须报错，不能悄悄变为零。
use crate::calculators::{calibration::TemperatureUnit, format::parse_number};
use serde::Serialize;
use serde_json::{Value, json};

pub(crate) struct Input(pub Value);
impl Input {
    pub fn new(text: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(text).map_err(|_| "输入数据格式无效")?;
        if !v.is_object() {
            return Err("输入须为字段对象".into());
        }
        Ok(Self(v))
    }
    pub fn number(&self, key: &str, default: f64) -> Result<f64, String> {
        match self.0.get(key) {
            None => Ok(default),
            Some(Value::Number(v)) => v
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("{key} 不是有限数值")),
            Some(Value::String(v)) => parse_number(v).map_err(|e| format!("{key}：{e}")),
            _ => Err(format!("{key} 请填写数值")),
        }
    }
    pub fn index(&self, key: &str, default: usize, count: usize) -> Result<usize, String> {
        let v = self.number(key, default as f64)?;
        if v < 0.0 || v.fract() != 0.0 || v >= count as f64 {
            return Err(format!("{key} 选项无效"));
        }
        Ok(v as usize)
    }
    pub fn flag(&self, key: &str) -> bool {
        self.0.get(key).and_then(Value::as_bool).unwrap_or(false)
    }
    pub fn text<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.0.get(key).and_then(Value::as_str).unwrap_or(default)
    }
    pub fn values(
        &self,
        key: &str,
        default: &[f64],
        minimum: usize,
        maximum: usize,
    ) -> Result<Vec<f64>, String> {
        let values = match self.0.get(key) {
            None => default.to_vec(),
            Some(Value::Array(a)) => a.iter().map(value_number).collect::<Result<Vec<_>, _>>()?,
            _ => return Err(format!("{key} 数据格式无效")),
        };
        if values.len() < minimum || values.len() > maximum {
            return Err(format!("{key} 需要 {minimum}～{maximum} 项"));
        }
        Ok(values)
    }
    pub fn points(
        &self,
        default: &[(f64, f64)],
        minimum: usize,
        maximum: usize,
    ) -> Result<Vec<(f64, f64)>, String> {
        let points = match self.0.get("points") {
            None => default.to_vec(),
            Some(Value::Array(rows)) => rows
                .iter()
                .map(|row| {
                    let a = row
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .ok_or("每个数据点须有 x、y 两个数值")?;
                    Ok((value_number(&a[0])?, value_number(&a[1])?))
                })
                .collect::<Result<Vec<_>, String>>()?,
            _ => return Err("数据点格式无效".into()),
        };
        if points.len() < minimum || points.len() > maximum {
            return Err(format!("需要 {minimum}～{maximum} 个数据点"));
        }
        Ok(points)
    }
    pub fn unit(&self) -> Result<TemperatureUnit, String> {
        Ok(TemperatureUnit(self.index("unit", 0, 3)?))
    }
}
pub(crate) fn value_number(v: &Value) -> Result<f64, String> {
    match v {
        Value::String(s) => parse_number(s),
        Value::Number(n) => n
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or("数值无效".into()),
        _ => Err("请填写完整数值".into()),
    }
}

/// 温标往返在边界可能相差几个浮点末位；仅吸收运算舍入，不放宽实际温区。
pub(crate) fn celsius_in_bounds(unit: TemperatureUnit, value: f64, bounds: (f64, f64)) -> f64 {
    let t = unit.celsius(value);
    let tolerance = 8. * f64::EPSILON * value.abs().max(t.abs()).max(1.);
    if t < bounds.0 && bounds.0 - t <= tolerance {
        bounds.0
    } else if t > bounds.1 && t - bounds.1 <= tolerance {
        bounds.1
    } else {
        t
    }
}

#[derive(Default, Serialize)]
pub(crate) struct Output {
    pub metrics: Vec<Metric>,
    pub headings: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub curve: Vec<[f64; 2]>,
    pub points: Vec<[f64; 2]>,
    pub residuals: Vec<[f64; 2]>,
    pub coefficients: Vec<f64>,
    pub tolerances: Vec<Value>,
    pub warnings: Vec<String>,
    pub details: Value,
    pub total: usize,
    pub equation: String,
}
#[derive(Serialize)]
pub(crate) struct Metric {
    pub label: String,
    pub value: Value,
    pub unit: String,
}
impl Output {
    pub fn metric(&mut self, label: &str, value: impl Into<Value>, unit: &str) {
        self.metrics.push(Metric {
            label: label.into(),
            value: value.into(),
            unit: unit.into(),
        })
    }
    pub fn optional(&mut self, label: &str, value: Option<f64>, unit: &str) {
        self.metric(label, json!(value), unit)
    }
}

/// 分度表按显示温标采样；最大 5 万行，末端包含终点。
pub(crate) fn division(
    out: &mut Output,
    input: &Input,
    bounds: (f64, f64),
    signal: &str,
    curve: impl Fn(f64) -> Result<f64, String>,
) -> Result<(), String> {
    if !input.flag("division") {
        return Ok(());
    }
    let unit = input.unit()?;
    let start = input.number("tableStart", unit.display(bounds.0))?;
    let end = input.number("tableEnd", unit.display(bounds.1))?;
    let step = input.number("tableStep", unit.span())?;
    if unit.celsius(start) < bounds.0 - 1e-9 || unit.celsius(end) > bounds.1 + 1e-9 {
        return Err("分度表超出所选温区".into());
    }
    let count = crate::calculators::spline::model::planned_sample_count(start, end, step)?;
    if count > 50_000 {
        return Err("分度表最多 50000 行，请增大步进".into());
    }
    out.headings = vec![
        format!("温度 ({})", unit.symbol()),
        format!("信号 ({signal})"),
        format!("变化率 ({signal}/{})", unit.symbol()),
    ];
    for index in 0..count {
        let x = crate::calculators::spline::model::sample_x(start, end, step, count, index);
        let t = unit.celsius(x).clamp(bounds.0, bounds.1);
        let delta = (t.abs() * f64::EPSILON * 8.0).max(0.01);
        let lo = (t - delta).max(bounds.0);
        let hi = (t + delta).min(bounds.1);
        out.rows.push(vec![
            json!(x),
            json!(curve(t)?),
            json!((curve(hi)? - curve(lo)?) / (hi - lo) / unit.span()),
        ]);
    }
    for i in 0..=200 {
        let x = start + (end - start) * i as f64 / 200.0;
        out.curve
            .push([x, curve(unit.celsius(x).clamp(bounds.0, bounds.1))?]);
    }
    out.total = count;
    Ok(())
}
