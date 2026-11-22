//! 自然端点 B 样条与采样规则；桌面及网页共用，不依赖界面或文件系统。
pub const MAX_SAMPLE_COUNT: usize = 5_000_000;
pub const MAX_POINT_COUNT: usize = 500;
pub const MIN_POINT_COUNT: usize = 2;
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
struct Cubic {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl Cubic {
    fn value(self, u: f64) -> f64 {
        self.d
            .mul_add(u, self.c)
            .mul_add(u, self.b)
            .mul_add(u, self.a)
    }

    fn derivative(self, u: f64) -> f64 {
        (3.0 * self.d).mul_add(u, 2.0 * self.c).mul_add(u, self.b)
    }

    fn minimum_derivative(self) -> f64 {
        let mut minimum = self.derivative(0.0).min(self.derivative(1.0));
        if self.d > 0.0 {
            let vertex = -self.c / (3.0 * self.d);
            if (0.0..1.0).contains(&vertex) {
                minimum = minimum.min(self.derivative(vertex));
            }
        }
        minimum
    }
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    x: Cubic,
    y: Cubic,
}

pub struct UniformCubicSpline {
    points: Vec<Point>,
    segments: Vec<Segment>,
    // 计算使用去基准、归一化后的坐标；显示和导出时恢复原始量纲。
    origin: Point,
    scale: Point,
}

impl UniformCubicSpline {
    pub fn interpolate(points: &[Point]) -> Result<Self, String> {
        if !(MIN_POINT_COUNT..=MAX_POINT_COUNT).contains(&points.len()) {
            return Err("Q 型值点数量必须在 2 到 500 之间".to_owned());
        }
        for (index, point) in points.iter().enumerate() {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(format!("Q{index} 的坐标必须是有限数值"));
            }
            if index > 0 && point.x <= points[index - 1].x {
                return Err(format!("Q{index} 的 x 必须大于 Q{} 的 x", index - 1));
            }
        }

        let origin = points[0];
        let x_scale = points[points.len() - 1].x - origin.x;
        let y_scale = points
            .iter()
            .map(|point| (point.y - origin.y).abs())
            .fold(0.0, f64::max);
        let scale = Point {
            x: x_scale,
            y: if y_scale == 0.0 { 1.0 } else { y_scale },
        };
        if !scale.x.is_finite() || !scale.y.is_finite() || scale.x <= 0.0 {
            return Err("坐标跨度超出双精度范围，请检查输入".to_owned());
        }
        let normalized: Vec<_> = points
            .iter()
            .map(|point| Point {
                x: (point.x - origin.x) / scale.x,
                y: (point.y - origin.y) / scale.y,
            })
            .collect();
        let controls = solve_controls(&normalized);
        let segments: Vec<_> = controls
            .windows(4)
            .enumerate()
            .map(|(index, window)| {
                let window = [window[0], window[1], window[2], window[3]];
                Segment {
                    x: coefficients(window.map(|point| point.x), normalized[index].x),
                    y: coefficients(window.map(|point| point.y), normalized[index].y),
                }
            })
            .collect();
        for (index, segment) in segments.iter().enumerate() {
            if segment.x.minimum_derivative() < -32.0 * f64::EPSILON {
                return Err(format!(
                    "Q{index} 到 Q{} 之间的 x 出现回折，无法唯一计算 y(x)",
                    index + 1
                ));
            }
        }
        Ok(Self {
            points: points.to_vec(),
            segments,
            origin,
            scale,
        })
    }

    pub fn y_at_x(&self, x: f64) -> Result<f64, String> {
        if !x.is_finite() {
            return Err("x 必须是有限数值".to_owned());
        }
        let insertion = self.points.partition_point(|point| point.x < x);
        if let Some(point) = self.points.get(insertion)
            && point.x == x
        {
            return Ok(point.y);
        }
        let target = (x - self.origin.x) / self.scale.x;
        if !target.is_finite() {
            return Err("外推距离超出双精度范围".to_owned());
        }
        let (segment, lower, upper) = if insertion == 0 || insertion == self.points.len() {
            let right = insertion == self.points.len();
            let segment = self.endpoint_polynomial(right);
            let (lower, upper) = external_bracket(segment.x, target, right)?;
            (segment, lower, upper)
        } else {
            (self.segments[insertion - 1], 0.0, 1.0)
        };
        let parameter = invert(segment.x, target, lower, upper)?;
        let y = segment
            .y
            .value(parameter)
            .mul_add(self.scale.y, self.origin.y);
        if y.is_finite() {
            Ok(y)
        } else {
            Err(format!("x={x} 对应的 y 超出双精度范围"))
        }
    }

    /// 说明实际使用的归一化基准和缩放因子。
    pub fn normalization_steps(&self) -> Vec<String> {
        use crate::calculators::common::{
            format_process_computed as c, format_process_number as n,
        };
        let last = self.points.last().expect("样条至少有两个型值点");
        let (index, point) = self
            .points
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                (a.y - self.origin.y)
                    .abs()
                    .total_cmp(&(b.y - self.origin.y).abs())
            })
            .expect("样条至少有两个型值点");
        let max_difference = (point.y - self.origin.y).abs();
        let mut lines = vec![
            format!(
                "归一化基准为首个型值点 Q₀：x₀ = {}，y₀ = {}。",
                n(self.origin.x),
                n(self.origin.y)
            ),
            format!(
                "横坐标缩放因子 sₓ = x末 − x₀ = {} − {} = {}。",
                n(last.x),
                n(self.origin.x),
                c(self.scale.x)
            ),
        ];
        lines.push(if max_difference == 0.0 {
            "所有型值点的纵坐标都等于 y₀，maxᵢ|yᵢ − y₀| = 0；程序将 sᵧ 设为 1，避免除以零。".to_owned()
        } else {
            format!("纵坐标缩放因子 sᵧ = maxᵢ|yᵢ − y₀|；最大差值来自 Q{}：|{} − {}| = {}，所以 sᵧ = {}。", index, n(point.y), n(self.origin.y), c(max_difference), c(self.scale.y))
        });
        lines.push("各型值点归一化：Xᵢ = (xᵢ − x₀)/sₓ，Yᵢ = (yᵢ − y₀)/sᵧ；计算后按 y = y₀ + sᵧY 恢复纵坐标。".to_owned());
        lines
    }

    /// 使用与 y_at_x 相同的分段和反算，展示预览点所用的归一化多项式。
    pub fn process_at_x(&self, x: f64) -> Result<Vec<String>, String> {
        use crate::calculators::common::{
            format_process_computed as c, format_process_number as n,
        };
        let insertion = self.points.partition_point(|point| point.x < x);
        if let Some(point) = self.points.get(insertion)
            && point.x == x
        {
            return Ok(vec![format!(
                "x = {} 恰为 Q{} 的横坐标，直接返回原始 y = {}。",
                n(x),
                insertion,
                n(point.y)
            )]);
        }
        let target = (x - self.origin.x) / self.scale.x;
        let (segment, lower, upper, branch) = if insertion == 0 || insertion == self.points.len() {
            let right = insertion == self.points.len();
            let segment = self.endpoint_polynomial(right);
            let (lower, upper) = external_bracket(segment.x, target, right)?;
            (
                segment,
                lower,
                upper,
                if right {
                    "右端外推".to_owned()
                } else {
                    "左端外推".to_owned()
                },
            )
        } else {
            (
                self.segments[insertion - 1],
                0.0,
                1.0,
                format!("Q{}～Q{} 插值", insertion - 1, insertion),
            )
        };
        let u = invert(segment.x, target, lower, upper)?;
        let y = segment.y.value(u).mul_add(self.scale.y, self.origin.y);
        Ok(vec![
            format!(
                "x = {}；{}；目标归一化横坐标 X = (x − x₀)/sₓ = ({} − {})/{} = {}。",
                n(x),
                branch,
                n(x),
                n(self.origin.x),
                c(self.scale.x),
                c(target)
            ),
            format!(
                "X(u) = a + bu + cu² + du³；本段 x 系数 a、b、c、d = {}、{}、{}、{}；在 [{}，{}] 内用带括区间的 Newton 法反求，必要时二分，得到 u = {}。",
                n(segment.x.a),
                n(segment.x.b),
                n(segment.x.c),
                n(segment.x.d),
                n(lower),
                n(upper),
                c(u)
            ),
            format!(
                "Y(u) = a + bu + cu² + du³；本段 y 系数 a、b、c、d = {}、{}、{}、{}。",
                n(segment.y.a),
                n(segment.y.b),
                n(segment.y.c),
                n(segment.y.d)
            ),
            format!(
                "恢复坐标 y = y₀ + sᵧY(u) = {} + {} × {} = {}；多项式及恢复坐标使用融合乘加。",
                n(self.origin.y),
                c(self.scale.y),
                c(segment.y.value(u)),
                c(y)
            ),
        ])
    }

    /// 将端点平移为局部参数零点，保留自然条件（二阶导数为零）下的端点三次式。
    fn endpoint_polynomial(&self, right: bool) -> Segment {
        let index = if right { self.segments.len() - 1 } else { 0 };
        let parameter = if right { 1.0 } else { 0.0 };
        let point = if right {
            self.points[self.points.len() - 1]
        } else {
            self.origin
        };
        let segment = self.segments[index];
        Segment {
            x: Cubic {
                a: (point.x - self.origin.x) / self.scale.x,
                b: segment.x.derivative(parameter),
                c: 0.0,
                d: segment.x.d,
            },
            y: Cubic {
                a: (point.y - self.origin.y) / self.scale.y,
                b: segment.y.derivative(parameter),
                c: 0.0,
                d: segment.y.d,
            },
        }
    }
}

fn coefficients(p: [f64; 4], value: f64) -> Cubic {
    let delta = [p[1] - p[0], p[2] - p[1], p[3] - p[2]];
    let mut c = (delta[1] - delta[0]) / 2.0;
    let mut d = (delta[2] - 2.0 * delta[1] + delta[0]) / 6.0;
    let roundoff = 32.0 * f64::EPSILON * p.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if c.abs() <= roundoff {
        c = 0.0;
    }
    if d.abs() <= roundoff {
        d = 0.0;
    }
    Cubic {
        a: value,
        b: (p[2] - p[0]) / 2.0,
        c,
        d,
    }
}

/// 内部控制点满足 P[i−1]+4P[i]+P[i+1]=6Q[i]，用三对角消元同时求 x/y。
/// 两侧虚控制点由自然端点条件给出；只有两个 Q 点时自然退化为直线。
fn solve_controls(points: &[Point]) -> Vec<Point> {
    let count = points.len();
    let mut controls = vec![Point::default(); count + 2];
    controls[1] = points[0];
    controls[count] = points[count - 1];
    let unknowns = count - 2;
    let mut diagonal = vec![4.0; unknowns];
    let mut rhs = vec![Point::default(); unknowns];
    for row in 0..unknowns {
        rhs[row] = Point {
            x: 6.0 * points[row + 1].x,
            y: 6.0 * points[row + 1].y,
        };
        if row == 0 {
            rhs[row].x -= points[0].x;
            rhs[row].y -= points[0].y;
        }
        if row + 1 == unknowns {
            rhs[row].x -= points[count - 1].x;
            rhs[row].y -= points[count - 1].y;
        }
        if row > 0 {
            let factor = 1.0 / diagonal[row - 1];
            diagonal[row] -= factor;
            rhs[row].x -= factor * rhs[row - 1].x;
            rhs[row].y -= factor * rhs[row - 1].y;
        }
    }
    for row in (0..unknowns).rev() {
        let next = if row + 1 == unknowns {
            Point::default()
        } else {
            controls[row + 3]
        };
        controls[row + 2] = Point {
            x: (rhs[row].x - next.x) / diagonal[row],
            y: (rhs[row].y - next.y) / diagonal[row],
        };
    }
    controls[0] = Point {
        x: 2.0 * controls[1].x - controls[2].x,
        y: 2.0 * controls[1].y - controls[2].y,
    };
    controls[count + 1] = Point {
        x: 2.0 * controls[count].x - controls[count - 1].x,
        y: 2.0 * controls[count].y - controls[count - 1].y,
    };
    controls
}

/// 端点向外扩大搜索区间，到 x 的首个转折处即停止，不跨越多值分支。
fn external_bracket(polynomial: Cubic, target: f64, right: bool) -> Result<(f64, f64), String> {
    let failure = || "端点外推到达 x 转折点，无法唯一反求参数".to_owned();
    if polynomial.b < 0.0 || (polynomial.b == 0.0 && polynomial.d <= 0.0) {
        return Err(failure());
    }
    let limit = if polynomial.d < 0.0 {
        (-polynomial.b / (3.0 * polynomial.d)).sqrt()
    } else {
        f64::INFINITY
    };
    let sign = if right { 1.0 } else { -1.0 };
    let mut distance = 1.0_f64.min(limit);
    for _ in 0..1024 {
        let bound = sign * distance;
        let value = polynomial.value(bound);
        if !value.is_finite() {
            return Err(failure());
        }
        if (right && value >= target) || (!right && value <= target) {
            return Ok(if right { (0.0, bound) } else { (bound, 0.0) });
        }
        if distance == limit {
            return Err(failure());
        }
        distance = (distance * 2.0).min(limit);
    }
    Err(failure())
}

/// 有括区间约束的 Newton 迭代；导数无效或步长越界时回退到二分。
fn invert(polynomial: Cubic, target: f64, mut lower: f64, mut upper: f64) -> Result<f64, String> {
    let low_x = polynomial.value(lower);
    let high_x = polynomial.value(upper);
    let span = high_x - low_x;
    if !span.is_finite() || span <= 0.0 {
        return Err("样条参数区间无效".to_owned());
    }
    let mut parameter = lower + (upper - lower) * ((target - low_x) / span);
    if !parameter.is_finite() || parameter <= lower || parameter >= upper {
        parameter = lower + (upper - lower) / 2.0;
    }
    // 横坐标残差小并不代表参数已收敛：在水平切线附近尤其如此。
    for _ in 0..1200 {
        let error = polynomial.value(parameter) - target;
        if !error.is_finite() {
            break;
        }
        if error == 0.0 {
            return Ok(parameter);
        }
        if error < 0.0 {
            lower = parameter;
        } else {
            upper = parameter;
        }
        let derivative = polynomial.derivative(parameter);
        let candidate = parameter - error / derivative;
        if candidate == parameter {
            return Ok(parameter);
        }
        let midpoint = lower + (upper - lower) / 2.0;
        if midpoint == lower || midpoint == upper {
            return Ok(
                if (polynomial.value(lower) - target).abs()
                    <= (polynomial.value(upper) - target).abs()
                {
                    lower
                } else {
                    upper
                },
            );
        }
        parameter = if candidate.is_finite() && candidate > lower && candidate < upper {
            candidate
        } else {
            midpoint
        };
    }
    Err("反求样条参数未收敛，请检查 x 间距或外推范围".to_owned())
}

pub fn planned_sample_count(minimum: f64, maximum: f64, step: f64) -> Result<usize, String> {
    bounded_sample_count(
        minimum,
        maximum,
        step,
        MAX_SAMPLE_COUNT,
        "采样点最多为 5000000 个，请增大步进或缩小范围",
    )
}

pub fn division_sample_count(minimum: f64, maximum: f64, step: f64) -> Result<usize, String> {
    bounded_sample_count(
        minimum,
        maximum,
        step,
        50_000,
        "分度表最多 50000 行，请增大步长",
    )
}

fn bounded_sample_count(
    minimum: f64,
    maximum: f64,
    step: f64,
    limit: usize,
    limit_error: &str,
) -> Result<usize, String> {
    if !minimum.is_finite() || !maximum.is_finite() || !step.is_finite() {
        return Err("采样范围和步进必须是有限数值".to_owned());
    }
    if minimum >= maximum || step <= 0.0 {
        return Err("采样范围无效或步进不大于 0".to_owned());
    }
    let intervals = (maximum - minimum) / step;
    if !intervals.is_finite() || intervals > limit as f64 {
        return Err(limit_error.to_owned());
    }
    let nearest = intervals.round();
    let intervals = if (intervals - nearest).abs() <= 8.0 * f64::EPSILON * intervals.max(1.0) {
        nearest
    } else {
        intervals
    };
    let mut count = (intervals.ceil() as usize + 1).max(2);
    // The last regular sample can round to the forced endpoint.
    if count > 2 && sample_x(minimum, maximum, step, count, count - 2) >= maximum {
        count -= 1;
    }
    if count > limit {
        return Err(limit_error.to_owned());
    }
    let resolution = (minimum.next_up() - minimum).max(maximum - maximum.next_down());
    let remainder = (minimum % resolution).abs();
    // 坐标精度可能使采样重复时，核对实际采样而不是直接拒绝。
    if count > 2
        && (step < resolution
            || (step == resolution && remainder > 0.0 && remainder == resolution * 0.5))
    {
        let reverse = minimum >= 0.0;
        let mut previous = if reverse { maximum } else { minimum };
        for offset in 1..count {
            let index = if reverse { count - 1 - offset } else { offset };
            let x = sample_x(minimum, maximum, step, count, index);
            if (reverse && x >= previous) || (!reverse && x <= previous) {
                return Err("步进小于当前坐标精度，请增大步进".to_owned());
            }
            // 向零检查到更细的浮点档位后，剩余采样点无需逐个核对。
            if (reverse && step > x.next_up() - x)
                || (maximum <= 0.0 && step > x - x.next_down())
            {
                break;
            }
            previous = x;
        }
    }
    Ok(count)
}

pub fn sample_x(minimum: f64, maximum: f64, step: f64, count: usize, index: usize) -> f64 {
    if index + 1 == count {
        maximum
    } else {
        (index as f64).mul_add(step, minimum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_plan_prevents_duplicate_coordinates() {
        let base = 1e16;
        assert!(planned_sample_count(base, base + 20.0, 1.5).is_err());
        let binade = (1_u64 << 53) as f64;
        assert!(planned_sample_count(binade - 1.0, binade + 10.0, 2.0).is_err());
        let start = binade - 1.0;
        let end = binade + 2.0;
        let count = planned_sample_count(start, end, 2.0).unwrap();
        assert_eq!(count, 3);
        assert_eq!(sample_x(start, end, 2.0, count, 1), binade);
        assert_eq!(planned_sample_count(binade - 9_999_995.0, end, 2.0).unwrap(), 5_000_000);
        for (end, step, expected) in [(base + 10.0, 3.25, 4), (base + 20.0, 2.0, 11)] {
            let count = planned_sample_count(base, end, step).unwrap();
            assert_eq!(count, expected);
            let coordinates: Vec<_> = (0..count)
                .map(|index| sample_x(base, end, step, count, index))
                .collect();
            assert_eq!(coordinates.last(), Some(&end));
            assert!(coordinates.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }

    #[test]
    fn inversion_resolves_flat_endpoint_at_small_scales() {
        // 此样条第一段及左端外推满足 x = y³，可独立验证反算。
        let points = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 6.0, y: 2.0 },
        ];
        let spline = UniformCubicSpline::interpolate(&points).unwrap();
        for x in [-1e-15_f64, 1e-300, 1e-15, 1e-12, 1e-9, 0.125, 1.0] {
            let expected = x.cbrt();
            let actual = spline.y_at_x(x).unwrap();
            assert!(
                (actual - expected).abs() <= expected.abs() * 1e-11,
                "x={x:e}, actual={actual:e}, expected={expected:e}"
            );
        }
    }

    #[test]
    fn inversion_reconstructs_many_points_on_each_segment() {
        let points = [
            Point { x: -2.0, y: 4.0 },
            Point { x: -1.0, y: 1.0 },
            Point { x: 0.2, y: -3.0 },
            Point { x: 1.1, y: 0.0 },
            Point { x: 2.0, y: 5.0 },
        ];
        let spline = UniformCubicSpline::interpolate(&points).unwrap();
        for (i, segment) in spline.segments.iter().enumerate() {
            for j in 0..=1000 {
                let u = j as f64 / 1000.0;
                let x = segment.x.value(u).mul_add(spline.scale.x, spline.origin.x);
                let expected = segment.y.value(u).mul_add(spline.scale.y, spline.origin.y);
                let actual = spline.y_at_x(x).unwrap();
                assert!(
                    (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
                    "segment={i}, u={u}, x={x}, actual={actual}, expected={expected}"
                );
            }
        }
    }
}
