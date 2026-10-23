//! 无界面的量规入口：字段联动和计算共用桌面模型，网页不维护第二份公式。
use super::{
    conventions, imperial, lookup::*, pipe, report::Report, smooth, specs, thread, unified,
};
use crate::catalog::{GaugeKind, ToolKind};
use serde_json::{Value, json};

struct Form {
    state: Value,
    fields: Vec<Value>,
}
impl Form {
    fn text(&mut self, key: &str, label: &str, default: &str, suggestions: Vec<Value>) -> String {
        if !self.state[key].is_string() {
            self.state[key] = json!(default);
        }
        let value = self.state[key].as_str().unwrap_or(default).to_owned();
        self.fields.push(json!({"key":key,"label":label,"type":"text","default":default,"suggestions":suggestions}));
        value
    }
    fn choice(&mut self, key: &str, label: &str, choices: Vec<String>, default: &str) -> String {
        let saved = self.state[key].as_str().unwrap_or(default);
        let value = if choices.iter().any(|s| s == saved) {
            saved.to_owned()
        } else if choices.iter().any(|s| s == default) {
            default.to_owned()
        } else {
            choices.first().cloned().unwrap_or_default()
        };
        if !choices.is_empty() {
            self.state[key] = json!(value);
        }
        self.fields.push(
            json!({"key":key,"label":label,"type":"select","default":default,
            "options":choices.iter().map(|s| json!({"value":s,"label":s})).collect::<Vec<_>>()}),
        );
        value
    }
    fn select(&mut self, key: &str, label: &str, choices: &[&str], default: &str) -> String {
        self.choice(
            key,
            label,
            choices.iter().map(|s| (*s).into()).collect(),
            default,
        )
    }
    fn number(&self, key: &str) -> Result<f64, String> {
        crate::calculators::format::parse_number(self.state[key].as_str().unwrap_or(""))
    }
    fn side(&mut self) -> bool {
        self.select(
            "side",
            "量规类型",
            &["塞规（内螺纹）", "环规（外螺纹）"],
            "塞规（内螺纹）",
        )
        .starts_with("环规")
    }
    fn calculate(&mut self, kind: GaugeKind, changed: &str) -> Result<Report, String> {
        match kind {
            GaugeKind::Metric | GaugeKind::Trapezoidal => {
                let trap = kind == GaugeKind::Trapezoidal;
                let ring = self.side();
                self.text(
                    "diameter",
                    "公称直径 D (mm)",
                    if trap { "24" } else { "20" },
                    suggestions(specs::diameters(trap)),
                );
                let d = self.number("diameter");
                let pitches = if trap {
                    specs::trap_pitches()
                } else {
                    specs::metric_pitches(d.clone().unwrap_or(20.))
                };
                self.text(
                    "pitch",
                    "螺距 P (mm)",
                    if trap { "5" } else { "2.5" },
                    suggestions(pitches.iter().map(ToString::to_string).collect()),
                );
                let p = self.number("pitch");
                let options = if let (Ok(d), Ok(p)) = (&d, &p) {
                    thread::grades(trap, *d, *p)
                } else {
                    Vec::new()
                };
                let grade = self.choice(
                    if ring { "grade.ring" } else { "grade.plug" },
                    if ring {
                        "外螺纹公差带"
                    } else {
                        "内螺纹公差带"
                    },
                    options
                        .into_iter()
                        .filter(|s| {
                            s.chars()
                                .last()
                                .is_some_and(|c| c.is_ascii_lowercase() == ring)
                        })
                        .collect(),
                    match (trap, ring) {
                        (false, false) => "6H",
                        (false, true) => "6g",
                        (true, false) => "8H",
                        (true, true) => "8e",
                    },
                );
                let lines = if trap {
                    self.select("lines", "线数 / 头数", &["1", "2", "3", "4", "5"], "1")
                        .parse()
                        .unwrap_or(1)
                } else {
                    1
                };
                thread::calculate(trap, d?, p?, &grade, lines)
            }
            GaugeKind::Unified => {
                let ring = self.side();
                let options = imperial::un_sizes()
                    .into_iter()
                    .map(|s| {
                        let d = imperial::un_diameter(&s).unwrap_or(f64::NAN);
                        json!({"value":s,"label":format!("{s}（{} inch / {} mm）",
                            crate::calculators::format::format_number(d),
                            crate::calculators::format::format_number(d*25.4))})
                    })
                    .collect();
                let size = self.text("size", "公称直径 (inch，可填 1/2、1 1/4)", "1/4", options);
                let variants = imperial::un_variants(&size);
                if changed == "size" {
                    let old = self.state["_lastSeries"].as_str().unwrap_or("");
                    if let Some((_, tpi)) = variants
                        .iter()
                        .find(|(s, _)| s == old)
                        .or_else(|| variants.first())
                    {
                        self.state["tpi"] = json!(tpi.to_string());
                    }
                }
                if changed == "series" {
                    if let Some((_, tpi)) = variants
                        .iter()
                        .find(|(s, _)| Some(s.as_str()) == self.state["series"].as_str())
                    {
                        self.state["tpi"] = json!(tpi.to_string());
                    }
                }
                let mut tpis: Vec<_> = variants.iter().map(|(_, p)| *p).collect();
                tpis.sort_by(|a, b| b.total_cmp(a));
                tpis.dedup_by(|a, b| same(*a, *b));
                self.text(
                    "tpi",
                    "每英寸牙数 TPI",
                    "20",
                    suggestions(tpis.iter().map(ToString::to_string).collect()),
                );
                let tpi = self.number("tpi");
                let detected = tpi
                    .as_ref()
                    .ok()
                    .and_then(|v| variants.iter().find(|(_, p)| same(*p, *v)))
                    .map(|(s, _)| s.as_str())
                    .unwrap_or("非标 / 自定义");
                let mut series: Vec<_> = variants.iter().map(|(s, _)| s.clone()).collect();
                if detected == "非标 / 自定义" {
                    series.insert(0, detected.into());
                }
                self.state["series"] = json!(detected);
                self.state["_lastSeries"] = json!(detected);
                self.choice("series", "螺纹系列", series, detected);
                let class = self.select(
                    if ring { "class.ring" } else { "class.plug" },
                    "工件公差等级",
                    if ring {
                        &["1A", "2A", "3A"]
                    } else {
                        &["1B", "2B", "3B"]
                    },
                    if ring { "2A" } else { "2B" },
                );
                unified::calculate(&unified::Input {
                    size: &size,
                    tpi: tpi?,
                    class: &class,
                })
            }
            GaugeKind::Pipe => {
                let npt = self
                    .select(
                        "system",
                        "管螺纹类型",
                        &["R · 55° 密封锥螺纹", "NPT · 60° 密封锥螺纹"],
                        "R · 55° 密封锥螺纹",
                    )
                    .starts_with("NPT");
                let code = self.choice(
                    if npt { "spec.npt" } else { "spec.r" },
                    "尺寸代号",
                    pipe::sizes(npt),
                    "1/2",
                );
                pipe::calculate(npt, &code)
            }
            GaugeKind::Rebar => {
                let angle = self.select("angle", "牙型角 (°)", &["75", "60", "70"], "75");
                let choices: Vec<_> = rows(&database()["REBAR"][&angle])
                    .iter()
                    .map(|r| {
                        format!(
                            "{} · {}",
                            r[0].as_str().unwrap_or(""),
                            r[1].as_str().unwrap_or("")
                        )
                    })
                    .collect();
                let spec = self.choice(
                    &format!("spec.{angle}"),
                    "钢筋规格 / 螺纹代号",
                    choices.clone(),
                    choices.first().map(String::as_str).unwrap_or(""),
                );
                let lines = self
                    .select("lines", "线数 / 头数", &["1", "2", "3", "4"], "1")
                    .parse()
                    .unwrap_or(1);
                thread::rebar(
                    angle.parse().unwrap_or(75),
                    choices.iter().position(|s| s == &spec).unwrap_or(0),
                    lines,
                )
            }
            GaugeKind::Smooth => {
                let mode = self.select(
                    "mode",
                    "量规类型",
                    &["孔用塞规", "轴用环规", "轴用卡规"],
                    "孔用塞规",
                );
                let mode = if mode == "孔用塞规" {
                    0
                } else if mode == "轴用卡规" {
                    2
                } else {
                    1
                };
                self.text("diameter", "基本尺寸 D (mm)", "20", vec![]);
                let letter = self.choice(
                    if mode == 0 {
                        "letter.hole"
                    } else {
                        "letter.shaft"
                    },
                    "公差带字母",
                    smooth::letters(mode == 0),
                    if mode == 0 { "H" } else { "h" },
                );
                let grade = self.choice(
                    "it",
                    "公差等级",
                    (6..=16).map(|n| format!("IT{n}")).collect(),
                    "IT7",
                );
                smooth::calculate(
                    self.number("diameter")?,
                    mode,
                    &letter,
                    grade.trim_start_matches("IT").parse().unwrap_or(7),
                )
            }
        }
    }
}
fn suggestions(items: Vec<String>) -> Vec<Value> {
    items.iter().map(|s| json!({"value":s,"label":s})).collect()
}
/// 报告只传显示值；所有计算、修约和英寸附注由共享数值模块完成。
pub(crate) fn page(kind: GaugeKind, state: Value, changed: &str) -> Value {
    let mut form = Form {
        state: if state.is_object() { state } else { json!({}) },
        fields: vec![],
    };
    let calculated = form.calculate(kind, changed);
    let explanation = super::notes(kind);
    let convention = conventions::text(ToolKind::Gauge(kind));
    let (report, error) = match calculated {
        Ok(r) => (
            json!({"title":r.title,"metrics":r.metrics.iter().map(|(k,c)|json!({"label":k,"value":c.display})).collect::<Vec<_>>(),
            "tables":r.tables.iter().map(|t|json!({"title":t.title,"headings":t.headings,"rows":t.rows.iter().map(|r|r.iter().map(|c|&c.display).collect::<Vec<_>>()).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "notes":r.notes,"text":r.text(explanation,&convention)}),
            String::new(),
        ),
        Err(e) => (Value::Null, e),
    };
    json!({"state":form.state,"fields":form.fields,"report":report,"error":error,"convention":convention})
}
