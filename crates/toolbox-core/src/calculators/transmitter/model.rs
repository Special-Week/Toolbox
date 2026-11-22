//! 变送器量程内的线性/开方双向模型；所有过程量先归一为 0～1 的量程比例。
use crate::calculators::numerics::{binary_parts as split, finite, scale_binary};
use crate::calculators::pressure::model::Range;

pub struct Transmitter {
    process: Range,
    signal: Range,
    square_root: bool,
}

enum Transfer {
    Linear,
    Root,
    Square,
}

pub fn uses_midpoint(fraction: f64) -> bool {
    (0.25..=0.75).contains(&fraction)
}

pub fn crosses_zero(range: Range) -> bool {
    range.low < 0.0 && range.high > 0.0
}

fn sum_with_error(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    let part = sum - a;
    (sum, (a - (sum - part)) + (b - part))
}

struct ScaledProduct {
    high: f64,
    low: f64,
    exponent: i32,
}

impl ScaledProduct {
    fn new(a: f64, b: f64) -> Self {
        let (a, ea) = split(a);
        let (b, eb) = split(b);
        let high = a * b;
        Self {
            high,
            low: a.mul_add(b, -high),
            exponent: ea + eb,
        }
    }

    fn add(self, other: Self) -> Self {
        if self.high == 0.0 && self.low == 0.0 {
            return other;
        }
        if other.high == 0.0 && other.low == 0.0 {
            return self;
        }
        let exponent = self.exponent.max(other.exponent);
        let (main, main_error) = sum_with_error(
            scale_binary(self.high, self.exponent - exponent),
            scale_binary(other.high, other.exponent - exponent),
        );
        let (tail, tail_error) = sum_with_error(
            scale_binary(self.low, self.exponent - exponent),
            scale_binary(other.low, other.exponent - exponent),
        );
        let (middle, middle_error) = sum_with_error(main_error, tail);
        let (high, carry) = sum_with_error(main, middle);
        let (high, low) = sum_with_error(high, (carry + middle_error) + tail_error);
        Self { high, low, exponent }
    }

    fn multiply(&self, other: &Self) -> Self {
        let high = self.high * other.high;
        let error = self.high.mul_add(other.high, -high);
        let (high, low) = sum_with_error(high, error + self.high * other.low + self.low * other.high);
        Self { high, low, exponent: self.exponent + other.exponent }
    }

    fn ratio_root(&self, denominator: &Self) -> Self {
        let (num, ne) = split(self.high + self.low);
        let (den, de) = split(denominator.high + denominator.low);
        let exponent = self.exponent + ne - denominator.exponent - de;
        let even = exponent.div_euclid(2);
        let ratio = if exponent % 2 == 0 { num / den } else { 2.0 * (num / den) };
        Self { high: ratio.sqrt(), low: 0.0, exponent: even }
    }

    fn divide(&self, denominator: &Self) -> f64 {
        let (high, exponent) = split(denominator.high);
        let low = scale_binary(denominator.low, -exponent);
        let quotient = self.high / high;
        let remainder = (-quotient).mul_add(high, self.high) + self.low - quotient * low;
        scale_binary(quotient + remainder / high, self.exponent - denominator.exponent - exponent)
    }
}

/// 线性结果跨零时，用补偿运算计算加权和以保留抵消后的有效数字。
fn crossing_linear(value: f64, source: Range, target: Range) -> f64 {
    if value == source.low {
        return target.low;
    }
    if value == source.high {
        return target.high;
    }
    if source.low == target.low && source.high == target.high {
        return value;
    }

    // 分子为 x(d − c) + cb − da。先合并常数项，各乘积保留独立的二进制指数。
    let offset = ScaledProduct::new(target.low, source.high)
        .add(ScaledProduct::new(-target.high, source.low));
    let increment = ScaledProduct::new(value, target.high)
        .add(ScaledProduct::new(value, -target.low));
    let numerator = offset.add(increment);
    let (denominator, exponent) = split(source.span);
    let quotient = numerator.high / denominator;
    let remainder = (-quotient).mul_add(denominator, numerator.high) + numerator.low;
    let result = scale_binary(quotient + remainder / denominator, numerator.exponent - exponent);
    result.clamp(target.low, target.high)
}

// 三个双精度尾数的乘积可用 12 位分段精确相加，跨零时不会留下舍入残差。
fn sum_triple_products(terms: &[(f64, f64, f64, i32)]) -> ScaledProduct {
    const BITS: usize = 12;
    const BASE: i64 = 1 << BITS;
    const MIN_EXPONENT: i32 = -3 * (1074 + 52);
    let mut digits = [0_i64; ((3 * 1024 - MIN_EXPONENT) as usize) / BITS + 3];
    let parts = |value| {
        let (mantissa, exponent) = split(value);
        ((mantissa.abs() * (1_u64 << 52) as f64) as u64, exponent - 52)
    };
    for &(a, b, c, twice) in terms {
        if a == 0.0 || b == 0.0 || c == 0.0 {
            continue;
        }
        let sign = if a.is_sign_negative() ^ b.is_sign_negative() ^ c.is_sign_negative() { -1 } else { 1 };
        let ((a, ae), (b, be), (c, ce)) = (parts(a), parts(b), parts(c));
        let offset = (ae + be + ce + twice - MIN_EXPONENT) as usize;
        for i in 0..5 {
            for j in 0..5 {
                for k in 0..5 {
                    let chunk = |n, i| ((n >> (i * BITS)) & (BASE - 1) as u64) as i64;
                    let product = chunk(a, i) * chunk(b, j) * chunk(c, k);
                    digits[offset / BITS + i + j + k] += sign * (product << (offset % BITS));
                }
            }
        }
    }
    let carry = |digits: &mut [i64]| {
        for i in 0..digits.len() - 1 {
            digits[i + 1] += digits[i].div_euclid(BASE);
            digits[i] = digits[i].rem_euclid(BASE);
        }
    };
    carry(&mut digits);
    let negative = digits.last().is_some_and(|&digit| digit < 0);
    if negative {
        for digit in &mut digits {
            *digit = -*digit;
        }
        carry(&mut digits);
    }
    let end = digits.iter().rposition(|&digit| digit != 0).unwrap_or(0);
    let mantissa = digits[end.saturating_sub(4)..=end].iter()
        .fold(0.0, |sum, &digit| sum / BASE as f64 + digit as f64);
    ScaledProduct {
        high: if negative { -mantissa } else { mantissa },
        low: 0.0,
        exponent: MIN_EXPONENT + (end * BITS) as i32,
    }
}

fn crossing_nonlinear(value: f64, source: Range, target: Range, forward: bool) -> f64 {
    if value == source.low {
        return target.low;
    }
    if value == source.high {
        return target.high;
    }
    let delta = ScaledProduct::new(value, 1.0).add(ScaledProduct::new(-source.low, 1.0));
    let input_span = ScaledProduct::new(source.high, 1.0).add(ScaledProduct::new(-source.low, 1.0));
    let output_span = ScaledProduct::new(target.high, 1.0).add(ScaledProduct::new(-target.low, 1.0));
    let (a, b, c, d, x) = (source.low, source.high, target.low, target.high, value);
    let result = if forward {
        // (c + R√f)(R√f − c) = R²f − c²，先计算分子里的微小差值。
        let numerator = sum_triple_products(&[
            (d, d, x, 0), (d, -c, x, 1), (c, c, x, 0),
            (d, d, -a, 0), (d, c, a, 1), (c, c, -b, 0),
        ]);
        let conjugate = output_span.multiply(&delta.ratio_root(&input_span))
            .add(ScaledProduct::new(-target.low, 1.0));
        numerator.divide(&input_span.multiply(&conjugate))
    } else {
        let denominator = input_span.multiply(&input_span);
        let numerator = sum_triple_products(&[
            (d, x, x, 0), (d, x, -a, 1), (d, a, a, 0),
            (-c, x, x, 0), (c, x, a, 1), (c, b, b, 0), (c, b, -a, 1),
        ]);
        numerator.divide(&denominator)
    };
    result.clamp(target.low, target.high)
}

impl Transmitter {
    pub fn new(process: Range, signal: Range, square_root: bool) -> Self {
        Self {
            process,
            signal,
            square_root,
        }
    }

    pub fn forward(&self, value: f64) -> Result<(f64, f64), String> {
        let fraction = self.fraction(value, self.process)?;
        let output_fraction = if self.square_root {
            fraction.sqrt()
        } else {
            fraction
        };
        let signal = if crosses_zero(self.signal) {
            if self.square_root {
                crossing_nonlinear(value, self.process, self.signal, true)
            } else {
                crossing_linear(value, self.process, self.signal)
            }
        } else if !self.square_root && uses_midpoint(fraction) {
            Self::from_midpoint(value, self.process, self.signal)
        } else if fraction > 0.5 {
            let factor = if self.square_root {
                1.0 / (1.0 + output_fraction)
            } else {
                1.0
            };
            Self::from_upper(value, self.process, self.signal, factor)
        } else if value > self.process.low && fraction < f64::MIN_POSITIVE {
            self.signal.low + scaled_step(
                value - self.process.low, self.process.span, self.signal.span,
                if self.square_root { Transfer::Root } else { Transfer::Linear }, 1.0,
            )
        } else {
            output_fraction.mul_add(self.signal.span, self.signal.low)
        };
        finite(&[signal])?;
        Ok((signal, fraction))
    }

    pub fn inverse(&self, signal: f64) -> Result<(f64, f64), String> {
        let fraction = self.fraction(signal, self.signal)?;
        let process_fraction = if self.square_root {
            fraction * fraction
        } else {
            fraction
        };
        let value = if crosses_zero(self.process) {
            if self.square_root {
                crossing_nonlinear(signal, self.signal, self.process, false)
            } else {
                crossing_linear(signal, self.signal, self.process)
            }
        } else if !self.square_root && uses_midpoint(fraction) {
            Self::from_midpoint(signal, self.signal, self.process)
        } else if fraction > 0.5 {
            let deficit = (self.signal.high - signal) / self.signal.span;
            let factor = if self.square_root {
                2.0 - deficit
            } else {
                1.0
            };
            Self::from_upper(signal, self.signal, self.process, factor)
        } else if signal > self.signal.low && process_fraction < f64::MIN_POSITIVE {
            self.process.low + scaled_step(
                signal - self.signal.low, self.signal.span, self.process.span,
                if self.square_root { Transfer::Square } else { Transfer::Linear }, 1.0,
            )
        } else {
            process_fraction.mul_add(self.process.span, self.process.low)
        };
        finite(&[value])?;
        Ok((value, process_fraction))
    }

    fn fraction(&self, value: f64, range: Range) -> Result<f64, String> {
        if !value.is_finite() || value < range.low || value > range.high {
            return Err("输入值超出所选量程".to_owned());
        }
        Ok((value - range.low) / range.span)
    }

    fn from_upper(value: f64, source: Range, target: Range, factor: f64) -> f64 {
        let distance = value - source.high;
        let slope = (target.span / source.span) * factor;
        if slope.is_normal() {
            distance.mul_add(slope, target.high)
        } else if distance == 0.0 {
            target.high
        } else {
            target.high - scaled_step(-distance, source.span, target.span, Transfer::Linear, factor)
        }
    }

    fn from_midpoint(value: f64, source: Range, target: Range) -> f64 {
        let midpoint = |range: Range| {
            if range.low < 0.0 && range.high > 0.0 {
                (range.low + range.high) * 0.5
            } else {
                range.low + range.span * 0.5
            }
        };
        let target_mid = midpoint(target);
        let distance = value - midpoint(source);
        let slope = target.span / source.span;
        if slope.is_normal() {
            distance.mul_add(slope, target_mid)
        } else if distance == 0.0 {
            target_mid
        } else {
            target_mid + scaled_step(distance.abs(), source.span, target.span, Transfer::Linear, 1.0)
                .copysign(distance)
        }
    }
}

/// 在归一化比例无法用正常双精度表示时，合并二进制指数后再计算实际增量。
fn scaled_step(delta: f64, source_span: f64, target_span: f64, transfer: Transfer, factor: f64) -> f64 {
    let (delta, delta_exponent) = split(delta);
    let (source, source_exponent) = split(source_span);
    let (target, target_exponent) = split(target_span);
    let mut ratio = delta / source;
    let mut exponent = delta_exponent - source_exponent;
    match transfer {
        Transfer::Linear => {}
        Transfer::Root => {
            if exponent % 2 != 0 {
                ratio *= 2.0;
                exponent -= 1;
            }
            ratio = ratio.sqrt();
            exponent /= 2;
        }
        Transfer::Square => {
            ratio *= ratio;
            exponent *= 2;
        }
    }
    exponent += target_exponent;
    scale_binary(ratio * target * factor, exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_round_trip_for_both_transmitter_modes() {
        let process = Range::new(-10.0, 0.4).unwrap();
        let signal = Range::new(-5.0, 0.2).unwrap();
        for square_root in [false, true] {
            let model = Transmitter::new(process, signal, square_root);
            for (input, output) in [(process.low, signal.low), (process.high, signal.high)] {
                assert_eq!(model.forward(input).unwrap().0, output);
                assert_eq!(model.inverse(output).unwrap().0, input);
            }
            assert!(model.inverse(signal.high.next_up()).is_err());
        }
    }
}
