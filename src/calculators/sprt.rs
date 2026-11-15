//! 标准铂电阻页面；按子区间限制计算温区，并分离证书系数和固定点拟合输入。
mod curves;
pub(crate) mod model;
pub(crate) mod ranges;
use crate::calculators::{calibration::TemperatureUnit, common::*};
use crate::{app::ToolboxApp, catalog::ToolDef};
use eframe::egui;
use model::Sprt;
use ranges::LOW_RANGES;
impl ToolboxApp {
    pub(crate) fn render_standard_resistance(&mut self, ui: &mut egui::Ui, tool: &ToolDef) {
        ui.set_max_width(ui.available_width().min(820.0));
        let archive_save = self.calibration_archive_controls(ui, tool);
        if self.select_index(
            ui,
            tool,
            "sprt.model",
            "计算模型",
            &["标准铂电阻（SPRT 换算）", "CVD 系数拟合"],
            0,
        ) == 1
        {
            let valid = self.render_cvd_fit(ui, tool);
            let required = valid.then(|| {
                ["sprt.model", "r0", "t1", "r1", "t2", "r2", "t3", "r3"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            });
            self.finish_calibration_save(tool, archive_save, required);
            return;
        }
        let mut archive_required = None;
        let mut table = None;
        let mut process = None;
        let result = panel(ui, |ui| {
            let mut required = [
                "sprt.model",
                "sprt.hr",
                "sprt.lr",
                "sprt.deviation",
                "sprt.source",
                "sprt.rtp",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
            let hr = self.select_index(
                ui,
                tool,
                "sprt.hr",
                "高温分段",
                &[
                    "A6 / B6 / C6：至 961.78 °C",
                    "A7 / B7 / C7：至 660.323 °C",
                    "A8 / B8：至 419.527 °C",
                    "A9 / B9：至 231.928 °C",
                    "A10：至 156.599 °C",
                    "A11：至 29.765 °C",
                    "不使用高温分区",
                ],
                1,
            );
            let lr = self.select_index(
                ui,
                tool,
                "sprt.lr",
                "低温分区",
                &LOW_RANGES.iter().map(|r| r.label).collect::<Vec<_>>(),
                0,
            );
            let low_range = &LOW_RANGES[lr];
            if hr == 6 && lr == 2 {
                return Err("高温和低温分区不能同时不使用".into());
            }
            let full = self.select_index(
                ui,
                tool,
                "sprt.deviation",
                "偏差函数",
                &["ITS-90 子区间偏差函数", "三项高温 / 对数低温偏差"],
                0,
            ) == 0;
            let source = self.select_index(
                ui,
                tool,
                "sprt.source",
                "系数设置",
                &["输入证书系数", "固定点电阻拟合"],
                0,
            );
            let mut m = Sprt {
                low_quadratic: full && lr == 1,
                ..Sprt::default()
            };
            let mut fit_points = None;
            self.input(
                ui,
                tool,
                "sprt.rtp",
                "水三相点电阻 Rtp (Ω)",
                &m.rtp.to_string(),
                "0.01 °C 时的电阻",
            );
            m.rtp = self.number(tool, "sprt.rtp", &m.rtp.to_string())?;
            if source == 0 {
                for (key, label, item) in if hr == 6 {
                    vec![]
                } else {
                    vec![
                        ("a", "a", &mut m.a),
                        ("b", "b", &mut m.b),
                        ("c", "c", &mut m.c),
                    ]
                } {
                    let k = format!("sprt.coefficient.{key}");
                    required.push(k.clone());
                    let d = item.to_string();
                    self.input(ui, tool, &k, label, &d, "证书系数");
                    *item = self.number(tool, &k, &d)?;
                }
                if let Some([a_label, b_label]) = low_range.coefficients {
                    for (label, item) in [(a_label, &mut m.a4), (b_label, &mut m.b4)] {
                        let k = format!("sprt.coefficient.{label}");
                        required.push(k.clone());
                        let d = item.to_string();
                        self.input(ui, tool, &k, label, &d, "所选低温分区的证书系数");
                        *item = self.number(tool, &k, &d)?;
                    }
                }
                if full && hr == 0 {
                    required.extend([
                        "sprt.coefficient.d".to_owned(),
                        "sprt.coefficient.w660".to_owned(),
                    ]);
                    let w = m.resistance(660.323)? / m.rtp;
                    self.input(ui, tool, "sprt.coefficient.d", "d6", "0", "银点偏差系数");
                    self.input(
                        ui,
                        tool,
                        "sprt.coefficient.w660",
                        "W660",
                        &w.to_string(),
                        "铝点电阻 / 水三相点电阻",
                    );
                    m.d = self.number(tool, "sprt.coefficient.d", "0")?;
                    m.w660 = self.number(tool, "sprt.coefficient.w660", &w.to_string())?;
                    if m.w660 <= 1.0 {
                        return Err("W660 须大于 1".into());
                    }
                }
            } else {
                let highs = ranges::high_fixed_points(hr);
                let mut hp = Vec::new();
                let mut lp = Vec::new();
                for (list, out) in [(highs, &mut hp), (low_range.fixed_points, &mut lp)] {
                    for &t in list {
                        let key = format!("sprt.fixed.{t}");
                        required.push(key.clone());
                        let d = m.resistance(t)?.to_string();
                        self.input(
                            ui,
                            tool,
                            &key,
                            &format!("{} °C · 实测电阻 (Ω)", format_number(t)),
                            &d,
                            "固定点证书电阻",
                        );
                        out.push((t, self.number(tool, &key, &d)?));
                    }
                }
                m.fit(
                    &hp,
                    &lp,
                    if hr <= 1 {
                        3
                    } else if hr <= 3 {
                        2
                    } else {
                        1
                    },
                )?;
                if full && hr == 0 {
                    required.push("sprt.fixed.961.78".to_owned());
                    m.w660 = hp
                        .iter()
                        .find(|(t, _)| *t == 660.323)
                        .ok_or("缺少铝点数据")?
                        .1
                        / m.rtp;
                    let default = m.resistance(961.78)?.to_string();
                    self.input(
                        ui,
                        tool,
                        "sprt.fixed.961.78",
                        "961.78 °C · 银点实测电阻 (Ω)",
                        &default,
                        "银点证书电阻",
                    );
                    let resistance = self.number(tool, "sprt.fixed.961.78", &default)?;
                    m.fit_silver(961.78, resistance)?;
                    hp.push((961.78, resistance));
                }
                let mut coefficients = vec![
                    ("a", format_process_number(m.a)),
                    ("b", format_process_number(m.b)),
                    ("c", format_process_number(m.c)),
                ];
                if let Some([a_label, b_label]) = low_range.coefficients {
                    coefficients.extend([
                        (a_label, format_process_number(m.a4)),
                        (b_label, format_process_number(m.b4)),
                    ]);
                }
                if full && hr == 0 {
                    coefficients
                        .extend([("d6", format_process_number(m.d)), ("W660", format_process_number(m.w660))]);
                }
                render_metrics(ui, "拟合系数", &coefficients);
                ui.collapsing("固定点拟合残差", |ui| {
                    for &(t, r) in hp.iter().chain(&lp) {
                        if let Ok(actual) = m.temperature(r) {
                            ui.label(format!(
                                "{} °C：{} °C（证书温度 − 反算温度）",
                                format_number(t),
                                format_number(t - actual)
                            ));
                        }
                    }
                });
                fit_points = Some((hp, lp));
            }
            m.validate()?;
            archive_required = Some(required);
            let u = TemperatureUnit(self.select_index(
                ui,
                tool,
                "sprt.unit",
                "温度单位",
                &["°C", "K", "°F"],
                0,
            ));
            let direction = self.select_index(
                ui,
                tool,
                "sprt.direction",
                "计算方式",
                &["温度 → 电阻", "电阻 → 温度"],
                0,
            );
            let key = if direction == 0 {
                format!("sprt.range.{hr}.{lr}.t.{}", u.0)
            } else {
                format!("sprt.range.{hr}.{lr}.r")
            };
            let d = if direction == 0 {
                u.display(if hr == 6 { -20.0 } else { 20.0 }).to_string()
            } else {
                m.resistance(if hr == 6 { -20.0 } else { 20.0 })?
                    .to_string()
            };
            self.input(
                ui,
                tool,
                &key,
                if direction == 0 {
                    u.symbol()
                } else {
                    "电阻 (Ω)"
                },
                &d,
                "输入换算值",
            );
            let v = self.number(tool, &key, &d)?;
            let (t, r) = if direction == 0 {
                let t = u.celsius(v);
                (t, m.resistance_branch(t, model::high_for_temperature(t, hr != 6, lr != 2))?)
            } else {
                (
                    m.temperature_branch(v, model::high_for_resistance(v, m.rtp, hr != 6, lr != 2))?,
                    v,
                )
            };
            let min = low_range.minimum;
            let max = [961.78, 660.323, 419.527, 231.928, 156.599, 29.765, 0.0][hr];
            if t < min - 1e-8 || t > max + 1e-8 {
                return Err(format!(
                    "所选分段范围为 {}～{} °C",
                    format_number(min),
                    format_number(max)
                ));
            }
            // 端点使用模型定义域内的单侧差分。
            let lo = (t - 1.0).max(min);
            let hi = (t + 1.0).min(max);
            let curve = |t| m.resistance_branch(t, model::high_for_temperature(t, hr != 6, lr != 2));
            let slope = (curve(hi)? - curve(lo)?) / (hi - lo);
            let w = r / m.rtp;
            let high = if direction == 0 {
                model::high_for_temperature(t, hr != 6, lr != 2)
            } else {
                model::high_for_resistance(r, m.rtp, hr != 6, lr != 2)
            };
            let wr = m.reference_ratio(w, high);
            let mut steps = vec![format!("{}；选用温区 {}～{} °C；Rtp = {} Ω。", if source == 0 { "使用证书系数" } else { "使用固定点拟合系数" }, format_process_number(min), format_process_number(max), format_process_number(m.rtp)), format!("温度 = {} °C = {} {}；电阻 R = {} Ω。", if direction == 0 { format_process_number(t) } else { format_process_computed(t) }, if direction == 0 { format_process_number(u.display(t)) } else { format_process_computed(u.display(t)) }, u.symbol(), if direction == 0 { format_process_computed(r) } else { format_process_number(r) }), temperature_conversion_note(u.symbol()).to_owned(), format!("实际电阻比 W = R / Rtp = {} / {} = {}。", if direction == 0 { format_process_computed(r) } else { format_process_number(r) }, format_process_number(m.rtp), format_process_computed(w))];
            if let Some((high_points, low_points)) = &fit_points {
                steps.push("固定点拟合：每点计算 W=R/Rtp、参考电阻比 Wr(t)，令 ΔW=W−Wr。高温使用 (W−1)、(W−1)²、(W−1)³ 中所选项；低温使用 (W−1) 和选定的二次项或对数项，按各自固定点最小二乘求解。".to_owned());
                let mut shown = [false; 2];
                for (is_high, points) in [(true, high_points), (false, low_points)] {
                    for &(point_t, point_r) in points {
                        steps.push(String::new());
                        let (coefficients, x, exponential) = curves::reference_form(point_t);
                        let group = usize::from(exponential);
                        if !shown[group] {
                            steps.push(if exponential { "固定点低温参考函数：x=[ln((t+273.15)/273.16)+1.5]/1.5，Wr=exp(Σ A[i]·xⁱ)。" } else { "固定点高温参考函数：x=(t−481)/481，Wr=Σ C[i]·xⁱ。" }.to_owned());
                            steps.extend(coefficients.iter().enumerate().map(|(i,c)| format!("参考 {}[{i}] = {}", if exponential { "A" } else { "C" }, format_process_number(*c))));
                            shown[group] = true;
                        }
                        let point_w = point_r / m.rtp;
                        let point_wr = curves::reference(point_t);
                        let z = point_w - 1.0;
                        steps.push(format!("{}固定点 t = {} °C，R = {} Ω；参考函数 x = {}，Wr = {}；W = R/Rtp = {}，ΔW = {}。", if is_high { "高温" } else { "低温" }, format_process_number(point_t), format_process_number(point_r), format_process_computed(x), format_process_computed(point_wr), format_process_computed(point_w), format_process_computed(point_w - point_wr)));
                        if is_high && hr == 0 && point_t == 961.78 {
                            steps.push(format!("银点单独求 d：d = [ΔW−a(W−1)−b(W−1)²−c(W−1)³]/(W−W660)²；W660 = {}，d = {}。", format_process_number(m.w660), format_process_number(m.d)));
                        } else if is_high {
                            let terms = if hr <= 1 { 3 } else if hr <= 3 { 2 } else { 1 };
                            steps.push(format!("该高温点方程：ΔW ≈ {}·a{}{}。", format_process_computed(z), if terms >= 2 { format!(" + {}·b", format_process_computed(z * z)) } else { String::new() }, if terms >= 3 { format!(" + {}·c", format_process_computed(z.powi(3))) } else { String::new() }));
                        } else {
                            let second = if m.low_quadratic { z.powi(2) } else { z * point_w.ln() };
                            steps.push(format!("该低温点方程：ΔW ≈ {}·a低 + {}·b低。", format_process_computed(z), format_process_computed(second)));
                        }
                    }
                }
                steps.push(format!("拟合结果：a = {}，b = {}，c = {}，a低 = {}，b低 = {}。", format_process_number(m.a), format_process_number(m.b), format_process_number(m.c), format_process_number(m.a4), format_process_number(m.b4)));
            }
            if high {
                steps.push(format!("高温偏差：a = {}，b = {}，c = {}，d = {}，W660 = {}；Wr = W − a(W−1) − b(W−1)² − c(W−1)³ − d·max(W−W660, 0)²。", format_process_number(m.a), format_process_number(m.b), format_process_number(m.c), format_process_number(m.d), format_process_number(m.w660)));
            } else {
                let labels = low_range.coefficients.unwrap_or(["a4", "b4"]);
                steps.push(format!("低温偏差：{} = {}，{} = {}；Wr = W − {}(W−1) − {}·{}。", labels[0], format_process_number(m.a4), labels[1], format_process_number(m.b4), labels[0], labels[1], if m.low_quadratic { "(W−1)²" } else { "(W−1)ln(W)" }));
            }
            steps.push(format!("偏差修正后的参考电阻比 Wr = {}；{}。", format_process_computed(wr), if direction == 0 { "由目标温度在选定分支反算电阻比，再乘以 Rtp" } else { "将 Wr 代入选定参考函数得到温度" }));
            let (coefficients, x, low_reference) = m.reference_function(w, wr, high);
            if low_reference {
                steps.push(format!("低温参考函数：x = (Wr^(1/6) − 0.65) / 0.35 = {}；t = 273.16 × Σ B[i]·xⁱ − 273.15 °C。", format_process_computed(x)));
            } else {
                steps.push(format!("高温参考函数：x = (Wr − 2.64) / 1.64 = {}；t = Σ D[i]·xⁱ °C。", format_process_computed(x)));
            }
            steps.extend(coefficients.iter().enumerate().map(|(i, c)| format!("{}[{i}] = {}", if low_reference { "B" } else { "D" }, format_process_number(*c))));
            let reference_value = crate::calculators::calibration::polynomial(coefficients, x);
            let reference_temperature = if low_reference { 273.16 * reference_value - 273.15 } else { reference_value };
            steps.push(format!("按秦九韶法计算多项式 = {}，参考函数得到温度 {} °C{}。", format_process_computed(reference_value), format_process_computed(reference_temperature), if direction == 0 { "（数值反算校验）" } else { "" }));
            steps.push(format!("电阻变化率采用 {}～{} °C 的差分，结果 {} Ω/{}。", format_process_number(lo), format_process_number(hi), format_process_computed(slope / u.span()), u.symbol()));
            process = Some(steps);
            table = Some((m, u, min, max, hr, lr));
            Ok(vec![
                (
                    "温度",
                    format!("{} {}", format_number(u.display(t)), u.symbol()),
                ),
                ("电阻", format!("{} Ω", format_number(r))),
                ("电阻比 W = R / Rtp", format_number(r / m.rtp)),
                (
                    "电阻变化率 dR/dt",
                    format!("{} Ω/{}", format_number(slope / u.span()), u.symbol()),
                ),
                (
                    "电阻比变化率 dW/dt",
                    format_number(slope / m.rtp / u.span()),
                ),
            ])
        })
        .inner;
        self.finish_calibration_save(tool, archive_save, archive_required);
        show_result_or_error(ui, "换算结果", result);
        if let Some((m, u, min, max, hr, lr)) = table {
            self.render_division_table(
                ui,
                tool,
                &format!("sprt.{hr}.{lr}"),
                (min, max),
                u,
                "Ω",
                |t| m.resistance_branch(t, model::high_for_temperature(t, hr != 6, lr != 2)),
            );
        }
        if let Some(lines) = &process {
            calculation_process(ui, "sprt_process", lines);
        }
        self.calculation_note(ui, tool);
    }
}
