//! 拟合与样条只组织输入输出，数值求解使用共享模型。
use crate::api::{Input, Output};
use crate::calculators::{
    curve_fit::model::{CurveFit, FitKind},
    spline::model::{Point, UniformCubicSpline, planned_sample_count, sample_x},
};
use crate::catalog::ToolKind;
use serde_json::json;

pub(crate) const SPLINE_POINTS: &[(f64, f64)] = &[
    (250., 2002.),
    (550., 6531.),
    (850., 12285.),
    (1150., 18708.),
    (1450., 25475.),
    (1750., 32170.),
    (2050., 38455.),
    (2350., 43954.),
    (2650., 48340.),
];
pub(crate) const FIT_POINTS: &[(f64, f64)] = &[
    (0., 0.197),
    (5., 0.208),
    (10., 0.231),
    (20., 0.263),
    (40., 0.338),
    (60., 0.439),
];

pub(crate) fn calculate(kind: ToolKind, i: &Input) -> Result<Output, String> {
    let mut o = Output::default();
    if matches!(kind, ToolKind::CalibrationCurveFit) {
        let pts = i.points(FIT_POINTS, 2, 200)?;
        let kind = FitKind::from_index(i.index("model", 0, 4)?);
        let fit = CurveFit::calculate(&pts, kind)?;
        o.coefficients = fit.coefficients.clone();
        o.equation = fit
            .coefficients
            .iter()
            .enumerate()
            .rev()
            .map(|(n, c)| match n {
                0 => format!("({c})"),
                1 => format!("({c})·x"),
                _ => format!("({c})·x^{n}"),
            })
            .collect::<Vec<_>>()
            .join(" + ");
        o.metric("数据点", pts.len(), "组");
        o.optional("相关系数 r", fit.correlation, "");
        o.optional("决定系数 R²", fit.r_squared, "");
        o.metric("均方根误差 RMSE", fit.rmse, "");
        o.optional("残差标准差", fit.residual_sd, "");
        o.metric("最大绝对残差", fit.maximum_residual, "");
        o.metric("残差自由度", pts.len() - kind.parameter_count(), "");
        for (j, &(x, y)) in pts.iter().enumerate() {
            o.rows.push(vec![
                json!(j + 1),
                json!(x),
                json!(y),
                json!(fit.fitted[j]),
                json!(fit.residuals[j]),
            ]);
            o.points.push([x, y]);
            o.residuals.push([x, fit.residuals[j]]);
        }
        o.headings = vec![
            "序号".into(),
            "输入量 x".into(),
            "实测响应 y".into(),
            "拟合响应".into(),
            "残差 y−拟合 y".into(),
        ];
        for j in 0..=240 {
            let x = fit.minimum_x + (fit.maximum_x - fit.minimum_x) * j as f64 / 240.;
            o.curve.push([x, fit.predict(x)?]);
        }
        for (j, &v) in fit.coefficients.iter().enumerate() {
            o.metric(&format!("K{j}"), v, "");
        }
        if fit.correlation.is_none() {
            o.warnings.push("当前数据的相关系数未定义。".into())
        }
        o.total = pts.len();
        return Ok(o);
    }
    let (pts, m, start, end, step, total) = spline_model(i)?;
    o.total = total;
    o.metric("Q 型值点", pts.len(), "个");
    o.metric("完整采样结果", total, "行");
    o.headings = vec!["x".into(), "y".into()];
    for j in 0..total.min(100) {
        let x = sample_x(start, end, step, total, j);
        o.rows.push(vec![json!(x), json!(m.y_at_x(x)?)]);
    }
    for j in 0..=240 {
        let x = start + (end - start) * j as f64 / 240.;
        o.curve.push([x, m.y_at_x(x)?]);
    }
    o.points = pts.iter().map(|p| [p.x, p.y]).collect();
    Ok(o)
}

fn spline_model(
    i: &Input,
) -> Result<(Vec<Point>, UniformCubicSpline, f64, f64, f64, usize), String> {
    let points = i
        .points(SPLINE_POINTS, 2, 500)?
        .into_iter()
        .map(|(x, y)| Point { x, y })
        .collect::<Vec<_>>();
    let m = UniformCubicSpline::interpolate(&points)?;
    let start = i.number("start", points[0].x)?;
    let end = i.number("end", points.last().ok_or("缺少 Q 点")?.x)?;
    let step = i.number("step", 0.5)?;
    let total = planned_sample_count(start, end, step)?;
    m.y_at_x(start)?;
    m.y_at_x(end)?;
    Ok((points, m, start, end, step, total))
}

pub(crate) fn spline_csv(i: &Input, offset: usize, limit: usize) -> Result<String, String> {
    use std::fmt::Write;
    if limit > 10_000 {
        return Err("每批采样最多 10000 行".into());
    }
    let (_, m, start, end, step, total) = spline_model(i)?;
    let stop = offset.saturating_add(limit).min(total);
    let mut text = String::new();
    for j in offset..stop {
        let x = sample_x(start, end, step, total, j);
        writeln!(text, "{x},{}", m.y_at_x(x)?).map_err(|_| "CSV 文本生成失败")?;
    }
    Ok(text)
}
