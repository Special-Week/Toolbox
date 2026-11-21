//! 变送器量程内的线性/开方双向模型；所有过程量先归一为 0～1 的量程比例。
use crate::calculators::numerics::finite;
use crate::calculators::pressure::model::Range;

pub(crate) struct Transmitter {
    process: Range,
    signal: Range,
    square_root: bool,
}

impl Transmitter {
    pub(crate) fn new(process: Range, signal: Range, square_root: bool) -> Self {
        Self {
            process,
            signal,
            square_root,
        }
    }

    pub(crate) fn forward(&self, value: f64) -> Result<(f64, f64), String> {
        let fraction = self.fraction(value, self.process)?;
        let output_fraction = if self.square_root {
            fraction.sqrt()
        } else {
            fraction
        };
        let signal = if output_fraction == 1.0 {
            self.signal.high
        } else {
            output_fraction.mul_add(self.signal.span, self.signal.low)
        };
        finite(&[signal])?;
        Ok((signal, fraction))
    }

    pub(crate) fn inverse(&self, signal: f64) -> Result<(f64, f64), String> {
        let fraction = self.fraction(signal, self.signal)?;
        let process_fraction = if self.square_root {
            fraction * fraction
        } else {
            fraction
        };
        let value = if fraction == 1.0 {
            self.process.high
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
