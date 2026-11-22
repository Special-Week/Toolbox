//! 拟合与样条只组织输入输出，数值求解使用共享模型。
use crate::api::{Input, Output};
use crate::calculators::{
    curve_fit::model::{CurveFit, FitKind},
    spline::model::{Point, UniformCubicSpline, planned_sample_count, sample_x},
};
use crate::catalog::ToolKind;
use serde_json::json;
use std::cell::RefCell;

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
        o.process = fit.process(&pts);
        o.coefficients = fit.coefficients.clone();
        o.prediction_formula = fit.prediction_formula();
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
            let fraction = j as f64 / 240.0;
            let x = (1.0 - fraction).mul_add(fit.minimum_x, fraction * fit.maximum_x);
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
    o.process = vec![format!(
        "输入 {} 个 Q 型值点，横坐标严格递增；端点二阶导数为零，内部控制点满足 Pᵢ₋₁ + 4Pᵢ + Pᵢ₊₁ = 6Qᵢ。",
        pts.len()
    )];
    o.process.extend(m.normalization_steps());
    o.process.push(format!("采样范围 {}～{}，步进 {}；共 {} 个采样点，最后一点固定为最大值。以下以一个预览点说明，其余点采用同样的方法。", start, end, step, total));
    o.process.push(String::new());
    let sample = sample_x(start, end, step, total, (total.min(100) - 1) / 2);
    o.process.extend(m.process_at_x(sample)?);
    o.total = total;
    o.metric("Q 型值点", pts.len(), "个");
    o.metric("完整采样结果", total, "行");
    o.headings = vec!["x".into(), "y".into()];
    for j in 0..total.min(100) {
        let x = sample_x(start, end, step, total, j);
        o.rows.push(vec![json!(x), json!(m.y_at_x(x)?)]);
    }
    for j in 0..=240 {
        let x = if j == 240 {
            end
        } else {
            (end - start).mul_add(j as f64 / 240.0, start).min(end)
        };
        o.curve.push([x, m.y_at_x(x)?]);
    }
    o.points = pts.iter().map(|p| [p.x, p.y]).collect();
    Ok(o)
}

/// 悬停读数使用与绘图相同的模型，不把采样点或屏幕坐标当作精确结果。
pub(crate) fn curve_value(id: &str, i: &Input, x: f64) -> Result<f64, String> {
    if !x.is_finite() {
        return Err("横坐标必须是有限数值".into());
    }
    match id {
        "calibration_curve_fit" => {
            let points = i.points(FIT_POINTS, 2, 200)?;
            let kind = FitKind::from_index(i.index("model", 0, 4)?);
            CurveFit::calculate(&points, kind)?.predict(x)
        }
        "bspline_interpolation" => spline_geometry(i)?.1.y_at_x(x),
        _ => Err("此工具不支持曲线悬停取值".into()),
    }
}

type SplineModelData = (Vec<Point>, UniformCubicSpline, f64, f64, f64, usize);

fn spline_geometry(i: &Input) -> Result<(Vec<Point>, UniformCubicSpline), String> {
    let points = i
        .points(SPLINE_POINTS, 2, 500)?
        .into_iter()
        .map(|(x, y)| Point { x, y })
        .collect::<Vec<_>>();
    let m = UniformCubicSpline::interpolate(&points)?;
    Ok((points, m))
}

fn spline_model(i: &Input) -> Result<SplineModelData, String> {
    let (points, m) = spline_geometry(i)?;
    let start = i.number("start", points[0].x)?;
    let end = i.number("end", points.last().ok_or("缺少 Q 点")?.x)?;
    let step = i.number("step", 0.5)?;
    let total = planned_sample_count(start, end, step)?;
    m.y_at_x(start)?;
    m.y_at_x(end)?;
    Ok((points, m, start, end, step, total))
}

thread_local! {
    // 导出请求携带同一份输入快照；页面改参或悬停不影响正在写出的分块。
    static SPLINE_EXPORT: RefCell<Option<(String, SplineModelData)>> = const { RefCell::new(None) };
}

pub(crate) fn spline_csv(input: &str, offset: usize, limit: usize) -> Result<String, String> {
    use std::fmt::Write;
    if limit > 10_000 {
        return Err("每批采样最多 10000 行".into());
    }
    SPLINE_EXPORT.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(key, _)| key != input) {
            let plan = spline_model(&Input::new(input)?)?;
            *cache = Some((input.to_owned(), plan));
        }
        let &(_, ref m, start, end, step, total) = &cache.as_ref().unwrap().1;
        let stop = offset.saturating_add(limit).min(total);
        let mut text = String::new();
        for j in offset..stop {
            let x = sample_x(start, end, step, total, j);
            writeln!(text, "{x},{}", m.y_at_x(x)?).map_err(|_| "CSV 文本生成失败")?;
        }
        Ok(text)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_csv_uses_the_requested_snapshot_across_chunks() {
        let initial = r#"{"points":[[0,0],[2,2]],"step":1}"#;
        let edited = r#"{"points":[[0,2],[2,4]],"step":0}"#;
        assert_eq!(spline_csv(initial, 0, 1).unwrap(), "0,0\n");
        let edited_input = Input::new(edited).unwrap();
        assert_eq!(curve_value("bspline_interpolation", &edited_input, 1.).unwrap(), 3.);
        assert!(spline_csv(edited, 0, 1).is_err());
        assert_eq!(spline_csv(initial, 1, 10).unwrap(), "1,1\n2,2\n");
        let edited = r#"{"points":[[0,2],[2,4]],"step":1}"#;
        assert_eq!(spline_csv(edited, 2, 1).unwrap(), "2,4\n");
        assert_eq!(spline_csv(initial, 2, 1).unwrap(), "2,2\n");
    }

    #[test]
    fn curve_hover_uses_the_same_fit_and_spline_models() {
        let input = Input::new("{}").unwrap();
        let fit = CurveFit::calculate(FIT_POINTS, FitKind::Linear).unwrap();
        assert!(
            (curve_value("calibration_curve_fit", &input, 12.345).unwrap()
                - fit.predict(12.345).unwrap())
            .abs()
                < 1e-12
        );
        let (_, spline, _, _, _, _) = spline_model(&input).unwrap();
        assert!(
            (curve_value("bspline_interpolation", &input, 1234.5).unwrap()
                - spline.y_at_x(1234.5).unwrap())
            .abs()
                < 1e-9
        );
        assert!(curve_value("other", &input, 1.0).is_err());
        assert!(curve_value("calibration_curve_fit", &input, f64::NAN).is_err());
    }
}
