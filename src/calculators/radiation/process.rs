//! 辐射模型的公式、实际参数及数值求解过程。
use super::model::{C2, Spectrum};
use crate::calculators::common::{format_process_computed as computed, format_process_number as n};

pub(crate) fn ear(
    target: f64,
    emissivity: f64,
    probe: f64,
    probe_emissivity: f64,
    single: bool,
    light: f64,
) -> Vec<String> {
    let black = target + 273.15;
    let probe_k = probe + 273.15;
    let scale = black.max(probe_k);
    let b = black / scale;
    let p = probe_k / scale;
    let mut lines = vec![
        format!(
            "黑体温度 T黑 = {} °C = {} K，发射率 ε黑 = {}；探头温度 T探 = {} °C = {} K，发射率 ε探 = {}。",
            n(target),
            computed(black),
            n(emissivity),
            n(probe),
            computed(probe_k),
            n(probe_emissivity)
        ),
        format!(
            "为避免四次方溢出，取 S = max(T黑, T探) = {} K；b = T黑/S = {}，p = T探/S = {}。",
            computed(scale),
            computed(b),
            computed(p)
        ),
        String::new(),
    ];
    if single {
        let signal = emissivity * b.powi(4)
            + (1.0 - emissivity)
                * (probe_emissivity * p.powi(4)
                    + (1.0 - probe_emissivity) * emissivity * b.powi(4));
        lines.push("一次反射近似：Q = ε黑·b⁴ + (1−ε黑)[ε探·p⁴ + (1−ε探)ε黑·b⁴]。".to_owned());
        lines.push(format!(
            "Q = {} × {}⁴ + (1 − {}) × [{} × {}⁴ + (1 − {}) × {} × {}⁴] = {}。",
            n(emissivity),
            computed(b),
            n(emissivity),
            n(probe_emissivity),
            computed(p),
            n(probe_emissivity),
            n(emissivity),
            computed(b),
            computed(signal)
        ));
    } else {
        let denominator = emissivity + (1.0 - emissivity) * probe_emissivity;
        let black_weight = emissivity / denominator;
        let probe_weight = (1.0 - emissivity) * probe_emissivity / denominator;
        let signal = black_weight * b.powi(4) + probe_weight * p.powi(4);
        lines.push(format!(
            "多次反射平衡：D = ε黑 + (1−ε黑)ε探 = {} + (1 − {}) × {} = {}。",
            n(emissivity),
            n(emissivity),
            n(probe_emissivity),
            computed(denominator)
        ));
        lines.push(format!(
            "黑体权重 = ε黑 / D = {}；探头权重 = (1−ε黑)ε探 / D = {}。",
            computed(black_weight),
            computed(probe_weight)
        ));
        lines.push(format!(
            "Q = 黑体权重 × b⁴ + 探头权重 × p⁴ = {} × {}⁴ + {} × {}⁴ = {}。",
            computed(black_weight),
            computed(b),
            computed(probe_weight),
            computed(p),
            computed(signal)
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "亮温 = S × √√Q − 273.15 = {} °C；亮温修正值 = 亮温 − 黑体温度 = {} − {} = {} °C。",
        computed(light),
        computed(light),
        n(target),
        computed(light - target)
    ));
    lines
}

pub(crate) fn wavelength(response: usize, lambda: f64, t: f64) -> Vec<String> {
    let line = match response {
        1 => format!(
            "8～14 μm 波段：有效波长 λ = 9.6 + 221 / (t + 273.15) = 9.6 + 221 / {} = {} μm。",
            computed(t + 273.15),
            computed(lambda)
        ),
        2 => format!(
            "7～18 μm 波段：有效波长 λ = 8.7 + 525 / (t + 273.15) = 8.7 + 525 / {} = {} μm。",
            computed(t + 273.15),
            computed(lambda)
        ),
        _ => format!("单一有效波长 λ = {} μm。", n(lambda)),
    };
    vec![
        "采用有效波长法，c₂ = 14388 μm·K；温度代入时使用 K。".to_owned(),
        line,
        String::new(),
    ]
}

pub(crate) fn effective_error(
    label: &str,
    emissivity: f64,
    lambda: f64,
    t: f64,
    background: f64,
    error: f64,
) -> Vec<String> {
    let mut lines = vec![format!(
        "{label}：发射率 ε = {}，温度 t = {} °C = {} K，背景温度 = {} °C = {} K。",
        n(emissivity),
        n(t),
        computed(t + 273.15),
        n(background),
        computed(background + 273.15)
    )];
    if emissivity == 1.0 {
        lines.push("ε = 1，模型直接返回发射率误差 0 °C。".to_owned());
        return lines;
    }
    let c = 14388.0 / lambda;
    let occupation = 1.0 / (c / (t + 273.15)).exp_m1()
        - (1.0 - emissivity) / (c / (background + 273.15)).exp_m1();
    let brightness = c / (emissivity / occupation).ln_1p();
    lines.push(format!(
        "c = 14388 / λ = 14388 / {} = {} K。",
        n(lambda),
        computed(c)
    ));
    lines.push(format!("C = 1 / [exp(c/T) − 1] − (1 − ε) / [exp(c/T背景) − 1] = 1 / [exp({}/{}) − 1] − (1 − {}) / [exp({}/{}) − 1] = {}。", computed(c), computed(t + 273.15), n(emissivity), computed(c), computed(background + 273.15), computed(occupation)));
    lines.push(format!(
        "M = c / ln(1 + ε/C) = {} / ln(1 + {}/{}) = {} K；发射率误差 = T − M = {} − {} = {} °C。",
        computed(c),
        n(emissivity),
        computed(occupation),
        computed(brightness),
        computed(t + 273.15),
        computed(brightness),
        computed(error)
    ));
    lines.push(
        "程序使用 expm1(x) 计算 exp(x)−1，使用 ln1p(x) 计算 ln(1+x)，以减少小数值的舍入误差。"
            .to_owned(),
    );
    lines
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn planck(
    spectrum: &Spectrum,
    response: usize,
    lambda: f64,
    temperature: f64,
    background: f64,
    source: f64,
    instrument: f64,
    result: f64,
    correction: bool,
) -> Vec<String> {
    let weight = if correction {
        instrument / source
    } else {
        source
    };
    let mut lines = vec![
        format!("采用 Planck 辐射平衡；输入温度 = {} °C = {} K，背景 = {} °C = {} K，辐射源发射率 = {}。", n(temperature), computed(temperature + 273.15), n(background), computed(background + 273.15), n(source)),
        match response { 1 => "光谱响应为 8～14 μm 平坦波段，采用 256 等分复合辛普森积分。".to_owned(), 2 => "光谱响应为 7～18 μm 平坦波段，采用 256 等分复合辛普森积分。".to_owned(), _ => format!("光谱响应为单一波长 {} μm。", n(lambda)) },
        format!("单波长相对辐射量 Bλ(T) = λ⁻⁵ / [exp(c₂/(λT)) − 1]；c₂ = hc/k = {} μm·K，比较时共同辐射常数约去。", n(C2)),
        "程序按对数域求值和加权求和；当 x = c₂/(λT) > 50 时，以 x 近似 ln(exp(x)−1)，避免指数溢出。".to_owned(),
        String::new(),
    ];
    if correction {
        lines.push(format!("ε源 B(T真) + (1−ε源)B(T背景) = ε表 B(T示) + (1−ε表)B(T背景)；ε表 = {}，权重 w = ε表/ε源 = {}。", n(instrument), computed(weight)));
    } else {
        lines.push(format!(
            "B(T亮) = ε源 B(T真) + (1−ε源)B(T背景)；权重 w = ε源 = {}。",
            computed(weight)
        ));
    }
    if weight == 1.0 {
        lines.push("w = 1，背景辐射项的系数为零，计算温度等于输入温度。".to_owned());
    } else {
        let lt = spectrum.log_radiance(temperature + 273.15);
        let lb = spectrum.log_radiance(background + 273.15);
        let scale = lt.max(lb);
        let signal = weight.mul_add((lt - scale).exp(), (1.0 - weight) * (lb - scale).exp());
        let log_target = scale + signal.ln();
        lines.push(format!(
            "输入温度的 ln(B) = {}，背景的 ln(B) = {}；取 S = max[ln(B输入), ln(B背景)] = {}。",
            computed(lt),
            computed(lb),
            computed(scale)
        ));
        lines.push(format!("对数域合成：Q = w·exp(ln(B输入)−S) + (1−w)·exp(ln(B背景)−S) = {}；目标 ln(B) = S + ln(Q) = {}。", computed(signal), computed(log_target)));
        lines.push("从 0～512 K 开始，必要时将上限加倍；再对同一辐射函数进行 85 次二分，以最终区间中点作为反算温度。".to_owned());
    }
    lines.push(String::new());
    if correction {
        lines.push(format!(
            "真温 = {} °C；修正值 = 真温 − 示值 = {} − {} = {} °C。",
            computed(result),
            computed(result),
            n(temperature),
            computed(result - temperature)
        ));
    } else {
        lines.push(format!(
            "表观亮温 = {} °C；示值误差 = 亮温 − 真温 = {} − {} = {} °C；修正值 = {} °C。",
            computed(result),
            computed(result),
            n(temperature),
            computed(result - temperature),
            computed(temperature - result)
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculators::radiation::{
        model::{self, Response},
        website,
    };

    #[test]
    fn process_tracks_reflection_effective_wavelength_and_planck_models() {
        for single in [false, true] {
            let (t, eps, probe, probe_eps) = (35.0, 0.9995, 18.0, 0.8);
            let light = if single {
                model::ear_single_reflection(t, eps, probe, probe_eps)
            } else {
                model::ear_blackbody(t, eps, probe, probe_eps)
            }
            .unwrap();
            let text = ear(t, eps, probe, probe_eps, single, light)
                .join("\n")
                .replace('\u{2063}', "");
            assert!(text.contains(if single {
                "一次反射近似"
            } else {
                "多次反射平衡"
            }));
            assert!(text.contains(&n(light)));
            assert!(text.contains(&n(light - t)));
        }
        for response in 0..3 {
            let lambda = website::wavelength(response, 0.65, 100.0);
            assert!(
                wavelength(response, lambda, 100.0)
                    .join("\n")
                    .replace('\u{2063}', "")
                    .contains(&n(lambda))
            );
            for eps in [0.8, 1.0] {
                let actual = website::error(eps, lambda, 100.0, 20.0).unwrap();
                let text = effective_error("辐射源", eps, lambda, 100.0, 20.0, actual)
                    .join("\n")
                    .replace('\u{2063}', "");
                assert!(text.contains(if eps == 1.0 {
                    "直接返回"
                } else {
                    "发射率误差 = T − M"
                }));
                assert!(text.contains(&n(actual)));
            }
            let spectrum = Spectrum::new(match response {
                1 => Response::Band(8.0, 14.0),
                2 => Response::Band(7.0, 18.0),
                _ => Response::Single(0.65),
            })
            .unwrap();
            for correction in [false, true] {
                let actual = if correction {
                    spectrum.corrected(100.0, 20.0, 0.95, 0.99)
                } else {
                    spectrum.apparent(100.0, 20.0, 0.99)
                }
                .unwrap();
                let text = planck(
                    &spectrum, response, 0.65, 100.0, 20.0, 0.99, 0.95, actual, correction,
                )
                .join("\n")
                .replace('\u{2063}', "");
                assert!(text.contains(if correction {
                    "ε表/ε源"
                } else {
                    "权重 w = ε源"
                }));
                assert!(text.contains(&n(actual)));
                assert!(text.contains(&n(spectrum.log_radiance(373.15))));
            }
        }
    }

    #[test]
    fn displayed_intermediates_match_radiation_models_across_branches() {
        let mut checked = 0;
        for response in 0..3 {
            let spectrum = Spectrum::new(match response {
                1 => Response::Band(8.0, 14.0),
                2 => Response::Band(7.0, 18.0),
                _ => Response::Single(0.65),
            })
            .unwrap();
            for temperature in [-100.0, 0.0, 100.0, 1000.0] {
                for background in [-50.0, 20.0, 200.0] {
                    for source in [0.3, 0.8, 1.0] {
                        for correction in [false, true] {
                            let instrument = 0.95;
                            let result = if correction {
                                spectrum.corrected(temperature, background, instrument, source)
                            } else {
                                spectrum.apparent(temperature, background, source)
                            };
                            let Ok(result) = result else {
                                continue;
                            };
                            let weight = if correction {
                                instrument / source
                            } else {
                                source
                            };
                            let text = planck(
                                &spectrum,
                                response,
                                0.65,
                                temperature,
                                background,
                                source,
                                instrument,
                                result,
                                correction,
                            )
                            .join("\n")
                            .replace('\u{2063}', "");
                            if weight == 1.0 {
                                assert_eq!(result, temperature);
                                assert!(text.contains("背景辐射项的系数为零"));
                            } else {
                                let lt = spectrum.log_radiance(temperature + 273.15);
                                let lb = spectrum.log_radiance(background + 273.15);
                                let scale = lt.max(lb);
                                let signal = weight.mul_add((lt - scale).exp(), (1.0 - weight) * (lb - scale).exp());
                                let target = scale + signal.ln();
                                assert!(text.contains(&format!("ln(B输入), ln(B背景)] = {}。", n(scale))));
                                assert!(text.contains(&format!("= {}；目标 ln(B) = S + ln(Q) = {}。", n(signal), n(target))));
                                assert!((spectrum.log_radiance(result + 273.15) - target).abs() < 1e-10);
                            }
                            checked += 1;
                        }
                        let lambda = website::wavelength(response, 0.65, temperature);
                        if let Ok(error) = website::error(source, lambda, temperature, background) {
                            let text = effective_error(
                                "辐射源",
                                source,
                                lambda,
                                temperature,
                                background,
                                error,
                            )
                            .join("\n")
                            .replace('\u{2063}', "");
                            assert!(text.contains(&n(error)));
                            if source != 1.0 {
                                let c = 14388.0 / lambda;
                                let occupation = 1.0 / (c / (temperature + 273.15)).exp_m1()
                                    - (1.0 - source) / (c / (background + 273.15)).exp_m1();
                                let brightness = c / (source / occupation).ln_1p();
                                assert_eq!(error, temperature + 273.15 - brightness);
                                assert!(
                                    text.contains(&format!("= {} K；发射率误差", n(brightness)))
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 150);
    }

    #[test]
    fn displayed_ear_weights_reproduce_actual_results() {
        for target in [-200.0_f64, 0.0, 35.0, 200.0] {
            for probe in [-100.0_f64, 18.0, 200.0] {
                for emissivity in [0.2_f64, 0.9995, 1.0] {
                    for probe_emissivity in [0.2_f64, 0.8, 1.0] {
                        let black = target + 273.15;
                        let probe_k = probe + 273.15;
                        let scale = black.max(probe_k);
                        let b = black / scale;
                        let p = probe_k / scale;
                        for single in [false, true] {
                            let result = if single {
                                model::ear_single_reflection(
                                    target,
                                    emissivity,
                                    probe,
                                    probe_emissivity,
                                )
                            } else {
                                model::ear_blackbody(target, emissivity, probe, probe_emissivity)
                            }
                            .unwrap();
                            let q = if single {
                                emissivity * b.powi(4)
                                    + (1.0 - emissivity)
                                        * (probe_emissivity * p.powi(4)
                                            + (1.0 - probe_emissivity) * emissivity * b.powi(4))
                            } else {
                                let d = emissivity + (1.0 - emissivity) * probe_emissivity;
                                emissivity / d * b.powi(4)
                                    + (1.0 - emissivity) * probe_emissivity / d * p.powi(4)
                            };
                            assert_eq!(result, scale * q.sqrt().sqrt() - 273.15);
                            let text =
                                ear(target, emissivity, probe, probe_emissivity, single, result)
                                    .join("\n")
                                    .replace('\u{2063}', "");
                            assert!(text.contains(&format!("= {}。", n(q))));
                            assert!(text.contains(&format!("= {} °C。", n(result - target))));
                        }
                    }
                }
            }
        }
    }
}
